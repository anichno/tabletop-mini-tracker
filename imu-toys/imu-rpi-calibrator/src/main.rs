use std::{
    fs::File,
    io::{BufWriter, Write},
    thread,
    time::{Duration, Instant},
};

use anyhow::Result;
use log::{debug, info, trace};
use lsm6dsox::PrimaryRegister;
use registers::{SensorVals, StatusReg};
use rppal::{gpio::Gpio, i2c::I2c};

mod registers;

fn get_sensor_data(sensor: &mut lsm6dsox::Lsm6dsox<I2c, rppal::hal::Delay>) -> SensorVals {
    let raw_reg: &mut lsm6dsox::RegisterAccess<I2c> = unsafe { sensor.register_access() };
    let status = StatusReg::from_bytes([raw_reg.read_reg(PrimaryRegister::STATUS_REG).unwrap()]);
    let mut data = [0; 12];
    raw_reg
        .read_regs(PrimaryRegister::OUTX_L_G, &mut data)
        .unwrap();
    SensorVals::new(data, status)
}

fn main() -> Result<()> {
    env_logger::init();

    let gpio = Gpio::new()?;
    let mut imu_int_pin = gpio.get(4)?.into_input_pullup();
    imu_int_pin.set_interrupt(rppal::gpio::Trigger::RisingEdge, None)?;

    let i2c = I2c::new().unwrap();

    let mut lsm =
        lsm6dsox::Lsm6dsox::new(i2c, lsm6dsox::SlaveAddress::High, rppal::hal::Delay::new());

    // Reset sensor
    let raw_reg = unsafe { lsm.register_access() };
    raw_reg.write_reg(PrimaryRegister::CTRL3_C, 0b1).unwrap();
    thread::sleep(Duration::from_secs(1));

    debug!("{:?}", lsm.check_id());

    lsm.setup().unwrap();
    lsm.set_accel_sample_rate(lsm6dsox::DataRate::Freq208Hz)
        .unwrap();
    lsm.set_accel_scale(lsm6dsox::AccelerometerScale::Accel8g)
        .unwrap();
    lsm.set_gyro_sample_rate(lsm6dsox::DataRate::Freq208Hz)
        .unwrap();
    lsm.set_gyro_scale(lsm6dsox::GyroscopeScale::Dps500)
        .unwrap();

    // enable data available pin
    let raw_reg = unsafe { lsm.register_access() };
    let mut ctrl4 = raw_reg.read_reg(PrimaryRegister::CTRL4_C).unwrap();
    ctrl4 |= 0b1000;
    raw_reg.write_reg(PrimaryRegister::CTRL4_C, ctrl4).unwrap();

    // enable interrupt when accel or gyro available
    let mut int1 = raw_reg.read_reg(PrimaryRegister::INT1_CTRL).unwrap();
    int1 |= 0b11;
    raw_reg.write_reg(PrimaryRegister::INT1_CTRL, int1).unwrap();

    // set pulsed data ready
    let mut bdr_reg1 = raw_reg.read_reg(PrimaryRegister::COUNTER_BDR_REG1).unwrap();
    bdr_reg1 |= 0b10000000;
    raw_reg
        .write_reg(PrimaryRegister::COUNTER_BDR_REG1, bdr_reg1)
        .unwrap();

    // // enable high pass filter
    // let mut ctrl8xl =
    //     registers::Ctrl8Xlreg::from_bytes([raw_reg.read_reg(PrimaryRegister::CTRL8_XL).unwrap()]);
    // ctrl8xl.set_hpcf_xl(0b101);
    // ctrl8xl.set_hp_slope_xl_en(true);
    // raw_reg
    //     .write_reg(PrimaryRegister::CTRL8_XL, ctrl8xl.into_bytes()[0])
    //     .unwrap();

    lsm.enable_interrupts(true).unwrap();
    debug!("ready");

    info!("Reading Z");
    let start_time = Instant::now();
    let mut num_readings = 0;
    let mut skip_first_x = 2000;
    let mut avg_gyro_x = 0.0;
    let mut avg_gyro_y = 0.0;
    let mut avg_gyro_z = 0.0;
    let mut avg_accel_x = 0.0;
    let mut avg_accel_y = 0.0;
    let mut avg_accel_z = 0.0;
    let mut max_z = 0.0f32;
    while start_time.elapsed().as_millis() < 5_000 {
        let sensor_data = get_sensor_data(&mut lsm);
        if let (Some(accel), Some(gyro)) = (sensor_data.accel, sensor_data.gyro) {
            trace!(
                "gyro(x: {}, y: {}, z: {}), accel(x: {}, y: {}, z: {})",
                gyro.x,
                gyro.y,
                gyro.z,
                accel.x,
                accel.y,
                accel.z
            );

            if skip_first_x > 0 {
                skip_first_x -= 1;
                continue;
            }
            num_readings += 1;

            avg_gyro_x += gyro.x;
            avg_gyro_y += gyro.y;
            avg_gyro_z += gyro.z;
            avg_accel_x += accel.x;
            avg_accel_y += accel.y;
            avg_accel_z += accel.z;
            max_z = max_z.max(accel.z);
        }

        if let None = imu_int_pin
            .poll_interrupt(false, Some(Duration::from_secs(1)))
            .unwrap()
        {
            panic!("interrupt locked up. num_readings: {num_readings}");
        }
    }

    avg_gyro_x /= num_readings as f32;
    avg_gyro_y /= num_readings as f32;
    avg_gyro_z /= num_readings as f32;
    avg_accel_x /= num_readings as f32;
    avg_accel_y /= num_readings as f32;
    avg_accel_z /= num_readings as f32;

    dbg!(
        avg_gyro_x,
        avg_gyro_y,
        avg_gyro_z,
        avg_accel_x,
        avg_accel_y,
        avg_accel_z,
        max_z
    );

    Ok(())
}
