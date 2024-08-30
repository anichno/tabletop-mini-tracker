#![no_std]

use num_enum::{IntoPrimitive, TryFromPrimitive};

#[derive(IntoPrimitive, TryFromPrimitive)]
#[repr(u8)]
pub enum Commands {
    Ping,
    GetLightValue,
    SetLightConversionTime,
    CalibrateDark,
    CalibrateLight,
    CalibrateMiddle,
    GetFrameDelay,
}
