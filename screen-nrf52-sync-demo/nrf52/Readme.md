https://devzone.nordicsemi.com/f/nordic-q-a/95314/nrf52840-dk-3-0-0-device-protection-is-not-removed-after-power-up

Softdevice S113 download from: https://www.nordicsemi.com/Products/Development-software/s113/download
probe-rs erase --chip nrf52840_xxAA --allow-erase-all
probe-rs download --verify --binary-format hex --chip nRF52840_xxAA s113_nrf52_softdevice/s113_nrf52_7.3.0_softdevice.hex