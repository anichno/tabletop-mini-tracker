use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream},
};

use anyhow::Result;
use opt4048::{IntCfg, IntDir, Opt4048};
use rppal::{
    gpio::{Gpio, InputPin},
    i2c::I2c,
};

fn recv_coords(
    color_sensor: &mut Opt4048<I2c>,
    color_int_pin: &mut InputPin,
    sync_channel: &mut TcpStream,
    mut num_x: u8,
    mut num_y: u8,
    mut num_crc: u8,
) -> Option<(u8, u8)> {
    let mut buf = [0; 2];
    let mut calibrate_low = None;
    let mut calibrate_high = None;
    let mut active_transmission = true;
    let mut zero_bit_max = None;
    let mut one_bit_min = None;

    let mut recv_x = 0;
    let mut recv_y = 0;
    let mut recv_crc = 0;

    while active_transmission {
        let bytes_read = sync_channel.read(&mut buf).unwrap();
        if bytes_read == 0 {
            return None;
        }

        color_sensor.trigger_oneshot().unwrap();
        color_int_pin.poll_interrupt(true, None).unwrap();

        let sensor_val = color_sensor.get_channel_3().unwrap();
        println!("[{}, {}] : {}", buf[0] as char, buf[1], sensor_val);

        match buf[0] {
            b'L' => calibrate_low = Some(sensor_val),
            b'H' => calibrate_high = Some(sensor_val),
            b'D' => {
                if let (Some(zero_bit_max), Some(one_bit_min)) = (zero_bit_max, one_bit_min) {
                    let val = if sensor_val < zero_bit_max {
                        0
                    } else if sensor_val > one_bit_min {
                        1
                    } else {
                        panic!("recieved value in deadband");
                    };

                    println!("Received: {}", val);

                    if num_x > 0 {
                        recv_x = (recv_x << 1) | val;
                        num_x -= 1;
                    } else if num_y > 0 {
                        recv_y = (recv_y << 1) | val;
                        num_y -= 1;
                    } else if num_crc > 0 {
                        recv_crc = (recv_crc << 1) | val;
                        num_crc -= 1;
                    } else {
                        panic!("too many bits sent");
                    }
                }
            }
            b'F' => active_transmission = false,
            _ => panic!("Invalid command: {} received", buf[0] as char),
        }

        if let (Some(c_low), Some(c_high)) = (calibrate_low, calibrate_high) {
            let diff = c_high - c_low;
            let range = diff / 3;
            zero_bit_max = Some(c_low + range);
            one_bit_min = Some(c_high - range);

            calibrate_low = None;
            calibrate_high = None;
        }

        sync_channel.write_all(&[b'A']).unwrap();
    }

    let crc_valid =
        crc::Crc::<u8>::new(&crc::CRC_4_G_704).checksum(&[recv_x as u8, recv_y as u8]) == recv_crc;

    if num_x == 0 && num_y == 0 && num_crc == 0 && crc_valid {
        Some((recv_x, recv_y))
    } else {
        None
    }
}

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
        .with_conversion_time(opt4048::ConversionTime::Time25ms)
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

    println!("Waiting for connection from screen");
    let (mut screen, _) = listener.accept()?;
    let mut buf = [0; 4];

    println!("Waiting for sync");
    screen.read(&mut buf)?;

    if buf[0] == b'I' {
        screen.write_all(&[b'A'])?;
    } else {
        anyhow::bail!("Screen connection sent wront start byte");
    }

    loop {
        println!("Waiting for start of transmission");
        if screen.read(&mut buf)? == 0 {
            break;
        }

        if buf[0] == b'S' {
            screen.write_all(&[b'A']).unwrap();
            println!(
                "Received coordinates: {:?}",
                recv_coords(
                    &mut color_sensor,
                    &mut color_int_pin,
                    &mut screen,
                    buf[1],
                    buf[2],
                    buf[3]
                )
            );
        } else {
            break;
        }
    }

    Ok(())
}
