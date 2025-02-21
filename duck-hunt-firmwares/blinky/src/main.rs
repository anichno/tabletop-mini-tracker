#![no_std]
#![no_main]

use defmt::info;
use embassy_executor::Spawner;
use embassy_nrf::gpio::{Level, Output, OutputDrive};
use embassy_time::Timer;
use {defmt_rtt as _, panic_probe as _};

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = embassy_nrf::init(Default::default());
    info!("running!");

    let red = Output::new(p.P0_20, Level::High, OutputDrive::Standard0Disconnect1);
    let green = Output::new(p.P0_18, Level::High, OutputDrive::Standard0Disconnect1);
    let blue = Output::new(p.P0_13, Level::High, OutputDrive::Standard0Disconnect1);

    let mut leds = [red, green, blue];
    loop {
        for (i, led) in leds.iter_mut().enumerate() {
            match i {
                0 => info!("red"),
                1 => info!("green"),
                2 => info!("blue"),
                _ => panic!("invalid color"),
            }
            led.set_low();
            Timer::after_millis(1000).await;
            led.set_high();
        }
    }
}
