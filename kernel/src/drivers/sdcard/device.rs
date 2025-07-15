// SD card device implementation for elinOS
// Provides block device interface for SD cards

use super::error::{SdCardError, SdCardResult};
use super::protocol::{SdCardProtocol, SpiInterface};
use super::commands::*;
use crate::drivers::spi::{SpiResult, SpiError, SpiConfig};
use crate::drivers::spi::controller::SpiController;
use crate::drivers::spi::gpio_spi::GpioSpi;
use crate::drivers::gpio::GpioSpiPins;
use elinos_common::console_println;
use spin::Mutex;
use core::convert::TryInto;

/// SD card device implementation
pub struct SdCardDevice {
    protocol: SdCardProtocol,
    spi: SdCardSpi,
    initialized: bool,
}

/// SPI interface wrapper for SD card
enum SdCardSpi {
    Hardware(SpiController),
    Gpio(GpioSpi),
}

impl SdCardDevice {
    /// Create new SD card device with hardware SPI
    pub fn new_hardware(spi_base: usize) -> Self {
        SdCardDevice {
            protocol: SdCardProtocol::new(),
            spi: SdCardSpi::Hardware(SpiController::new(spi_base)),
            initialized: false,
        }
    }
    
    /// Create new SD card device with GPIO SPI
    pub fn new_gpio(gpio_base: usize, pins: GpioSpiPins) -> Self {
        SdCardDevice {
            protocol: SdCardProtocol::new(),
            spi: SdCardSpi::Gpio(GpioSpi::new(gpio_base, pins)),
            initialized: false,
        }
    }
    
    /// Initialize SD card device
    pub fn init(&mut self) -> SdCardResult<()> {
        // Configure SPI for SD card (400kHz for initialization)
        let config = SpiConfig {
            clock_rate: 400_000, // 400kHz
            mode: crate::drivers::spi::controller::SpiMode::Mode0,
            bit_order: crate::drivers::spi::controller::SpiBitOrder::MsbFirst,
            cs_polarity: false,
        };
        
        // Initialize SPI interface
        match &mut self.spi {
            SdCardSpi::Hardware(spi) => {
                spi.init(config).map_err(|_| SdCardError::SpiError)?;
            }
            SdCardSpi::Gpio(spi) => {
                spi.init(config).map_err(|_| SdCardError::SpiError)?;
            }
        }
        
        // Initialize SD card protocol
        self.protocol.init(&mut self.spi)?;
        
        // Switch to higher speed after initialization
        let fast_config = SpiConfig {
            clock_rate: 25_000_000, // 25MHz
            ..config
        };
        
        match &mut self.spi {
            SdCardSpi::Hardware(spi) => {
                spi.init(fast_config).map_err(|_| SdCardError::SpiError)?;
            }
            SdCardSpi::Gpio(spi) => {
                spi.init(fast_config).map_err(|_| SdCardError::SpiError)?;
            }
        }
        
        self.initialized = true;
        Ok(())
    }
    
    /// Read a single block from SD card
    pub fn read_block(&mut self, block_addr: u32, buffer: &mut [u8; 512]) -> SdCardResult<()> {
        if !self.initialized {
            return Err(SdCardError::NotInitialized);
        }
        
        if !self.protocol.is_ready() {
            return Err(SdCardError::InitializationFailed);
        }
        
        self.spi.cs_active()?;
        
        // Send CMD17 (READ_SINGLE_BLOCK)
        let cmd = SdCommand::new(CMD17_READ_SINGLE_BLOCK, block_addr);
        let response = self.send_command(cmd)?;
        
        if response.has_error() {
            self.spi.cs_inactive()?;
            return Err(SdCardError::ReadError);
        }
        
        // Wait for data start token
        let mut token = 0xFF;
        let mut timeout = 1000;
        
        while timeout > 0 && token != DATA_START_TOKEN {
            let tx = [0xFF];
            let mut rx = [0];
            self.spi.transfer(&tx, &mut rx)?;
            token = rx[0];
            timeout -= 1;
        }
        
        if token != DATA_START_TOKEN {
            self.spi.cs_inactive()?;
            return Err(SdCardError::ReadError);
        }
        
        // Read 512 bytes of data
        let tx = [0xFF; 512];
        self.spi.transfer(&tx, buffer)?;
        
        // Read CRC (2 bytes, ignored)
        let tx_crc = [0xFF, 0xFF];
        let mut rx_crc = [0u8; 2];
        self.spi.transfer(&tx_crc, &mut rx_crc)?;
        
        self.spi.cs_inactive()?;
        Ok(())
    }
    
    /// Write a single block to SD card
    pub fn write_block(&mut self, block_addr: u32, buffer: &[u8; 512]) -> SdCardResult<()> {
        if !self.initialized {
            return Err(SdCardError::NotInitialized);
        }
        
        if !self.protocol.is_ready() {
            return Err(SdCardError::InitializationFailed);
        }
        
        self.spi.cs_active()?;
        
        // Send CMD24 (WRITE_BLOCK)
        let cmd = SdCommand::new(CMD24_WRITE_BLOCK, block_addr);
        let response = self.send_command(cmd)?;
        
        if response.has_error() {
            self.spi.cs_inactive()?;
            return Err(SdCardError::WriteError);
        }
        
        // Send data start token
        let tx_token = [DATA_START_TOKEN];
        let mut rx_token = [0];
        self.spi.transfer(&tx_token, &mut rx_token)?;
        
        // Send 512 bytes of data
        let mut rx_data = [0u8; 512];
        self.spi.transfer(buffer, &mut rx_data)?;
        
        // Send dummy CRC (2 bytes)
        let tx_crc = [0xFF, 0xFF];
        let mut rx_crc = [0u8; 2];
        self.spi.transfer(&tx_crc, &mut rx_crc)?;
        
        // Read data response
        let tx_resp = [0xFF];
        let mut rx_resp = [0];
        self.spi.transfer(&tx_resp, &mut rx_resp)?;
        
        let data_response = rx_resp[0] & DATA_RESPONSE_MASK;
        if data_response != DATA_RESPONSE_ACCEPTED {
            self.spi.cs_inactive()?;
            return Err(SdCardError::WriteError);
        }
        
        // Wait for card to complete write operation
        let mut busy = 0;
        let mut timeout = 10000;
        
        while timeout > 0 && busy == 0 {
            let tx = [0xFF];
            let mut rx = [0];
            self.spi.transfer(&tx, &mut rx)?;
            busy = rx[0];
            timeout -= 1;
        }
        
        if busy == 0 {
            self.spi.cs_inactive()?;
            return Err(SdCardError::WriteError);
        }
        
        self.spi.cs_inactive()?;
        Ok(())
    }
    
    /// Read multiple blocks from SD card
    pub fn read_blocks(&mut self, start_block: u32, buffer: &mut [u8]) -> SdCardResult<()> {
        if buffer.len() % 512 != 0 {
            return Err(SdCardError::InvalidSector);
        }
        
        let block_count = buffer.len() / 512;
        
        for i in 0..block_count {
            let block_addr = start_block + i as u32;
            let offset = i * 512;
            let block_buffer = &mut buffer[offset..offset + 512];
            let block_array: &mut [u8; 512] = block_buffer.try_into()
                .map_err(|_| SdCardError::InvalidSector)?;
            
            self.read_block(block_addr, block_array)?;
        }
        
        Ok(())
    }
    
    /// Write multiple blocks to SD card
    pub fn write_blocks(&mut self, start_block: u32, buffer: &[u8]) -> SdCardResult<()> {
        if buffer.len() % 512 != 0 {
            return Err(SdCardError::InvalidSector);
        }
        
        let block_count = buffer.len() / 512;
        
        for i in 0..block_count {
            let block_addr = start_block + i as u32;
            let offset = i * 512;
            let block_buffer = &buffer[offset..offset + 512];
            let block_array: &[u8; 512] = block_buffer.try_into()
                .map_err(|_| SdCardError::InvalidSector)?;
            
            self.write_block(block_addr, block_array)?;
        }
        
        Ok(())
    }
    
    /// Get SD card capacity in sectors
    pub fn get_capacity(&self) -> u32 {
        self.protocol.get_info().capacity_sectors
    }
    
    /// Check if SD card is initialized
    pub fn is_initialized(&self) -> bool {
        self.initialized && self.protocol.is_ready()
    }
    
    /// Send command to SD card
    fn send_command(&mut self, cmd: SdCommand) -> SdCardResult<SdResponse> {
        let cmd_bytes = cmd.to_bytes();
        
        // Send command
        let mut dummy = [0u8; 6];
        self.spi.transfer(&cmd_bytes, &mut dummy)?;
        
        // Wait for response
        let mut response = 0xFF;
        let mut timeout = 100;
        
        while timeout > 0 && response == 0xFF {
            let tx = [0xFF];
            let mut rx = [0];
            self.spi.transfer(&tx, &mut rx)?;
            response = rx[0];
            timeout -= 1;
        }
        
        if response == 0xFF {
            return Err(SdCardError::CommandTimeout);
        }
        
        Ok(SdResponse::new(response))
    }
}

// Implement SpiInterface for SdCardSpi
impl SpiInterface for SdCardSpi {
    fn transfer(&mut self, tx_data: &[u8], rx_data: &mut [u8]) -> SpiResult<()> {
        match self {
            SdCardSpi::Hardware(spi) => spi.transfer(tx_data, rx_data),
            SdCardSpi::Gpio(spi) => spi.transfer(tx_data, rx_data),
        }
    }
    
    fn cs_active(&mut self) -> SpiResult<()> {
        match self {
            SdCardSpi::Hardware(spi) => spi.cs_active(),
            SdCardSpi::Gpio(spi) => spi.cs_active(),
        }
    }
    
    fn cs_inactive(&mut self) -> SpiResult<()> {
        match self {
            SdCardSpi::Hardware(spi) => spi.cs_inactive(),
            SdCardSpi::Gpio(spi) => spi.cs_inactive(),
        }
    }
}

// Global SD card instance
static SD_CARD: Mutex<Option<SdCardDevice>> = Mutex::new(None);

/// Initialize SD card with GPIO SPI
pub fn init_sdcard_gpio(gpio_base: usize, pins: GpioSpiPins) -> SdCardResult<()> {
    let mut device = SdCardDevice::new_gpio(gpio_base, pins);
    device.init()?;
    
    *SD_CARD.lock() = Some(device);
    console_println!("[o] SD card initialized with GPIO SPI");
    Ok(())
}

/// Initialize SD card with hardware SPI
pub fn init_sdcard_hardware(spi_base: usize) -> SdCardResult<()> {
    let mut device = SdCardDevice::new_hardware(spi_base);
    device.init()?;
    
    *SD_CARD.lock() = Some(device);
    console_println!("[o] SD card initialized with hardware SPI");
    Ok(())
}

/// Read blocks from SD card
pub fn read_sdcard_blocks(start_block: u32, buffer: &mut [u8]) -> SdCardResult<()> {
    let mut sd_card = SD_CARD.lock();
    if let Some(device) = sd_card.as_mut() {
        device.read_blocks(start_block, buffer)
    } else {
        Err(SdCardError::NotInitialized)
    }
}

/// Write blocks to SD card
pub fn write_sdcard_blocks(start_block: u32, buffer: &[u8]) -> SdCardResult<()> {
    let mut sd_card = SD_CARD.lock();
    if let Some(device) = sd_card.as_mut() {
        device.write_blocks(start_block, buffer)
    } else {
        Err(SdCardError::NotInitialized)
    }
}

/// Get SD card capacity
pub fn get_sdcard_capacity() -> u32 {
    let sd_card = SD_CARD.lock();
    if let Some(device) = sd_card.as_ref() {
        device.get_capacity()
    } else {
        0
    }
}