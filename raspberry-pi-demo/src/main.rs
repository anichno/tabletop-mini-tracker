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
        .with_operating_mode(opt4048::OperatingMode::Continuous)
        .with_conversion_time(opt4048::ConversionTime::Time600us)
        .with_range(opt4048::Range::RangeAuto)
        .with_int_pol(true)
        .with_latch(false);
    color_sensor.set_config(config)?;

    let int_config = color_sensor
        .get_interrupt_config()?
        .with_int_dir(IntDir::Output)
        .with_int_cfg(IntCfg::IntDrAllChannels);
    color_sensor.set_interrupt_config(int_config)?;

    loop {
        color_int_pin.poll_interrupt(false, None)?;
        // let (x, y, z) = (
        //     color_sensor.get_channel_0()?,
        //     color_sensor.get_channel_1()?,
        //     color_sensor.get_channel_2()?,
        // );
        // println!(
        //     "{},{},{},{}",
        //     std::time::SystemTime::now()
        //         .duration_since(std::time::UNIX_EPOCH)?
        //         .as_millis(),
        //     x,
        //     y,
        //     z
        // );
        // let test = color_sensor.get_channel_test();
        // println!("{:?} : {:?}", test, test.to_float());
        // println!("{:?}", color_sensor.get_color_cie());
        // std::thread::sleep(std::time::Duration::from_secs(1));

        let brightness = color_sensor.get_channel_3()?;
        println!(
            "{},{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_millis(),
            brightness
        );
    }

    Ok(())
}
