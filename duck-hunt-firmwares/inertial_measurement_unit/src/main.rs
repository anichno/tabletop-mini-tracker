#![no_std]
#![no_main]

use defmt::{debug, info};
use embassy_executor::Spawner;
use embassy_nrf::{
    bind_interrupts,
    gpio::{self},
    interrupt::InterruptExt,
    twim,
};
use lsm6dsox::PrimaryRegister;
use {defmt_rtt as _, panic_probe as _};

mod registers;

bind_interrupts!(struct Irqs {
    TWISPI0 => embassy_nrf::twim::InterruptHandler<embassy_nrf::peripherals::TWISPI0>;
});

fn get_sensor_data(
    sensor: &mut lsm6dsox::Lsm6dsox<
        twim::Twim<'_, embassy_nrf::peripherals::TWISPI0>,
        embassy_time::Delay,
    >,
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
    let p = embassy_nrf::init(Default::default());
    info!("running!");

    let mut imu_int_pin = gpio::Input::new(p.P0_12, gpio::Pull::Down);

    info!("Initializing TWI...");
    let config = twim::Config::default();
    embassy_nrf::interrupt::TWISPI0.set_priority(embassy_nrf::interrupt::Priority::P3);
    let i2c = twim::Twim::new(p.TWISPI0, Irqs, p.P0_26, p.P0_27, config);

    let mut lsm = lsm6dsox::Lsm6dsox::new(i2c, lsm6dsox::SlaveAddress::Low, embassy_time::Delay);

    // Reset sensor
    let raw_reg = unsafe { lsm.register_access() };
    let _val = raw_reg.read_reg(PrimaryRegister::CTRL3_C).unwrap();
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

    lsm.enable_interrupts(true).unwrap();
    debug!("ready");

    loop {
        let sensor_data = get_sensor_data(&mut lsm);
        if let (Some(accel), Some(gyro)) = (sensor_data.accel, sensor_data.gyro) {
            debug!(
                "{},{},{}\t{},{},{}",
                accel.x, accel.y, accel.z, gyro.x, gyro.y, gyro.z
            );
        }

        imu_int_pin.wait_for_high().await;
    }
}
