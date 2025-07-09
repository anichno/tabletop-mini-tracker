#![no_std]
#![no_main]

use defmt::{debug, error, info, println};
use embassy_executor::Spawner;
use embassy_nrf::gpio::{Level, Output, OutputDrive};
use embassy_nrf::spim::Spim;
use embassy_nrf::{bind_interrupts, peripherals, spim, uarte};
use {defmt_rtt as _, panic_probe as _};

bind_interrupts!(struct Irqs {
    SPIM3 => spim::InterruptHandler<peripherals::SPI3>;
    UARTE0 => uarte::InterruptHandler<peripherals::UARTE0>;

});

async fn get_still_frame(
    sensor: &mut paw3395::Paw3395<Spim<'_, peripherals::SPI3>, Output<'_>, embassy_time::Delay>,
) -> [[u8; paw3395::IMAGE_WIDTH]; paw3395::IMAGE_HEIGHT] {
    loop {
        if let paw3395::BurstResult::Motion(delta) = sensor.burst_motion_read().await.unwrap() {
            if delta.delta_x == 0 && delta.delta_y == 0 {
                // Make sure actually stopped and not just switching directions (vibrating)
                embassy_time::Timer::after_millis(50).await;

                // Chip not lifted?
                if let paw3395::BurstResult::Motion(delta) =
                    sensor.burst_motion_read().await.unwrap()
                {
                    // Sensor still hasn' moved?
                    if delta.delta_x == 0 && delta.delta_y == 0 {
                        let frame = sensor.frame_capture().await.unwrap();
                        return frame;
                    }
                }
            }
            embassy_time::Timer::after_millis(10).await;
        }
    }
}

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = embassy_nrf::init(Default::default());
    info!("running!");

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

    loop {
        let start = embassy_time::Instant::now();

        let frame = get_still_frame(&mut sensor).await;
        let frame_time = embassy_time::Instant::now() - start;

        let start = embassy_time::Instant::now();
        let solved = mouse_vision::solve(&frame);
        let solve_time = embassy_time::Instant::now() - start;

        uart.write(&[b'F', b'R', b'A', b'M', b'E']).await.unwrap();
        for row in frame {
            uart.write(&row).await.unwrap();
        }
        // uart.write(&[b'S', b'O', b'L', b'V', b'E']).await.unwrap();
        if let Some(solved) = solved {
            // for row in solved {
            //     let mut buf = [0; 8];
            //     for (idx, c) in row.into_iter().enumerate() {
            //         buf[idx] = c as u8;
            //     }
            //     uart.write(&buf).await.unwrap();
            // }
        } else {
            uart.write(&[0; 8 * 8]).await.unwrap();
        }

        // let start = embassy_time::Instant::now();
        // let frame = sensor.frame_capture().await;
        // let frame_time = embassy_time::Instant::now() - start;
        // debug!("Took {} ms to capture frame", frame_time.as_millis());
        // match frame {
        //     Ok(data) => {
        //         let start = embassy_time::Instant::now();
        //         uart.write(&[b'F', b'R', b'A', b'M', b'E']).await.unwrap();
        //         for row in data {
        //             uart.write(&row).await.unwrap();
        //         }
        //         let transmit_time = embassy_time::Instant::now() - start;
        //         debug!("Took {} ms to transmit frame", transmit_time.as_millis());
        //     }
        //     Err(e) => error!("frame capture error: {:?}", e),
        // }
    }
}
