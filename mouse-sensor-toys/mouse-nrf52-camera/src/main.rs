#![no_std]
#![no_main]

//! MISO    = P0_28
//! MOSI    = P0_30
//! SS      = P0_31
//! SCK     = P0_29
//! MT      = NC: Motion (active low interrupt line)
//! RST     = NC: Reset
//! GND     = GND
//! VIN     = VDD

use core::fmt::Debug;
use defmt::{debug, error, info, println, unwrap};
use embassy_executor::Spawner;
use embassy_nrf::gpio::{Level, Output, OutputDrive};
use embassy_nrf::spim::Spim;
use embassy_nrf::spis::MODE_3;
use embassy_nrf::{bind_interrupts, peripherals, spim, uarte};
use embassy_sync::mutex::Mutex;
use embassy_time::Timer;
use embedded_hal_async::spi::{Operation, SpiDevice};
use {defmt_rtt as _, panic_probe as _};

mod registers;
mod srom;

mod timing {
    //! All times in ns

    pub const NCS_SCLK: u64 = 120;
    pub const WAKEUP: u64 = 50 * 1000 * 1000;
    pub const SWW: u64 = 180 * 1000;
    pub const SWR: u64 = 180 * 1000;
    pub const SRW: u64 = 20 * 1000;
    pub const SRR: u64 = 20 * 1000;
    pub const SRAD: u64 = 160 * 1000;
    pub const SRAD_MOTBR: u64 = 35 * 1000;
    pub const SCLK_NCS: u64 = 35 * 1000;
}

const IMAGE_WIDTH: usize = 36;
const IMAGE_HEIGHT: usize = 36;
const TOTAL_PIXELS: usize = IMAGE_WIDTH * IMAGE_HEIGHT;

bind_interrupts!(struct Irqs {
    SPIM3 => spim::InterruptHandler<peripherals::SPI3>;
    UARTE0 => uarte::InterruptHandler<peripherals::UARTE0>;

});

#[derive(Debug, thiserror::Error, defmt::Format)]
pub enum Pmw3360Error<Error: Debug> {
    #[error("Invalid signature")]
    InvalidSignature,
    #[error("SPI: {0:?}")]
    Spi(Error),
    #[error("Not supported")]
    NotSupported,
}

pub struct Pmw3360<'a> {
    spi: Spim<'a, peripherals::SPI3>,
    cs: Output<'a>, // in_burst_mode: bool,
}
impl<'a> Pmw3360<'a> {
    async fn write(
        &mut self,
        address: u8,
        data: u8,
    ) -> Result<(), Pmw3360Error<embassy_nrf::spim::Error>> {
        self.cs.set_low();
        Timer::after_nanos(timing::NCS_SCLK).await;

        self.spi
            .blocking_write(&[address | 0x80, data])
            .map_err(Pmw3360Error::Spi)?;

        Timer::after_nanos(timing::SCLK_NCS).await;
        self.cs.set_high();

        Ok(())
    }

    async fn read(&mut self, address: u8) -> Result<u8, Pmw3360Error<embassy_nrf::spim::Error>> {
        let mut buf = [0x00];

        self.cs.set_low();
        Timer::after_nanos(timing::NCS_SCLK).await;

        self.spi
            .blocking_write(&[address])
            .map_err(Pmw3360Error::Spi)?;
        Timer::after_nanos(timing::SRAD).await;
        self.spi
            .blocking_read(&mut buf)
            .map_err(Pmw3360Error::Spi)?;

        Timer::after_nanos(timing::SCLK_NCS).await;
        self.cs.set_high();

        Ok(buf[0])
    }

    pub async fn check_signature(
        &mut self,
    ) -> Result<bool, Pmw3360Error<embassy_nrf::spim::Error>> {
        let srom = self.read(registers::SROM_ID).await.unwrap_or(0);
        let pid = self.read(registers::PRODUCT_ID).await.unwrap_or(0);
        let ipid = self.read(registers::INVERSE_PRODUCT_ID).await.unwrap_or(0);

        // debug!("pid: {:x}, ipid: {:x}, srom: {:x}", pid, ipid, srom);

        // signature for SROM 0x04
        Ok(srom == 0x04 && pid == 0x42 && ipid == 0xBD)
    }

    #[allow(dead_code)]
    pub async fn self_test(&mut self) -> Result<bool, Pmw3360Error<embassy_nrf::spim::Error>> {
        self.write(registers::SROM_ENABLE, 0x15).await?;
        Timer::after_micros(10000).await;

        let u = self.read(registers::DATA_OUT_UPPER).await.unwrap_or(0); // should be 0xBE
        let l = self.read(registers::DATA_OUT_LOWER).await.unwrap_or(0); // should be 0xEF

        // debug!("u: {:x}, l: {:x}", u, l);
        Ok(u == 0xBE && l == 0xEF)
    }

    async fn power_up(&mut self) -> Result<(), Pmw3360Error<embassy_nrf::spim::Error>> {
        let is_valid_signature = self.power_up_inner().await?;
        if is_valid_signature {
            Ok(())
        } else {
            Err(Pmw3360Error::InvalidSignature)
        }
    }

    async fn power_up_inner(&mut self) -> Result<bool, Pmw3360Error<embassy_nrf::spim::Error>> {
        // hard reset
        self.cs.set_low();
        Timer::after_nanos(timing::NCS_SCLK).await;

        Timer::after_micros(1).await;
        self.cs.set_high();

        self.cs.set_low();
        Timer::after_nanos(timing::NCS_SCLK).await;

        Timer::after_micros(1).await;
        self.cs.set_high();

        self.write(registers::SHUTDOWN, 0xb6).await?;
        Timer::after_millis(300).await;

        // Write to reset register
        self.write(registers::POWER_UP_RESET, 0x5A).await?;

        // Wait at least 50ms
        Timer::after_millis(100).await;

        // read registers 0x02 to 0x06
        self.read(registers::MOTION).await?;
        self.read(registers::DELTA_X_L).await?;
        self.read(registers::DELTA_X_H).await?;
        self.read(registers::DELTA_Y_L).await?;
        self.read(registers::DELTA_Y_H).await?;

        // perform SROM download
        self.srom_download().await?;

        let is_valid_signature = self.check_signature().await.unwrap_or(false);

        // self.write(registers::CONFIG_1, 0x77).await?;

        // Write 0x00 (rest disable) to Config2 register for wired mouse or 0x20 for
        // wireless mouse design.
        self.write(registers::CONFIG_2, 0x00).await?;

        Timer::after_micros(100).await;

        Ok(is_valid_signature)
    }

    async fn srom_download(&mut self) -> Result<(), Pmw3360Error<embassy_nrf::spim::Error>> {
        // Write 0 to Rest_En bit of Config2 register to disable Rest mode
        self.write(registers::CONFIG_2, 0x00).await?;

        // Write 0x1d to SROM_Enable register for initializing
        self.write(registers::SROM_ENABLE, 0x1d).await?;

        // Wait for 10 ms
        Timer::after_millis(10).await;

        // Write 0x18 to SROM_Enable register again to start SROM Download
        self.write(registers::SROM_ENABLE, 0x18).await?;

        // Write SROM file into SROM_Load_Burst register, 1st data must start with SROM_Load_Burst address. All the SROM data must be downloaded before SROM starts running
        self.cs.set_low();
        Timer::after_nanos(timing::NCS_SCLK).await;

        self.spi
            .blocking_write(&[registers::SROM_LOAD_BURST | 0x80])
            .map_err(Pmw3360Error::Spi)?;
        Timer::after_micros(15).await;

        for b in srom::FIRMWARE_DATA {
            self.spi.blocking_write(&[*b]).map_err(Pmw3360Error::Spi)?;
            Timer::after_micros(15).await;
        }
        debug!("fw len: {}", srom::FIRMWARE_DATA.len());

        self.cs.set_high();
        Timer::after_micros(250).await;

        // Read the SROM_ID register to verify the ID before any other register reads or writes
        let id = self.read(registers::SROM_ID).await?;
        debug!("srom id: {:x}", id);

        // Write 0x00 to Config2 register for wired mouse or 0x20 for wireless mouse design
        self.write(registers::CONFIG_2, 0x00).await?;

        Ok(())
    }

    async fn frame_capture(
        &mut self,
    ) -> Result<[u8; TOTAL_PIXELS], Pmw3360Error<embassy_nrf::spim::Error>> {
        // Write 0 to Rest_En bit of Config2 register to disable Rest mode.
        self.write(registers::CONFIG_2, 0x00).await?;

        // Write 0x83 to Frame_Capture register
        self.write(registers::FRAME_CAPTURE, 0x83).await?;

        // Write 0xC5 to Frame_Capture register
        self.write(registers::FRAME_CAPTURE, 0xc5).await?;

        // Wait for 20ms
        Timer::after_millis(20).await;

        // Continue burst read from Raw_data_Burst register until all 1296 raw data are transferred.
        let mut frame_data = [0; TOTAL_PIXELS];

        self.cs.set_low();
        Timer::after_nanos(timing::NCS_SCLK).await;

        self.spi
            .blocking_write(&[registers::RAW_DATA_BURST])
            .map_err(Pmw3360Error::Spi)?;
        Timer::after_nanos(timing::SRAD).await;

        let mut buf = [0];
        for b in frame_data.iter_mut() {
            self.spi
                .blocking_read(&mut buf)
                .map_err(Pmw3360Error::Spi)?;
            *b = buf[0];
            Timer::after_micros(15).await;
        }

        self.cs.set_high();

        // tBEXIT
        Timer::after_micros(4).await;

        Ok(frame_data)
    }
}

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = embassy_nrf::init(Default::default());
    info!("running!");

    let mut config = uarte::Config::default();
    config.parity = uarte::Parity::EXCLUDED;
    config.baudrate = uarte::Baudrate::BAUD115200;

    let mut uart = uarte::Uarte::new(p.UARTE0, Irqs, p.P0_08, p.P0_06, config);

    info!("uarte initialized!");

    let mut config = spim::Config::default();
    config.frequency = spim::Frequency::M2;
    // config.mode.polarity = spim::Polarity::IdleHigh;
    // config.mode.phase = spim::Phase::CaptureOnSecondTransition;
    config.mode = MODE_3;

    let spim: Spim<'_, peripherals::SPI3> =
        spim::Spim::new(p.SPI3, Irqs, p.P0_29, p.P0_28, p.P0_30, config);
    let ncs = Output::new(p.P0_31, Level::High, OutputDrive::Standard);

    // let spi_bus: Mutex<
    //     embassy_sync::blocking_mutex::raw::NoopRawMutex,
    //     spim::Spim<'_, peripherals::SPI3>,
    // > = Mutex::new(spim);
    // let spi_device = embassy_embedded_hal::shared_bus::asynch::spi::SpiDevice::new(&spi_bus, ncs);

    let mut pmw3360 = Pmw3360 {
        spi: spim,
        // in_burst_mode: false,
        cs: ncs,
    };

    debug!("{:?}", pmw3360.power_up().await);

    Timer::after_millis(250).await;

    debug!("{:?}", pmw3360.self_test().await);

    loop {
        let frame = pmw3360.frame_capture().await;
        match frame {
            Ok(data) => {
                // debug!("{:?}", data);
                debug!("{}", data.len());
                uart.write(&data).await.unwrap();
                uart.write(&[b'F', b'R', b'A', b'M', b'E']).await.unwrap();
            }
            Err(e) => error!("frame capture error: {:?}", e),
        }
        Timer::after_millis(100).await;
    }
}
