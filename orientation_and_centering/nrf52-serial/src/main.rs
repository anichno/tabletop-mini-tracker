#![no_std]
#![no_main]

use core::{fmt::Write, time::Duration, u32};
use defmt::{error, info, unwrap};
use embassy_executor::Spawner;
use embassy_nrf::{
    bind_interrupts, gpio, peripherals,
    twim::{self, Twim},
    uarte,
};
use embassy_time::{Instant, Timer};
use opt4048::{IntCfg, IntDir};
use serial_common::Commands;
use {defmt_rtt as _, panic_probe as _};

bind_interrupts!(struct Irqs {
    UARTE0_UART0 => uarte::InterruptHandler<peripherals::UARTE0>;
    SPIM0_SPIS0_TWIM0_TWIS0_SPI0_TWI0 => twim::InterruptHandler<peripherals::TWISPI0>;

});

async fn ping_pong(uart: &mut uarte::Uarte<'_, peripherals::UARTE0>) {
    unwrap!(uart.write(&[b'P', b'O', b'N', b'G']).await);
}

async fn light_sensor(
    uart: &mut uarte::Uarte<'_, peripherals::UARTE0>,
    color_sensor: &mut opt4048::Opt4048<Twim<'_, peripherals::TWISPI0>>,
    color_int_pin: &mut gpio::Input<'_, peripherals::P1_15>,
) {
    color_sensor.trigger_oneshot().unwrap();
    color_int_pin.wait_for_rising_edge().await;
    let val = color_sensor.get_channel_3().unwrap();
    unwrap!(uart.write(&val.to_le_bytes()).await);
}

async fn light_sensor_config(
    uart: &mut uarte::Uarte<'_, peripherals::UARTE0>,
    color_sensor: &mut opt4048::Opt4048<Twim<'_, peripherals::TWISPI0>>,
) {
    let mut param = [0; 1];
    uart.read(&mut param).await.unwrap();
    let param = param[0];

    let conversion_time = match param {
        1 => opt4048::ConversionTime::Time600us,
        2 => opt4048::ConversionTime::Time1ms,
        3 => opt4048::ConversionTime::Time1ms8,
        4 => opt4048::ConversionTime::Time3ms4,
        5 => opt4048::ConversionTime::Time6ms5,
        6 => opt4048::ConversionTime::Time12ms7,
        7 => opt4048::ConversionTime::Time25ms,
        8 => opt4048::ConversionTime::Time50ms,
        9 => opt4048::ConversionTime::Time100ms,
        10 => opt4048::ConversionTime::Time200ms,
        11 => opt4048::ConversionTime::Time400ms,
        12 => opt4048::ConversionTime::Time800ms,
        _ => panic!("unknown conversion time: {}", param),
    };

    let config = color_sensor
        .get_config()
        .unwrap()
        .with_conversion_time(conversion_time);
    color_sensor.set_config(config).unwrap();

    unwrap!(uart.write(&[b'A']).await);
}

async fn calibrate_dark(
    data: &mut CalibrationData,
    uart: &mut uarte::Uarte<'_, peripherals::UARTE0>,
    color_sensor: &mut opt4048::Opt4048<Twim<'_, peripherals::TWISPI0>>,
    color_int_pin: &mut gpio::Input<'_, peripherals::P1_15>,
) {
    const NUM_READINGS: u32 = 100;
    let mut max = 0;
    let mut min = u32::MAX;
    let mut avg = 0;

    for _ in 0..NUM_READINGS {
        color_sensor.trigger_oneshot().unwrap();
        color_int_pin.wait_for_rising_edge().await;
        let val = color_sensor.get_channel_3().unwrap();

        max = max.max(val);
        min = min.min(val);
        avg += val;
    }

    avg /= NUM_READINGS;
    info!("calibrate_dark:\tmin: {}, max: {}, avg: {}", min, max, avg);

    data.dark_val = avg;

    unwrap!(uart.write(&[b'A']).await);
}

async fn calibrate_light(
    data: &mut CalibrationData,
    uart: &mut uarte::Uarte<'_, peripherals::UARTE0>,
    color_sensor: &mut opt4048::Opt4048<Twim<'_, peripherals::TWISPI0>>,
    color_int_pin: &mut gpio::Input<'_, peripherals::P1_15>,
) {
    const NUM_READINGS: u32 = 100;
    let mut max = 0;
    let mut min = u32::MAX;
    let mut avg = 0;

    for _ in 0..NUM_READINGS {
        color_sensor.trigger_oneshot().unwrap();
        color_int_pin.wait_for_rising_edge().await;
        let val = color_sensor.get_channel_3().unwrap();

        max = max.max(val);
        min = min.min(val);
        avg += val;
    }

    avg /= NUM_READINGS;
    info!("calibrate_light:\tmin: {}, max: {}, avg: {}", min, max, avg);

    data.light_val = avg;

    unwrap!(uart.write(&[b'A']).await);
}

async fn calibrate_middle(
    data: &mut CalibrationData,
    uart: &mut uarte::Uarte<'_, peripherals::UARTE0>,
    color_sensor: &mut opt4048::Opt4048<Twim<'_, peripherals::TWISPI0>>,
    color_int_pin: &mut gpio::Input<'_, peripherals::P1_15>,
) {
    const NUM_READINGS: u32 = 100;
    let mut max = 0;
    let mut min = u32::MAX;
    let mut avg = 0;

    for _ in 0..NUM_READINGS {
        color_sensor.trigger_oneshot().unwrap();
        color_int_pin.wait_for_rising_edge().await;
        let val = color_sensor.get_channel_3().unwrap();

        max = max.max(val);
        min = min.min(val);
        avg += val;
    }

    avg /= NUM_READINGS;
    info!(
        "calibrate_middle:\tmin: {}, max: {}, avg: {}",
        min, max, avg
    );

    data.middle_val = avg;

    unwrap!(uart.write(&[b'A']).await);
}

async fn get_delay(
    data: &CalibrationData,
    uart: &mut uarte::Uarte<'_, peripherals::UARTE0>,
    color_sensor: &mut opt4048::Opt4048<Twim<'_, peripherals::TWISPI0>>,
    color_int_pin: &mut gpio::Input<'_, peripherals::P1_15>,
) {
    let mut input_buf = [0; 2];
    unwrap!(uart.read(&mut input_buf).await);
    let dark_to_light: bool = input_buf[0] > 0;
    let delay = input_buf[1] as u64;

    let target_val = if dark_to_light {
        data.light_val - (data.light_val as f32 * 0.1) as u32
    } else {
        data.dark_val + (data.dark_val as f32 * 0.1) as u32
    };

    Timer::after_millis(delay).await;
    let start = Instant::now();

    for wait in 0..2000_u32 {
        color_sensor.trigger_oneshot().unwrap();
        color_int_pin.wait_for_rising_edge().await;
        let val = color_sensor.get_channel_3().unwrap();

        if (dark_to_light && val > target_val) || (!dark_to_light && val < target_val) {
            let diff = (Instant::now() - start).as_millis() as u32;
            unwrap!(uart.write(&diff.to_le_bytes()).await);
            return;
        }
    }

    unwrap!(uart.write(&u32::MAX.to_le_bytes()).await);
}

#[derive(Default)]
struct CalibrationData {
    delay: Duration,
    dark_val: u32,
    light_val: u32,
    middle_val: u32,
}

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = embassy_nrf::init(Default::default());
    let mut config = uarte::Config::default();
    config.parity = uarte::Parity::EXCLUDED;
    config.baudrate = uarte::Baudrate::BAUD115200;

    let mut uart = uarte::Uarte::new(p.UARTE0, Irqs, p.P0_08, p.P0_06, config);

    info!("uarte initialized!");

    let mut color_int_pin = gpio::Input::new(p.P1_15, gpio::Pull::Up);

    info!("Initializing TWI...");
    let config = twim::Config::default();
    let mut i2c = Twim::new(p.TWISPI0, Irqs, p.P0_26, p.P0_27, config);

    // Reset I2C devices
    i2c.blocking_write(0x0, &[0x6]).unwrap();

    let mut color_sensor = opt4048::Opt4048::new(i2c).unwrap();

    let config = color_sensor
        .get_config()
        .unwrap()
        .with_operating_mode(opt4048::OperatingMode::PowerDown)
        .with_conversion_time(opt4048::ConversionTime::Time25ms)
        .with_range(opt4048::Range::Range2_2klux)
        .with_int_pol(true)
        .with_latch(false);
    color_sensor.set_config(config).unwrap();

    let int_config = color_sensor
        .get_interrupt_config()
        .unwrap()
        .with_int_dir(IntDir::Output)
        .with_int_cfg(IntCfg::IntDrAllChannels);
    color_sensor.set_interrupt_config(int_config).unwrap();

    let mut calibration_data = CalibrationData::default();

    let mut cmd_buf = [0; 1];

    loop {
        uart.read(&mut cmd_buf).await.unwrap();
        if let Ok(cmd) = Commands::try_from(cmd_buf[0]) {
            match cmd {
                Commands::Ping => ping_pong(&mut uart).await,
                Commands::GetLightValue => {
                    light_sensor(&mut uart, &mut color_sensor, &mut color_int_pin).await
                }
                Commands::SetLightConversionTime => {
                    light_sensor_config(&mut uart, &mut color_sensor).await
                }
                Commands::CalibrateDark => {
                    calibrate_dark(
                        &mut calibration_data,
                        &mut uart,
                        &mut color_sensor,
                        &mut color_int_pin,
                    )
                    .await
                }
                Commands::CalibrateLight => {
                    calibrate_light(
                        &mut calibration_data,
                        &mut uart,
                        &mut color_sensor,
                        &mut color_int_pin,
                    )
                    .await
                }
                Commands::CalibrateMiddle => {
                    calibrate_middle(
                        &mut calibration_data,
                        &mut uart,
                        &mut color_sensor,
                        &mut color_int_pin,
                    )
                    .await
                }
                Commands::GetFrameDelay => {
                    get_delay(
                        &calibration_data,
                        &mut uart,
                        &mut color_sensor,
                        &mut color_int_pin,
                    )
                    .await
                }
            }
        } else {
            error!("Unknown command: {}", cmd_buf[0])
        }
    }
}
