use anyhow::Result;
use binary_serde::BinarySerde;
use btleplug::api::{
    Central, Characteristic, Manager as _, Peripheral as _, ScanFilter, Service, WriteType,
};
use btleplug::platform::{Adapter, Manager, Peripheral};
use common::btle_constants;
use futures::future::{select, Either};
use futures::StreamExt;
use log::{debug, info, warn};
use speedy2d::window::UserEventSender;
use std::time::Duration;
use tokio::sync::mpsc::UnboundedReceiver;
use tokio::{pin, time};

#[derive(Debug)]
pub enum MiniMessage {
    Command(common::ScreenCommands),
    Position(common::Location),
}

struct Mini {
    peripheral: Peripheral,
    command_service: CommandService,
    location_service: LocationService,
}

struct CommandService {
    command: Characteristic,
    _ack: Characteristic,
    ack_channel: UnboundedReceiver<common::CommandAck>,
    transmission_description: Characteristic,
    _screen_command: Characteristic,
    screen_command_channel: Option<UnboundedReceiver<common::ScreenCommands>>,
}

struct LocationService {
    _location: Characteristic,
    location_channel: Option<UnboundedReceiver<common::Location>>,
}

impl CommandService {
    async fn new(
        s: Service,
        p: &Peripheral,
        ack_channel: UnboundedReceiver<common::CommandAck>,
        screen_command_channel: UnboundedReceiver<common::ScreenCommands>,
    ) -> Self {
        use btle_constants::command_service::characteristics::*;

        let command = get_characteristic(&s, COMMAND_UUID);
        let ack = get_characteristic(&s, ACK_UUID);
        let transmission_description = get_characteristic(&s, TRANSMISSION_DESCRIPTION_UUID);
        let screen_command = get_characteristic(&s, SCREEN_COMMAND_UUID);

        p.subscribe(&ack).await.unwrap();
        p.subscribe(&screen_command).await.unwrap();

        Self {
            command,
            _ack: ack,
            ack_channel,
            transmission_description,
            _screen_command: screen_command,
            screen_command_channel: Some(screen_command_channel),
        }
    }

    async fn write_transmission_description(
        &self,
        p: &Peripheral,
        num_x_bits: u8,
        num_y_bits: u8,
        num_crc_bits: u8,
    ) {
        p.write(
            &self.transmission_description,
            &[num_x_bits, num_y_bits, num_crc_bits],
            WriteType::WithResponse,
        )
        .await
        .unwrap();
    }
}

impl LocationService {
    async fn new(
        s: Service,
        p: &Peripheral,
        location_channel: UnboundedReceiver<common::Location>,
    ) -> Self {
        use btle_constants::location_service::characteristics::*;

        let location = get_characteristic(&s, LOCATION_UUID);

        p.subscribe(&location).await.unwrap();

        Self {
            _location: location,
            location_channel: Some(location_channel),
        }
    }
}

impl Mini {
    async fn new(p: Peripheral) -> Self {
        debug!("Connecting");
        p.connect().await.unwrap();

        debug!("Discovering Services");
        p.discover_services().await.unwrap();

        let (ack_sender, ack_receiver) = tokio::sync::mpsc::unbounded_channel();
        let (screen_cmd_sender, screen_cmd_receiver) = tokio::sync::mpsc::unbounded_channel();
        let command_service = CommandService::new(
            get_service(&p, btle_constants::command_service::SERVICE_UUID),
            &p,
            ack_receiver,
            screen_cmd_receiver,
        )
        .await;

        let (location_sender, location_receiver) = tokio::sync::mpsc::unbounded_channel();
        let location_service = LocationService::new(
            get_service(&p, btle_constants::location_service::SERVICE_UUID),
            &p,
            location_receiver,
        )
        .await;

        let mut notifications = p.notifications().await.unwrap();

        tokio::spawn(async move {
            while let Some(notification) = notifications.next().await {
                match notification.uuid {
                    btle_constants::command_service::characteristics::ACK_UUID => {
                        if let Ok(ack_val) = common::CommandAck::try_from(notification.value[0]) {
                            ack_sender.send(ack_val).unwrap();
                        }
                    }
                    btle_constants::command_service::characteristics::SCREEN_COMMAND_UUID => {
                        if let Ok(screen_command) =
                            common::ScreenCommands::try_from(notification.value[0])
                        {
                            screen_cmd_sender.send(screen_command).unwrap();
                        }
                    }
                    btle_constants::location_service::characteristics::LOCATION_UUID => {
                        if let Ok(location_val) = common::Location::binary_deserialize(
                            &notification.value,
                            binary_serde::Endianness::Little,
                        ) {
                            location_sender.send(location_val).unwrap();
                        }
                    }
                    _ => {
                        warn!(
                            "Received indication/notification from unknown uuid: {}",
                            notification.uuid
                        );
                    }
                }
            }
        });

        Self {
            peripheral: p,
            command_service,
            location_service,
        }
    }

    async fn send_command(&mut self, cmd: common::MiniCommands) -> common::CommandAck {
        self.peripheral
            .write(
                &self.command_service.command,
                &[cmd as u8],
                WriteType::WithResponse,
            )
            .await
            .unwrap();

        let ack_val = self.command_service.ack_channel.recv().await.unwrap();

        ack_val
    }

    async fn register_screen_channel(&mut self, screen_channel: UserEventSender<MiniMessage>) {
        let mut location_channel = self.location_service.location_channel.take().unwrap();
        let mut screen_command_channel =
            self.command_service.screen_command_channel.take().unwrap();
        tokio::spawn(async move {
            loop {
                let screen_cmd_future = screen_command_channel.recv();
                pin!(screen_cmd_future);
                let location_future = location_channel.recv();
                pin!(location_future);
                match select(screen_cmd_future, location_future).await {
                    Either::Left(screen_cmd) => {
                        if let Some(screen_cmd) = screen_cmd.0 {
                            screen_channel
                                .send_event(MiniMessage::Command(screen_cmd))
                                .unwrap();
                        }
                    }
                    Either::Right(location_cmd) => {
                        if let Some(location) = location_cmd.0 {
                            screen_channel
                                .send_event(MiniMessage::Position(location))
                                .unwrap();
                        }
                    }
                }
            }
        });
    }
}

pub fn get_service(periph: &Peripheral, uuid: uuid::Uuid) -> Service {
    periph
        .services()
        .iter()
        .find(|s| s.uuid == uuid)
        .unwrap()
        .clone()
}

pub fn get_characteristic(service: &Service, uuid: uuid::Uuid) -> Characteristic {
    service
        .characteristics
        .iter()
        .find(|c| c.uuid == uuid)
        .unwrap()
        .clone()
}

pub async fn find_mini(central: &Adapter) -> Peripheral {
    loop {
        let p = central.peripherals().await.unwrap();
        if p.len() >= 1 {
            return p[0].clone();
        } else {
            time::sleep(Duration::from_millis(100)).await;
        }
    }
}

pub struct Transmission {
    mini_errors: Vec<Option<common::CommandAck>>,
    remaining_bits: u8,
}

impl Transmission {
    async fn start(
        minis: &mut Vec<Mini>,
        num_x_bits: u8,
        num_y_bits: u8,
        num_crc_bits: u8,
    ) -> Self {
        let mut mini_errors = vec![None; minis.len()];
        for (i, mini) in minis.iter_mut().enumerate() {
            mini.command_service
                .write_transmission_description(
                    &mini.peripheral,
                    num_x_bits,
                    num_y_bits,
                    num_crc_bits,
                )
                .await;
            let mini_addr = mini.peripheral.address();
            let mini_error = mini
                .send_command(common::MiniCommands::StartTransmission)
                .await;
            println!("{:?} : {:?}", mini_addr, mini_error);
            if let common::CommandAck::Error = mini_error {
                mini_errors[i] = Some(mini_error);
            }
        }
        Self {
            mini_errors,
            remaining_bits: num_x_bits + num_y_bits + num_crc_bits,
        }
    }

    async fn send_command_to_all(&mut self, minis: &mut Vec<Mini>, cmd: common::MiniCommands) {
        for (i, mini) in minis.iter_mut().enumerate() {
            let mini_addr = mini.peripheral.address();
            let mini_error = mini.send_command(cmd).await;
            println!("{:?} : {:?}", mini_addr, mini_error);
            if let common::CommandAck::Error = mini_error {
                self.mini_errors[i] = Some(mini_error);
            }
        }
    }

    async fn calibrate_high(&mut self, minis: &mut Vec<Mini>) {
        self.send_command_to_all(minis, common::MiniCommands::CalibrateHigh)
            .await
    }

    async fn calibrate_low(&mut self, minis: &mut Vec<Mini>) {
        self.send_command_to_all(minis, common::MiniCommands::CalibrateLow)
            .await
    }

    async fn send_bit(&mut self, minis: &mut Vec<Mini>) -> bool {
        if self.remaining_bits > 0 {
            self.send_command_to_all(minis, common::MiniCommands::BitSent)
                .await;
            self.remaining_bits -= 1;

            if self.remaining_bits == 0 {
                self.send_command_to_all(minis, common::MiniCommands::EndTransmission)
                    .await;
                return false;
            } else {
                return true;
            }
        }

        false
    }
}

pub struct MiniManager {
    minis: Vec<Mini>,
    pub active_transmission: Option<Transmission>,
}

impl MiniManager {
    pub async fn new() -> Result<Self> {
        let manager = Manager::new().await.unwrap();

        // get the first bluetooth adapter
        let adapters = manager.adapters().await?;
        let central = adapters.into_iter().nth(0).unwrap();

        // start scanning for devices
        central
            .start_scan(ScanFilter {
                services: vec![common::btle_constants::command_service::SERVICE_UUID], //uuid::uuid!("9e7312e0-2354-11eb-9f10-fbc30a62cf38")],
            })
            .await?;
        // instead of waiting, you can use central.events() to get a stream which will
        // notify you of new devices, for an example of that see examples/event_driven_discovery.rs

        info!("Searching for minis");
        let mini = Mini::new(find_mini(&central).await).await;
        info!("Found mini: {}", mini.peripheral.address());

        let minis = vec![mini];

        Ok(Self {
            minis,
            active_transmission: None,
        })
    }

    pub async fn register_screen_channel(&mut self, screen_channel: UserEventSender<MiniMessage>) {
        for mini in self.minis.iter_mut() {
            mini.register_screen_channel(screen_channel.clone()).await;
        }
    }

    pub async fn start_transmission(
        &mut self,
        num_x_bits: u8,
        num_y_bits: u8,
        num_crc_bits: u8,
    ) -> bool {
        if let None = self.active_transmission {
            self.active_transmission = Some(
                Transmission::start(&mut self.minis, num_x_bits, num_y_bits, num_crc_bits).await,
            );
            true
        } else {
            false
        }
    }

    pub async fn calibrate_high(&mut self) {
        if let Some(transmission) = &mut self.active_transmission {
            transmission.calibrate_high(&mut self.minis).await;
        }
    }

    pub async fn calibrate_low(&mut self) {
        if let Some(transmission) = &mut self.active_transmission {
            transmission.calibrate_low(&mut self.minis).await;
        }
    }

    pub async fn send_bit(&mut self) {
        if let Some(transmission) = &mut self.active_transmission {
            transmission.send_bit(&mut self.minis).await;
        }
    }

    pub fn end_transmission(&mut self) {
        self.active_transmission = None;
    }
}
