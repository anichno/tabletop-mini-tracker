#![no_std]
#![no_main]

use defmt::{debug, info, println};
use embassy_executor::Spawner;
use embassy_nrf::{
    bind_interrupts,
    gpio::{self},
    interrupt::{self, InterruptExt},
    peripherals::TWISPI0,
    twim::{self, Twim},
};
use embassy_time::Timer;
use {defmt_rtt as _, panic_probe as _};

bind_interrupts!(struct Irqs {
    TWISPI0 => twim::InterruptHandler<embassy_nrf::peripherals::TWISPI0>;
});

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let config = embassy_nrf::config::Config::default();
    let p = embassy_nrf::init(config);

    // color sensor interrupt pin
    let mut color_int1_pin = gpio::Input::new(p.P0_11, gpio::Pull::None);
    let mut color_int2_pin = gpio::Input::new(p.P0_03, gpio::Pull::None);

    info!("Initializing TWI...");
    let config = twim::Config::default();
    interrupt::TWISPI0.set_priority(interrupt::Priority::P3);
    let mut i2c = Twim::new(p.TWISPI0, Irqs, p.P0_26, p.P0_27, config);

    // Reset I2C devices
    i2c.blocking_write(0x0, &[0x6]).unwrap();

    static I2C_BUS: static_cell::StaticCell<
        embassy_sync::blocking_mutex::NoopMutex<core::cell::RefCell<Twim<TWISPI0>>>,
    > = static_cell::StaticCell::new();
    let i2c_bus = embassy_sync::blocking_mutex::NoopMutex::new(core::cell::RefCell::new(i2c));
    let i2c_bus = I2C_BUS.init(i2c_bus);

    let mut color_sensor1 = opt4001::Opt4001::new(
        embassy_embedded_hal::shared_bus::blocking::i2c::I2cDevice::new(i2c_bus),
        opt4001::Address::Addr0x44,
    )
    .unwrap();
    let mut color_sensor2 = opt4001::Opt4001::new(
        embassy_embedded_hal::shared_bus::blocking::i2c::I2cDevice::new(i2c_bus),
        opt4001::Address::Addr0x45,
    )
    .unwrap();

    debug!("Sensors initialized");

    let config = color_sensor1
        .get_config()
        .unwrap()
        .with_operating_mode(opt4001::OperatingMode::PowerDown)
        .with_conversion_time(opt4001::ConversionTime::Time25ms)
        .with_range(opt4001::Range::RangeAuto)
        .with_int_pol(false)
        .with_latch(false);
    color_sensor1.set_config(config).unwrap();

    let int_config = color_sensor1
        .get_interrupt_config()
        .unwrap()
        .with_int_dir(opt4001::IntDir::Output)
        .with_int_cfg(opt4001::IntCfg::IntEveryConv);
    color_sensor1.set_interrupt_config(int_config).unwrap();

    let config = color_sensor2
        .get_config()
        .unwrap()
        .with_operating_mode(opt4001::OperatingMode::PowerDown)
        .with_conversion_time(opt4001::ConversionTime::Time25ms)
        .with_range(opt4001::Range::RangeAuto)
        .with_int_pol(false)
        .with_latch(false);
    color_sensor2.set_config(config).unwrap();

    let int_config = color_sensor2
        .get_interrupt_config()
        .unwrap()
        .with_int_dir(opt4001::IntDir::Output)
        .with_int_cfg(opt4001::IntCfg::IntEveryConv);
    color_sensor2.set_interrupt_config(int_config).unwrap();

    loop {
        color_sensor1.trigger_oneshot().unwrap();
        color_sensor2.trigger_oneshot().unwrap();

        color_int1_pin.wait_for_low().await;
        color_int2_pin.wait_for_low().await;

        let val1 = color_sensor1.get_output().unwrap();
        let val2 = color_sensor2.get_output().unwrap();

        println!("{}\t{}", val1, val2);
        Timer::after_millis(250).await;
    }
}
