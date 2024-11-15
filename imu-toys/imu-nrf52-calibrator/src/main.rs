#![no_std]
#![no_main]

use core::ffi::c_char;

use cortex_m::prelude::_embedded_hal_blocking_delay_DelayMs;
use defmt::{debug, error, info, println, trace, unwrap};
use embassy_executor::Spawner;
use embassy_nrf::{
    self as _,
    gpio::{Input, Output},
    interrupt::{self, InterruptExt},
    peripherals::{P0_14, P1_15, TWISPI0},
    twim::Frequency,
    uarte,
};
use embassy_nrf::{bind_interrupts, peripherals};
use embassy_nrf::{
    gpio,
    twim::{self, Twim},
};
use lsm6dsox::PrimaryRegister;

use {defmt_rtt as _, panic_probe as _};

mod registers;

const SAMPLE_RATE_HZ: u32 = 208;

bind_interrupts!(struct Irqs {
    UARTE0_UART0 => uarte::InterruptHandler<peripherals::UARTE0>;
    SPIM0_SPIS0_TWIM0_TWIS0_SPI0_TWI0 => twim::InterruptHandler<peripherals::TWISPI0>;

});

fn get_sensor_data(
    sensor: &mut lsm6dsox::Lsm6dsox<Twim<'_, TWISPI0>, embassy_time::Delay>,
) -> registers::SensorVals {
    let raw_reg = unsafe { sensor.register_access() };
    let status =
        registers::StatusReg::from_bytes([raw_reg.read_reg(PrimaryRegister::STATUS_REG).unwrap()]);

    if !(status.gda() && status.xlda()) {
        return registers::SensorVals {
            accel: None,
            gyro: None,
        };
    }

    let mut data = [0; 12];
    raw_reg
        .read_regs(PrimaryRegister::OUTX_L_G, &mut data)
        .unwrap();
    registers::SensorVals::new(data, status)
}

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let mut config = embassy_nrf::config::Config::default();
    config.gpiote_interrupt_priority = interrupt::Priority::P2;
    config.time_interrupt_priority = interrupt::Priority::P2;
    let p = embassy_nrf::init(config);

    // setup uart
    let mut config = uarte::Config::default();
    config.parity = uarte::Parity::EXCLUDED;
    config.baudrate = uarte::Baudrate::BAUD115200;

    let mut uart = uarte::Uarte::new(p.UARTE0, Irqs, p.P0_08, p.P0_06, config);

    info!("uarte initialized!");

    // color sensor interrupt pin
    let mut imu_int_pin = gpio::Input::new(p.P1_15, gpio::Pull::Down);

    info!("Initializing TWI...");
    let mut config = twim::Config::default();
    // config.frequency = Frequency::K400;
    interrupt::SPIM0_SPIS0_TWIM0_TWIS0_SPI0_TWI0.set_priority(interrupt::Priority::P3);
    let mut i2c = Twim::new(p.TWISPI0, Irqs, p.P0_26, p.P0_27, config);

    let mut lsm = lsm6dsox::Lsm6dsox::new(i2c, lsm6dsox::SlaveAddress::High, embassy_time::Delay);

    // Reset sensor
    let raw_reg = unsafe { lsm.register_access() };
    let val = raw_reg.read_reg(PrimaryRegister::CTRL3_C).unwrap();
    raw_reg.write_reg(PrimaryRegister::CTRL3_C, 0b1).unwrap();
    embassy_time::Timer::after_secs(1).await;

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
    let start_time = embassy_time::Instant::now();
    let mut num_readings = 0;
    let mut tot_samples = 0;
    let mut skip_first_x = 100;
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
            tot_samples += 1;
            trace!(
                "gyro(x: {},\ty: {},\tz: {}),\taccel(x: {},\ty: {},\tz: {})",
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

        imu_int_pin.wait_for_rising_edge().await;
    }

    avg_gyro_x /= num_readings as f32;
    avg_gyro_y /= num_readings as f32;
    avg_gyro_z /= num_readings as f32;
    avg_accel_x /= num_readings as f32;
    avg_accel_y /= num_readings as f32;
    avg_accel_z /= num_readings as f32;

    println!(
        "\ngyro_x: {}\ngyro_y: {}\ngyro_z: {}\n\naccel_x: {}\naccel_y: {}\naccel_z: {}\nmax_z: {}",
        avg_gyro_x, avg_gyro_y, avg_gyro_z, avg_accel_x, avg_accel_y, avg_accel_z, max_z
    );
    debug!("{}", tot_samples);

    let gyr_offset = imu_fusion::FusionVector::new(avg_gyro_x, avg_gyro_y, avg_gyro_z);
    let acc_offset = imu_fusion::FusionVector::new(avg_accel_x, avg_accel_y, avg_accel_z - 1.0);

    let mut ahrs_settings = imu_fusion::FusionAhrsSettings::new();
    ahrs_settings.convention = imu_fusion::FusionConvention::NWU;
    ahrs_settings.gain = 0.5f32;
    ahrs_settings.gyr_range = 500.0f32; // replace this with actual gyroscope range in degrees/s
    ahrs_settings.acc_rejection = 10.0f32;
    ahrs_settings.recovery_trigger_period = 5 * SAMPLE_RATE_HZ as i32;

    let mut fusion = imu_fusion::Fusion::new(SAMPLE_RATE_HZ, ahrs_settings);
    // fusion.acc_offset = acc_offset;
    // fusion.gyr_offset = gyr_offset;

    // let start_time = embassy_time::Instant::now();
    let mut prev_time = embassy_time::Instant::now();
    let mut last_print = prev_time;
    let mut v_x = 0.0;
    let mut v_y = 0.0;
    let mut v_z = 0.0;
    let mut x = 0.0;
    let mut y = 0.0;
    let mut z = 0.0;

    loop {
        let sensor_data = get_sensor_data(&mut lsm);
        if let (Some(accel), Some(gyro)) = (sensor_data.accel, sensor_data.gyro) {
            let cur_time = embassy_time::Instant::now(); //.as_micros() as f32 / 1_000_000.0;
                                                         // let timestamp = start_time.elapsed().as_micros() as f32 / 1_000_000.0;
            let delta_t = (cur_time - prev_time).as_micros() as f32 / 1_000_000.0;

            let gyr = imu_fusion::FusionVector::new(gyro.x, gyro.y, gyro.z);
            let mut gyr = fusion.inertial_calibration(
                gyr,
                imu_fusion::FusionMatrix::identity(),
                imu_fusion::FusionVector::ones(),
                gyr_offset,
            );
            // if gyr.x <= 1.0 && gyr.x >= -1.0 {
            //     gyr.x = 0.0;
            // }
            // if gyr.y <= 1.0 && gyr.y >= -1.0 {
            //     gyr.y = 0.0;
            // }
            // if gyr.z <= 1.0 && gyr.z >= -1.0 {
            //     gyr.z = 0.0;
            // }

            let acc = imu_fusion::FusionVector::new(accel.x, accel.y, accel.z);
            let mut acc = fusion.inertial_calibration(
                acc,
                imu_fusion::FusionMatrix::identity(),
                imu_fusion::FusionVector::ones(),
                acc_offset,
            );

            // if acc.x < 0.01 && acc.x > -0.01 {
            //     acc.x = 0.0;
            // }
            // if acc.y < 0.01 && acc.y > -0.01 {
            //     acc.y = 0.0;
            // }
            // if acc.z < 0.01 && acc.y > -0.01 {
            //     acc.z = 0.0;
            // }

            // debug!(
            //     "{},{},{}\t{},{},{}",
            //     acc.x, acc.y, acc.z, gyr.x, gyr.y, gyr.z
            // );

            // fusion.update_no_mag(gyr, acc, timestamp);
            fusion.update_no_mag_by_duration_seconds(gyr, acc, delta_t);

            // // let euler = fusion.euler();
            let mut earth_acc = fusion.earth_acc();
            // // let time_diff = line.timestamp - prev_time;
            // x += (earth_acc.x * 9.81) * delta_t;
            // y += (earth_acc.y * 9.81) * delta_t;
            // z += (earth_acc.z * 9.81) * delta_t;

            if earth_acc.x < 0.01 && earth_acc.x > -0.01 {
                earth_acc.x = 0.0;
            }
            if earth_acc.y < 0.01 && earth_acc.y > -0.01 {
                earth_acc.y = 0.0;
            }
            if earth_acc.z < 0.01 && earth_acc.z > -0.01 {
                earth_acc.z = 0.0;
            }

            v_x += (earth_acc.x * 9.81) * delta_t;
            v_y += (earth_acc.y * 9.81) * delta_t;
            v_z += (earth_acc.z * 9.81) * delta_t;

            x += v_x * delta_t;
            y += v_y * delta_t;
            z += v_z * delta_t;

            prev_time = cur_time;

            if last_print.elapsed().as_millis() >= 1000 {
                println!(
                    "\nraw (calibrated): accel(x: {}\ty: {}\tz: {})\tgyro(x: {}\ty: {}\tz: {})",
                    acc.x, acc.y, acc.z, gyr.x, gyr.y, gyr.z
                );
                println!("\nv_x: {}\tv_y: {}\tv_z: {}", v_x, v_y, v_z);

                // println!("\nx: {}\ty: {}\tz: {}", x, y, z);

                // let earth_acc = fusion.earth_acc();
                // println!(
                //     "\nx: {}\ty: {}\tz: {}",
                //     earth_acc.x, earth_acc.y, earth_acc.z
                // );

                let euler = fusion.euler();
                println!(
                    "\nroll: {}\tpitch: {}\tyaw: {}\t\tx: {}\ty: {}\tz: {}",
                    euler.angle.roll, euler.angle.pitch, euler.angle.yaw, x, y, z
                );
                last_print = embassy_time::Instant::now();
            }
        }

        imu_int_pin.wait_for_rising_edge().await;
    }
}
