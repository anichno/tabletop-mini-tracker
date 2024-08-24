use binary_serde::BinarySerde;
use common::btle_constants;

fn print_command_service() {
    println!("\ncommand service:");
    println!(
        "SERVICE_UUID: {}",
        btle_constants::command_service::SERVICE_UUID
    );
    println!(
        "COMMAND_UUID: {}",
        btle_constants::command_service::characteristics::COMMAND_UUID
    );
    println!(
        "ACK_UUID: {}",
        btle_constants::command_service::characteristics::ACK_UUID
    );
    println!(
        "TRANSMISSION_DESCRIPTION_UUID: {}",
        btle_constants::command_service::characteristics::TRANSMISSION_DESCRIPTION_UUID
    );
    println!(
        "SCREEN_COMMAND_UUID: {}",
        btle_constants::command_service::characteristics::SCREEN_COMMAND_UUID
    );
}

fn print_location_service() {
    println!("\nlocation service:");
    println!(
        "SERVICE_UUID: {}",
        btle_constants::location_service::SERVICE_UUID
    );
    println!(
        "LOCATION_UUID: {}",
        btle_constants::location_service::characteristics::LOCATION_UUID
    );
}

fn main() {
    println!("BASE_UUID: {}", btle_constants::BASE_UUID);

    print_command_service();
    print_location_service();
}
