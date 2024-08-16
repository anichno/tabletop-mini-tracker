#![no_std]
#![no_main]

mod fmt;

#[cfg(not(feature = "defmt"))]
use panic_halt as _;
#[cfg(feature = "defmt")]
use {defmt_rtt as _, panic_probe as _};

use core::mem;

use embassy_executor::Spawner;
use embassy_nrf as _;
use embassy_nrf::{bind_interrupts, peripherals};
use embassy_nrf::{
    gpio,
    twim::{self, Twim},
};
use fmt::{info, unwrap};
use nrf_softdevice::ble::advertisement_builder::{
    Flag, LegacyAdvertisementBuilder, LegacyAdvertisementPayload, ServiceList, ServiceUuid16,
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

#[nrf_softdevice::gatt_service(uuid = "180f")]
struct BatteryService {
    #[characteristic(uuid = "2a19", read, notify)]
    battery_level: u8,
}

#[nrf_softdevice::gatt_service(uuid = "9e7312e0-2354-11eb-9f10-fbc30a62cf38")]
struct FooService {
    #[characteristic(
        uuid = "9e7312e0-2354-11eb-9f10-fbc30a63cf38",
        read,
        write,
        notify,
        indicate
    )]
    foo: u16,
}

#[nrf_softdevice::gatt_server]
struct Server {
    bas: BatteryService,
    foo: FooService,
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    info!("Hello World!");

    let p = embassy_nrf::init(Default::default());

    // color sensor interrupt pin
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

    loop {
        color_sensor.trigger_oneshot().unwrap();
        color_int_pin.wait_for_rising_edge().await;
        let val = color_sensor.get_channel_3().unwrap();
        info!("{}", val);
    }

    // let config = nrf_softdevice::Config {
    //     clock: Some(raw::nrf_clock_lf_cfg_t {
    //         source: raw::NRF_CLOCK_LF_SRC_XTAL as u8,
    //         rc_ctiv: 0,
    //         rc_temp_ctiv: 0,
    //         accuracy: raw::NRF_CLOCK_LF_ACCURACY_500_PPM as u8,
    //     }),
    //     conn_gap: Some(raw::ble_gap_conn_cfg_t {
    //         conn_count: 1,
    //         event_length: 24,
    //     }),
    //     conn_gatt: Some(raw::ble_gatt_conn_cfg_t { att_mtu: 256 }),
    //     gatts_attr_tab_size: Some(raw::ble_gatts_cfg_attr_tab_size_t {
    //         attr_tab_size: raw::BLE_GATTS_ATTR_TAB_SIZE_DEFAULT,
    //     }),
    //     gap_role_count: Some(raw::ble_gap_cfg_role_count_t {
    //         adv_set_count: 1,
    //         periph_role_count: 1,
    //     }),
    //     gap_device_name: Some(raw::ble_gap_cfg_device_name_t {
    //         p_value: b"HelloRust" as *const u8 as _,
    //         current_len: 9,
    //         max_len: 9,
    //         write_perm: unsafe { mem::zeroed() },
    //         _bitfield_1: raw::ble_gap_cfg_device_name_t::new_bitfield_1(
    //             raw::BLE_GATTS_VLOC_STACK as u8,
    //         ),
    //     }),
    //     ..Default::default()
    // };

    // let sd = Softdevice::enable(&config);
    // let server = unwrap!(Server::new(sd));
    // unwrap!(spawner.spawn(softdevice_task(sd)));

    // static ADV_DATA: LegacyAdvertisementPayload = LegacyAdvertisementBuilder::new()
    //     .flags(&[Flag::GeneralDiscovery, Flag::LE_Only])
    //     .services_16(ServiceList::Complete, &[ServiceUuid16::BATTERY])
    //     .full_name("HelloRust")
    //     .build();

    // static SCAN_DATA: LegacyAdvertisementPayload = LegacyAdvertisementBuilder::new()
    //     .services_128(
    //         ServiceList::Complete,
    //         &[0x9e7312e0_2354_11eb_9f10_fbc30a62cf38_u128.to_le_bytes()],
    //     )
    //     .build();

    // loop {
    //     let config = peripheral::Config::default();
    //     let adv = peripheral::ConnectableAdvertisement::ScannableUndirected {
    //         adv_data: &ADV_DATA,
    //         scan_data: &SCAN_DATA,
    //     };
    //     let conn = unwrap!(peripheral::advertise_connectable(sd, adv, &config).await);

    //     info!("advertising done!");

    //     // Run the GATT server on the connection. This returns when the connection gets disconnected.
    //     //
    //     // Event enums (ServerEvent's) are generated by nrf_softdevice::gatt_server
    //     // proc macro when applied to the Server struct above
    //     let e = gatt_server::run(&conn, &server, |e| match e {
    //         ServerEvent::Bas(e) => match e {
    //             BatteryServiceEvent::BatteryLevelCccdWrite { notifications } => {
    //                 info!("battery notifications: {}", notifications)
    //             }
    //         },
    //         ServerEvent::Foo(e) => match e {
    //             FooServiceEvent::FooWrite(val) => {
    //                 info!("wrote foo: {}", val);
    //                 if let Err(e) = server.foo.foo_notify(&conn, &(val + 1)) {
    //                     info!("send notification error: {:?}", e);
    //                 }
    //             }
    //             FooServiceEvent::FooCccdWrite {
    //                 indications,
    //                 notifications,
    //             } => {
    //                 info!(
    //                     "foo indications: {}, notifications: {}",
    //                     indications, notifications
    //                 )
    //             }
    //         },
    //     })
    //     .await;

    //     info!("gatt_server run exited with error: {:?}", e);
    // }
}
