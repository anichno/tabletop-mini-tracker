use uuid::Uuid;

pub const BASE_UUID: Uuid = Uuid::from_bytes([
    0x0c, 0xde, 0x00, 0x00, 0x51, 0xfd, 0x47, 0x4f, 0xb0, 0x21, 0xd2, 0x2d, 0x0c, 0xd0, 0x5e, 0xc5,
]);

pub mod command_service {
    use super::*;

    pub const SERVICE_UUID: Uuid = u16_to_uuid(0x1400);
    pub mod characteristics {
        use super::*;

        pub const COMMAND_UUID: Uuid = u16_to_uuid(0x1401);
        pub const ACK_UUID: Uuid = u16_to_uuid(0x1402);
        pub const TRANSMISSION_DESCRIPTION_UUID: Uuid = u16_to_uuid(0x1403);
        pub const SCREEN_COMMAND_UUID: Uuid = u16_to_uuid(0x1404);
    }
}

pub mod location_service {
    use super::*;

    pub const SERVICE_UUID: Uuid = u16_to_uuid(0x1500);
    pub mod characteristics {
        use super::*;

        pub const LOCATION_UUID: Uuid = u16_to_uuid(0x1501);
    }
}

const fn u16_to_uuid(val: u16) -> Uuid {
    let mut base_bytes = BASE_UUID.into_bytes();
    base_bytes[2] = ((val >> 8) & 0xff) as u8;
    base_bytes[3] = (val & 0xff) as u8;
    Uuid::from_bytes(base_bytes)
}
