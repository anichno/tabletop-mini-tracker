use byteorder::{ByteOrder, LittleEndian};
use defmt::println;
use modular_bitfield_msb::prelude::*;

const FS_8G_PER_BIT: f32 = 0.000244;
const FS_500DPS_PER_BIT: f32 = 0.0175;

#[bitfield(bits = 8)]
pub struct StatusReg {
    #[skip]
    __: B5,
    pub tda: bool,
    pub gda: bool,
    pub xlda: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct Accelerometer {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

#[derive(Debug, Clone, Copy)]
pub struct Gyroscope {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

#[derive(Debug, Clone, Copy)]
pub struct SensorVals {
    pub accel: Option<Accelerometer>,
    pub gyro: Option<Gyroscope>,
}

impl SensorVals {
    pub fn new(vals: [u8; 12], status_reg: StatusReg) -> Self {
        let mut values = [0; 6];
        for (i, vals) in vals.chunks(2).enumerate() {
            let low = vals[0];
            let high = vals[1];
            values[i] = LittleEndian::read_i16(&[low, high]);
        }

        let gyro = if true {
            //status_reg.gda() {
            Some(Gyroscope {
                x: values[0] as f32 * FS_500DPS_PER_BIT,
                y: values[1] as f32 * FS_500DPS_PER_BIT,
                z: values[2] as f32 * FS_500DPS_PER_BIT,
            })
        } else {
            None
        };

        let accel = if true {
            //status_reg.xlda() {
            Some(Accelerometer {
                x: values[3] as f32 * FS_8G_PER_BIT,
                y: values[4] as f32 * FS_8G_PER_BIT,
                z: values[5] as f32 * FS_8G_PER_BIT,
            })
        } else {
            None
        };

        Self { accel, gyro }
    }
}

#[bitfield(bits = 8)]
pub struct Ctrl8Xlreg {
    pub hpcf_xl: B3,
    pub hp_ref_mode_xl: bool,
    pub fastsettl_mode_xl: bool,
    pub hp_slope_xl_en: bool,
    pub xl_fs_mode: bool,
    pub low_pass_on_6d: bool,
}
