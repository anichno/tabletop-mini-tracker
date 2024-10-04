use std::{
    fs::File,
    io::{BufWriter, Write},
    thread,
    time::Duration,
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

    let log_file = File::create("sensor_data.log").unwrap();
    let mut log_file = BufWriter::new(log_file);

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

    let raw_reg = unsafe { lsm.register_access() };
    let mut ctrl4 = raw_reg.read_reg(PrimaryRegister::CTRL4_C).unwrap();
    ctrl4 |= 0b1000;
    raw_reg.write_reg(PrimaryRegister::CTRL4_C, ctrl4).unwrap();

    let mut int1 = raw_reg.read_reg(PrimaryRegister::INT1_CTRL).unwrap();
    int1 |= 0b11;
    raw_reg.write_reg(PrimaryRegister::INT1_CTRL, int1).unwrap();

    lsm.enable_interrupts(true).unwrap();
    debug!("ready");

    let mut num_readings = 0;
    let start_time = std::time::Instant::now();
    loop {
        if let None = imu_int_pin
            .poll_interrupt(false, Some(Duration::from_secs(1)))
            .unwrap()
        {
            panic!("interrupt locked up. num_readings: {num_readings}");
        }
        num_readings += 1;
        if num_readings % 1000 == 0 {
            debug!("{num_readings}");
        }

        let sensor_data = get_sensor_data(&mut lsm);
        if let (Some(accel), Some(gyro)) = (sensor_data.accel, sensor_data.gyro) {
            let timestamp = start_time.elapsed().as_secs_f32();
            let line = format!(
                "{},{},{},{},{},{},{}",
                timestamp, accel.x, accel.y, accel.z, gyro.x, gyro.y, gyro.z
            );
            trace!("{line}");
            log_file.write_all(line.as_bytes()).unwrap();
            log_file.write_all(&[b'\n']).unwrap();
        }
    }
}
