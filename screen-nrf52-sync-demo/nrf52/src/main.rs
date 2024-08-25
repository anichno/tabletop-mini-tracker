#![no_std]
#![no_main]

// mod fmt;

use binary_serde::BinarySerde;
use common::btle_constants::command_service;
use defmt::{error, info, unwrap};
use embassy_futures::select::select;
use embassy_sync::{blocking_mutex::raw::NoopRawMutex, signal::Signal};
#[cfg(not(feature = "defmt"))]
use panic_halt as _;
#[cfg(feature = "defmt")]
use {defmt_rtt as _, panic_probe as _};

use core::mem;

use embassy_executor::Spawner;
use embassy_nrf::{
    self as _,
    gpio::{Input, Output},
    interrupt::{self, InterruptExt},
    peripherals::{P0_14, P1_15, TWISPI0},
};
use embassy_nrf::{bind_interrupts, peripherals};
use embassy_nrf::{
    gpio,
    twim::{self, Twim},
};
// use fmt::{error, info, unwrap};
use nrf_softdevice::ble::{
    advertisement_builder::{
        Flag, LegacyAdvertisementBuilder, LegacyAdvertisementPayload, ServiceList, ServiceUuid16,
    },
    Connection,
};
use nrf_softdevice::ble::{gatt_server, peripheral};
use nrf_softdevice::{raw, Softdevice};
use opt4048::{IntCfg, IntDir, Opt4048};

bind_interrupts!(struct Irqs {
    SPIM0_SPIS0_TWIM0_TWIS0_SPI0_TWI0 => twim::InterruptHandler<peripherals::TWISPI0>;
});

#[embassy_executor::task]
async fn softdevice_task(sd: &'static Softdevice) -> ! {
    sd.run().await
}

#[nrf_softdevice::gatt_service(uuid = "0cde1400-51fd-474f-b021-d22d0cd05ec5")]
struct CommandService {
    #[characteristic(uuid = "0cde1401-51fd-474f-b021-d22d0cd05ec5", write)]
    command: u8,
    #[characteristic(uuid = "0cde1402-51fd-474f-b021-d22d0cd05ec5", read, notify)]
    ack: u8,
    #[characteristic(uuid = "0cde1403-51fd-474f-b021-d22d0cd05ec5", write)]
    transmission_description: [u8; 3],
    #[characteristic(uuid = "0cde1404-51fd-474f-b021-d22d0cd05ec5", read, notify)]
    screen_command: u8,
}

#[nrf_softdevice::gatt_service(uuid = "0cde1500-51fd-474f-b021-d22d0cd05ec5")]
struct LocationService {
    #[characteristic(uuid = "0cde1501-51fd-474f-b021-d22d0cd05ec5", read, indicate)]
    location: [u8; common::Location::SERIALIZED_SIZE],
}

#[nrf_softdevice::gatt_server]
struct Server {
    commands: CommandService,
    location: LocationService,
}

#[derive(Debug, Clone, Copy, Default)]
struct TransmissionState {
    calibrate_low: Option<u32>,
    calibrate_high: Option<u32>,
    zero_bit_max: Option<u32>,
    one_bit_min: Option<u32>,
    recv_x: u32,
    recv_y: u32,
    recv_crc: u8,
    num_x_bits: u8,
    num_y_bits: u8,
    num_crc_bits: u8,
}

async fn get_brightness(
    color_sensor: &mut Opt4048<Twim<'static, TWISPI0>>,
    color_int_pin: &mut Input<'static, P1_15>,
) -> u32 {
    color_sensor.trigger_oneshot().unwrap();
    color_int_pin.wait_for_rising_edge().await;
    color_sensor.get_channel_3().unwrap()
}

fn send_ack(server: &Server, conn: &Connection, ack: common::CommandAck) {
    let ack_val = ack as u8;
    server.commands.ack_set(&ack_val).unwrap();
    if let Err(e) = server.commands.ack_notify(conn, &ack_val) {
        info!("Failed to ack: {:?}", e);
    }
}

#[embassy_executor::task]
async fn light_sensor_task(
    mut color_sensor: Opt4048<Twim<'static, TWISPI0>>,
    mut color_int_pin: Input<'static, P1_15>,
    mut transmission_status_led: Output<'static, P0_14>,
    server: &'static Server,
    color_signal: &'static Signal<NoopRawMutex, (common::MiniCommands, Connection)>,
) {
    let mut cur_transmission = TransmissionState::default();
    loop {
        let (cmd, conn) = color_signal.wait().await;
        info!("Received cmd: {:?}", cmd as u8);

        let mut ack = common::CommandAck::Ok;

        match cmd {
            common::MiniCommands::StartTransmission => {
                transmission_status_led.set_low();
                cur_transmission = TransmissionState::default();
                let num_bits = server.commands.transmission_description_get().unwrap();
                cur_transmission.num_x_bits = num_bits[0];
                cur_transmission.num_y_bits = num_bits[1];
                cur_transmission.num_crc_bits = num_bits[2];
            }
            common::MiniCommands::CalibrateLow => {
                cur_transmission.calibrate_low =
                    Some(get_brightness(&mut color_sensor, &mut color_int_pin).await)
            }
            common::MiniCommands::CalibrateHigh => {
                cur_transmission.calibrate_high =
                    Some(get_brightness(&mut color_sensor, &mut color_int_pin).await)
            }
            common::MiniCommands::BitSent => {
                if let (Some(zero_bit_max), Some(one_bit_min)) =
                    (cur_transmission.zero_bit_max, cur_transmission.one_bit_min)
                {
                    let sensor_val = get_brightness(&mut color_sensor, &mut color_int_pin).await;
                    let val = if sensor_val < zero_bit_max {
                        Some(0)
                    } else if sensor_val > one_bit_min {
                        Some(1)
                    } else {
                        error!("recieved value in deadband");
                        ack = common::CommandAck::Error;
                        None
                    };

                    if let Some(val) = val {
                        info!("Received bit: {}", val);

                        if cur_transmission.num_x_bits > 0 {
                            cur_transmission.recv_x = (cur_transmission.recv_x << 1) | val;
                            cur_transmission.num_x_bits -= 1;
                        } else if cur_transmission.num_y_bits > 0 {
                            cur_transmission.recv_y = (cur_transmission.recv_y << 1) | val;
                            cur_transmission.num_y_bits -= 1;
                        } else if cur_transmission.num_crc_bits > 0 {
                            cur_transmission.recv_crc =
                                (cur_transmission.recv_crc << 1) | val as u8;
                            cur_transmission.num_crc_bits -= 1;
                        } else {
                            error!("too many bits sent");
                            ack = common::CommandAck::Error;
                        }
                    }
                } else {
                    error!("Received BitSent command, but no calibration info available");
                }
            }
            common::MiniCommands::EndTransmission => {
                let crc_valid = crc::Crc::<u8>::new(&crc::CRC_4_G_704)
                    .checksum(&[cur_transmission.recv_x as u8, cur_transmission.recv_y as u8])
                    == cur_transmission.recv_crc;

                if cur_transmission.num_x_bits == 0
                    && cur_transmission.num_y_bits == 0
                    && cur_transmission.num_crc_bits == 0
                    && crc_valid
                {
                    let location = common::Location {
                        x: cur_transmission.recv_x,
                        y: cur_transmission.recv_y,
                        z: 0,
                        error: 0,
                    };

                    info!("Located at: ({}, {})", location.x, location.y);
                    let mut location_buf: [u8; common::Location::SERIALIZED_SIZE] =
                        [0; common::Location::SERIALIZED_SIZE];
                    location.binary_serialize(&mut location_buf, binary_serde::Endianness::Little);
                    server.location.location_set(&location_buf).unwrap();
                    server
                        .location
                        .location_indicate(&conn, &location_buf)
                        .unwrap();
                } else {
                    error!("Checksum mismatch");
                    ack = common::CommandAck::Error;
                }

                transmission_status_led.set_high();
            }
        }

        if matches!(
            cmd,
            common::MiniCommands::CalibrateLow | common::MiniCommands::CalibrateHigh
        ) {
            if let (Some(low), Some(high)) = (
                cur_transmission.calibrate_low,
                cur_transmission.calibrate_high,
            ) {
                if let Some(diff) = high.checked_sub(low) {
                    let range = diff / 3;
                    cur_transmission.zero_bit_max = Some(low + range);
                    cur_transmission.one_bit_min = Some(high - range);
                    info!("Calibrated");
                } else {
                    ack = common::CommandAck::Error;
                }
            }
        }

        send_ack(server, &conn, ack);
    }
}

#[embassy_executor::task]
async fn btn_manager_task(
    mut btn: Input<'static, peripherals::P0_11>,
    server: &'static Server,
    conn_signal: &'static Signal<NoopRawMutex, Option<Connection>>,
) {
    let mut current_conn = None;
    loop {
        let new_conn = conn_signal.wait();
        let btn_input = btn.wait_for_falling_edge();

        let event = select(new_conn, btn_input).await;
        match event {
            embassy_futures::select::Either::First(new_conn) => {
                info!("Connection status changed");
                current_conn = new_conn;
            }
            embassy_futures::select::Either::Second(_) => {
                info!("Trigger Button Pressed");
                if let Some(conn) = &current_conn {
                    if let Err(e) = server.commands.screen_command_notify(
                        conn,
                        &(common::ScreenCommands::TriggerTransmission as u8),
                    ) {
                        info!("Failed to trigger transmission: {:?}", e);
                    }
                } else {
                    info!("No current connection");
                }

                // Bad debounce (no reason to loop right away, device will be busy with screen transmission)
                embassy_time::Timer::after_secs(1).await;
            }
        }
    }
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let mut config = embassy_nrf::config::Config::default();
    config.gpiote_interrupt_priority = interrupt::Priority::P2;
    config.time_interrupt_priority = interrupt::Priority::P2;
    let p = embassy_nrf::init(config);

    // color sensor interrupt pin
    let color_int_pin = gpio::Input::new(p.P1_15, gpio::Pull::Up);
    let trigger_transmission_btn = gpio::Input::new(p.P0_11, gpio::Pull::Up);

    let mut connection_status_led =
        gpio::Output::new(p.P0_13, gpio::Level::High, gpio::OutputDrive::Standard);
    let transmission_status_led =
        gpio::Output::new(p.P0_14, gpio::Level::High, gpio::OutputDrive::Standard);

    info!("Initializing TWI...");
    let config = twim::Config::default();
    interrupt::SPIM0_SPIS0_TWIM0_TWIS0_SPI0_TWI0.set_priority(interrupt::Priority::P3);
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

    static COLOR_SIGNAL: static_cell::StaticCell<
        Signal<NoopRawMutex, (common::MiniCommands, Connection)>,
    > = static_cell::StaticCell::new();
    let color_signal = COLOR_SIGNAL.init(Signal::new());

    static TRIGGER_TRANSMISSION_SIGNAL: static_cell::StaticCell<
        Signal<NoopRawMutex, Option<Connection>>,
    > = static_cell::StaticCell::new();
    let trigger_transmission_signal = TRIGGER_TRANSMISSION_SIGNAL.init(Signal::new());

    let config = nrf_softdevice::Config {
        clock: Some(raw::nrf_clock_lf_cfg_t {
            source: raw::NRF_CLOCK_LF_SRC_XTAL as u8,
            rc_ctiv: 0,
            rc_temp_ctiv: 0,
            accuracy: raw::NRF_CLOCK_LF_ACCURACY_500_PPM as u8,
        }),
        conn_gap: Some(raw::ble_gap_conn_cfg_t {
            conn_count: 1,
            event_length: 24,
        }),
        conn_gatt: Some(raw::ble_gatt_conn_cfg_t { att_mtu: 256 }),
        gatts_attr_tab_size: Some(raw::ble_gatts_cfg_attr_tab_size_t {
            attr_tab_size: raw::BLE_GATTS_ATTR_TAB_SIZE_DEFAULT,
        }),
        gap_role_count: Some(raw::ble_gap_cfg_role_count_t {
            adv_set_count: 1,
            periph_role_count: 1,
        }),
        gap_device_name: Some(raw::ble_gap_cfg_device_name_t {
            p_value: b"HelloRust" as *const u8 as _,
            current_len: 9,
            max_len: 9,
            write_perm: unsafe { mem::zeroed() },
            _bitfield_1: raw::ble_gap_cfg_device_name_t::new_bitfield_1(
                raw::BLE_GATTS_VLOC_STACK as u8,
            ),
        }),
        ..Default::default()
    };

    let sd = Softdevice::enable(&config);

    static SERVER: static_cell::StaticCell<Server> = static_cell::StaticCell::new();
    let server = SERVER.init(unwrap!(Server::new(sd)));

    unwrap!(spawner.spawn(softdevice_task(sd)));
    spawner
        .spawn(light_sensor_task(
            color_sensor,
            color_int_pin,
            transmission_status_led,
            server,
            color_signal,
        ))
        .unwrap();

    spawner
        .spawn(btn_manager_task(
            trigger_transmission_btn,
            server,
            trigger_transmission_signal,
        ))
        .unwrap();

    static ADV_DATA: LegacyAdvertisementPayload = LegacyAdvertisementBuilder::new()
        .flags(&[Flag::GeneralDiscovery, Flag::LE_Only])
        .services_16(ServiceList::Complete, &[ServiceUuid16::BATTERY])
        .full_name("HelloRust")
        .build();

    static SCAN_DATA: LegacyAdvertisementPayload = LegacyAdvertisementBuilder::new()
        .services_128(
            ServiceList::Complete,
            &[command_service::SERVICE_UUID.as_u128().to_le_bytes()],
        )
        .build();

    loop {
        let config = peripheral::Config::default();
        let adv = peripheral::ConnectableAdvertisement::ScannableUndirected {
            adv_data: &ADV_DATA,
            scan_data: &SCAN_DATA,
        };
        let conn = unwrap!(peripheral::advertise_connectable(sd, adv, &config).await);

        info!("advertising done!");

        trigger_transmission_signal.signal(Some(conn.clone()));
        connection_status_led.set_low();

        // Run the GATT server on the connection. This returns when the connection gets disconnected.
        //
        // Event enums (ServerEvent's) are generated by nrf_softdevice::gatt_server
        // proc macro when applied to the Server struct above
        let e = gatt_server::run(&conn, server, |e| match e {
            ServerEvent::Commands(e) => match e {
                CommandServiceEvent::CommandWrite(cmd_id) => {
                    if let Ok(cmd) = common::MiniCommands::try_from(cmd_id) {
                        color_signal.signal((cmd, conn.clone()));
                    } else {
                        error!("Received invalid cmd id: {}", cmd_id);
                    }
                }

                CommandServiceEvent::AckCccdWrite { notifications } => {
                    info!("Ack notifications: {}", notifications)
                }
                CommandServiceEvent::TransmissionDescriptionWrite(descr) => {
                    info!(
                        "num_x_bits: {}, num_y_bits: {}, num_crc_bits: {}",
                        descr[0], descr[1], descr[2]
                    )
                }
                CommandServiceEvent::ScreenCommandCccdWrite { notifications } => {
                    info!("Screen Command notifications: {}", notifications)
                }
            },
            ServerEvent::Location(e) => match e {
                LocationServiceEvent::LocationCccdWrite { indications } => {
                    info!("Location inidications: {}", indications)
                }
            },
        })
        .await;

        info!("gatt_server run exited with error: {:?}", e);

        trigger_transmission_signal.signal(None);
        connection_status_led.set_high();
    }
}
