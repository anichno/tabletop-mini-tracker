use binary_serde::BinarySerde;
use btleplug::api::{
    Central, Characteristic, Manager as _, Peripheral as _, ScanFilter, Service, WriteType,
};
use btleplug::platform::{Adapter, Manager, Peripheral};
use common::btle_constants;
use futures::StreamExt;
use log::{info, warn};
use std::error::Error;
use std::time::Duration;
use tokio::sync::mpsc::UnboundedReceiver;
use tokio::time;

struct Transmission<'a> {
    minis: &'a mut Vec<Mini>,
    mini_errors: Vec<Option<common::CommandAck>>,
    remaining_bits: u8,
}

impl<'a> Transmission<'a> {
    async fn start(
        minis: &'a mut Vec<Mini>,
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
            minis: minis,
            mini_errors,
            remaining_bits: num_x_bits + num_y_bits + num_crc_bits,
        }
    }

    async fn send_command_to_all(&mut self, cmd: common::MiniCommands) {
        for (i, mini) in self.minis.iter_mut().enumerate() {
            let mini_addr = mini.peripheral.address();
            let mini_error = mini.send_command(cmd).await;
            println!("{:?} : {:?}", mini_addr, mini_error);
            if let common::CommandAck::Error = mini_error {
                self.mini_errors[i] = Some(mini_error);
            }
        }
    }

    async fn calibrate_high(&mut self) {
        self.send_command_to_all(common::MiniCommands::CalibrateHigh)
            .await
    }

    async fn calibrate_low(&mut self) {
        self.send_command_to_all(common::MiniCommands::CalibrateLow)
            .await
    }

    async fn send_bit(&mut self) -> bool {
        if self.remaining_bits > 0 {
            self.send_command_to_all(common::MiniCommands::BitSent)
                .await;
            self.remaining_bits -= 1;

            if self.remaining_bits == 0 {
                self.send_command_to_all(common::MiniCommands::EndTransmission)
                    .await;
                return false;
            } else {
                return true;
            }
        }

        false
    }
}

struct Mini {
    peripheral: Peripheral,
    command_service: CommandService,
    location_service: LocationService,
}

struct CommandService {
    command: Characteristic,
    ack: Characteristic,
    ack_channel: UnboundedReceiver<common::CommandAck>,
    transmission_description: Characteristic,
    screen_command: Characteristic,
    screen_command_channel: UnboundedReceiver<common::ScreenCommands>,
}

struct LocationService {
    location: Characteristic,
    location_channel: UnboundedReceiver<common::Location>,
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
            ack,
            ack_channel,
            transmission_description,
            screen_command,
            screen_command_channel,
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
            location,
            location_channel,
        }
    }
}

impl Mini {
    async fn new(p: Peripheral) -> Self {
        p.connect().await.unwrap();
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
}

fn get_service(periph: &Peripheral, uuid: uuid::Uuid) -> Service {
    periph
        .services()
        .iter()
        .find(|s| s.uuid == uuid)
        .unwrap()
        .clone()
}

fn get_characteristic(service: &Service, uuid: uuid::Uuid) -> Characteristic {
    service
        .characteristics
        .iter()
        .find(|c| c.uuid == uuid)
        .unwrap()
        .clone()
}

async fn find_mini(central: &Adapter) -> Peripheral {
    loop {
        let p = central.peripherals().await.unwrap();
        if p.len() >= 1 {
            return p[0].clone();
        } else {
            time::sleep(Duration::from_millis(100)).await;
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();

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

    let mini = Mini::new(find_mini(&central).await).await;
    info!("Found mini: {}", mini.peripheral.address());

    let mut minis = vec![mini];

    loop {
        info!("Waiting for command from mini");
        match minis[0]
            .command_service
            .screen_command_channel
            .recv()
            .await
            .unwrap()
        {
            common::ScreenCommands::TriggerTransmission => {
                let mut transmission = Transmission::start(&mut minis, 10, 10, 4).await;

                transmission.calibrate_low().await;
                transmission.calibrate_high().await;

                while transmission.send_bit().await {}
            }
        }
    }

    // Ok(())
}
