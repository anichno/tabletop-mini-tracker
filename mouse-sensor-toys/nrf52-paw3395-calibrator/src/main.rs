#![no_std]
#![no_main]

use cortex_m::prelude::_embedded_hal_blocking_serial_Write;
use defmt::{debug, error, info};
use embassy_executor::Spawner;
use embassy_nrf::gpio::{self, Level, Output, OutputDrive};
use embassy_nrf::spim::Spim;
use embassy_nrf::{bind_interrupts, peripherals, spim, uarte};
use embassy_time::Timer;
use {defmt_rtt as _, panic_probe as _};

bind_interrupts!(struct Irqs {
    SPIM3 => spim::InterruptHandler<peripherals::SPI3>;
    UARTE0 => uarte::InterruptHandler<peripherals::UARTE0>;

});

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = embassy_nrf::init(Default::default());
    info!("running!");

    let pause_btn = gpio::Input::new(p.P0_11, gpio::Pull::Up);
    let capture_btn = gpio::Input::new(p.P0_12, gpio::Pull::Up);

    let mut config = uarte::Config::default();
    config.parity = uarte::Parity::EXCLUDED;
    config.baudrate = uarte::Baudrate::BAUD115200;

    let mut uart = uarte::Uarte::new(p.UARTE0, Irqs, p.P0_08, p.P0_06, config);

    info!("uarte initialized!");

    let mut config = spim::Config::default();
    config.frequency = spim::Frequency::M2;
    config.mode = spim::MODE_3;

    let spim: Spim<'_, peripherals::SPI3> =
        spim::Spim::new(p.SPI3, Irqs, p.P0_29, p.P0_28, p.P0_30, config);
    let ncs = Output::new(p.P0_31, Level::High, OutputDrive::Standard);

    let sensor = paw3395::Paw3395::new(spim, ncs, embassy_time::Delay).await;
    let mut sensor = match sensor {
        Ok(sensor) => {
            debug!("Sensor init success");
            sensor
        }
        Err(e) => panic!("Sensor init failed: {:?}", e),
    };

    let mut camera_disabled = false;
    let mut btn_pressed = false;
    let mut capture_btn_pressed = false;
    loop {
        if btn_pressed && pause_btn.is_high() {
            btn_pressed = false;
        }

        if capture_btn_pressed && capture_btn.is_high() {
            capture_btn_pressed = false;
        }

        if camera_disabled {
            if !capture_btn_pressed && capture_btn.is_low() {
                debug!("Sending capture request");
                capture_btn_pressed = true;
                uart.write(&[b'C', b'A', b'P', b'T', b'U', b'R', b'E'])
                    .await
                    .unwrap();
                uart.bflush().unwrap();
            }

            Timer::after_millis(100).await;
            if !btn_pressed && pause_btn.is_low() {
                debug!("Resuming");
                camera_disabled = false;
                btn_pressed = true;
            }
            continue;
        } else {
            if !btn_pressed && pause_btn.is_low() {
                debug!("Pausing");
                camera_disabled = true;
                btn_pressed = true;
                continue;
            }
        }
        let start = embassy_time::Instant::now();
        let frame = sensor.frame_capture().await;
        let frame_time = embassy_time::Instant::now() - start;
        debug!("Took {} ms to capture frame", frame_time.as_millis());
        match frame {
            Ok(data) => {
                let start = embassy_time::Instant::now();
                uart.write(&[b'F', b'R', b'A', b'M', b'E']).await.unwrap();
                for row in data {
                    uart.write(&row).await.unwrap();
                }
                let transmit_time = embassy_time::Instant::now() - start;
                debug!("Took {} ms to transmit frame", transmit_time.as_millis());
                uart.bflush().unwrap();
            }
            Err(e) => error!("frame capture error: {:?}", e),
        }
    }
}
