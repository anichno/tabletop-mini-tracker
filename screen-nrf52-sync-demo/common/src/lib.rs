#![no_std]

use binary_serde::BinarySerde;
use num_enum::TryFromPrimitive;
use serde::{Deserialize, Serialize};

pub mod btle_constants;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, TryFromPrimitive)]
#[repr(u8)]
pub enum MiniCommands {
    StartTransmission,
    CalibrateLow,
    CalibrateHigh,
    BitSent,
    EndTransmission,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, TryFromPrimitive)]
#[repr(u8)]
pub enum ScreenCommands {
    TriggerTransmission,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, TryFromPrimitive)]
#[repr(u8)]
pub enum CommandAck {
    Ok,
    Error,
}

// #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
// pub struct TransmissionCharacteristics {
//     pub num_x_bits: u8,
//     pub num_y_bits: u8,
//     pub num_crc_bits: u8,
// }

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, BinarySerde,
)]
pub struct Location {
    pub x: u32,
    pub y: u32,
    pub z: u32,
    pub error: u32,
}
