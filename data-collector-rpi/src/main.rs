use std::{
    io::{Read, Write},
    net::TcpListener,
};

use anyhow::Result;
use opt4048::{IntCfg, IntDir};
use rppal::{gpio::Gpio, i2c::I2c};

fn main() -> Result<()> {
    let gpio = Gpio::new()?;
    let mut color_int_pin = gpio.get(17)?.into_input_pullup();
    color_int_pin.set_interrupt(rppal::gpio::Trigger::RisingEdge)?;

    let mut i2c = I2c::new()?;

    // Reset I2C devices
    i2c.set_slave_address(0)?;
    i2c.write(&[0x6])?;

    let mut color_sensor = opt4048::Opt4048::new(i2c)?;

    let config = color_sensor
        .get_config()?
        .with_operating_mode(opt4048::OperatingMode::PowerDown)
        .with_conversion_time(opt4048::ConversionTime::Time100ms)
        .with_range(opt4048::Range::Range2_2klux)
        .with_int_pol(true)
        .with_latch(false);
    color_sensor.set_config(config)?;

    let int_config = color_sensor
        .get_interrupt_config()?
        .with_int_dir(IntDir::Output)
        .with_int_cfg(IntCfg::IntDrAllChannels);
    color_sensor.set_interrupt_config(int_config)?;

    let listener = TcpListener::bind("0.0.0.0:9000")?;
    let (mut screen, _) = listener.accept()?;
    let mut buf = [0; 1];
    screen.read(&mut buf)?;

    if buf[0] == b'R' {
        screen.write_all(&[b'S'])?;
    } else {
        anyhow::bail!("Screen connection sent wront start byte");
    }

    loop {
        if screen.read(&mut buf)? == 0 {
            break;
        }

        if buf[0] == b'B' || buf[0] == b'W' {
            // wait for screen to actually be drawn
            std::thread::sleep(std::time::Duration::from_millis(100));

            color_sensor.trigger_oneshot()?;
            color_int_pin.poll_interrupt(true, None)?;

            let brightness = color_sensor.get_channel_3()?;
            println!(
                "{},{}",
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)?
                    .as_millis(),
                brightness
            );

            let mut output = [b'B', 0, 0, 0, 0];
            for (i, b) in brightness.to_le_bytes().into_iter().enumerate() {
                output[i + 1] = b;
            }
            screen.write_all(&output)?;
        } else {
            break;
        }
    }

    Ok(())
}
