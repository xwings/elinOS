// SD card protocol implementation for elinOS
// Implements low-level SD card communication over SPI

use super::error::{SdCardError, SdCardResult};
use super::commands::*;
use crate::drivers::spi::{SpiResult, SpiError};
use elinos_common::console_println;

/// SD card protocol state
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SdCardState {
    Uninitialized,
    Initializing,
    Idle,
    Ready,
    Active,
    Error,
}

/// SD card type
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SdCardType {
    Unknown,
    SdscV1,     // SD Standard Capacity v1.x
    SdscV2,     // SD Standard Capacity v2.0
    Sdhc,       // SD High Capacity
    Sdxc,       // SD eXtended Capacity
    MmcV3,      // MultiMediaCard v3
}

/// SD card information
#[derive(Debug, Clone, Copy)]
pub struct SdCardInfo {
    pub card_type: SdCardType,
    pub capacity_sectors: u32,
    pub block_size: u16,
    pub cid: [u8; 16],    // Card Identification
    pub csd: [u8; 16],    // Card Specific Data
    pub ocr: u32,         // Operating Conditions Register
}

impl Default for SdCardInfo {
    fn default() -> Self {
        SdCardInfo {
            card_type: SdCardType::Unknown,
            capacity_sectors: 0,
            block_size: 512,
            cid: [0; 16],
            csd: [0; 16],
            ocr: 0,
        }
    }
}

/// SD card protocol implementation
pub struct SdCardProtocol {
    state: SdCardState,
    info: SdCardInfo,
}

impl SdCardProtocol {
    pub fn new() -> Self {
        SdCardProtocol {
            state: SdCardState::Uninitialized,
            info: SdCardInfo::default(),
        }
    }
    
    /// Initialize SD card
    pub fn init<T: SpiInterface>(&mut self, spi: &mut T) -> SdCardResult<()> {
        console_println!("[i] Initializing SD card...");
        self.state = SdCardState::Initializing;
        
        // Step 1: Send 74+ clock cycles with CS high
        spi.cs_inactive()?;
        self.send_clocks(spi, 10)?; // 10 bytes = 80 clocks
        
        // Step 2: Send CMD0 (GO_IDLE_STATE)
        spi.cs_active()?;
        let response = self.send_command(spi, SdCommand::new(CMD0_GO_IDLE_STATE, 0))?;
        if !response.is_idle() {
            spi.cs_inactive()?;
            return Err(SdCardError::InitializationFailed);
        }
        
        // Step 3: Send CMD8 (SEND_IF_COND) to check voltage range
        let cmd8_response = self.send_command(spi, SdCommand::new(CMD8_SEND_IF_COND, 0x1AA))?;
        let is_v2_card = cmd8_response.is_valid() && !cmd8_response.has_error();
        
        // Step 4: Initialize card with ACMD41
        let mut timeout = 1000;
        let mut card_ready = false;
        
        while timeout > 0 && !card_ready {
            // Send CMD55 (APP_CMD) followed by ACMD41
            let cmd55_response = self.send_command(spi, SdCommand::new(CMD55_APP_CMD, 0))?;
            if cmd55_response.is_valid() {
                let acmd41_arg = if is_v2_card { 0x40000000 } else { 0 };
                let acmd41_response = self.send_command(spi, SdCommand::new(ACMD41_SD_SEND_OP_COND, acmd41_arg))?;
                
                if acmd41_response.is_valid() && !acmd41_response.is_idle() {
                    card_ready = true;
                }
            }
            
            if !card_ready {
                timeout -= 1;
                self.delay_ms(1);
            }
        }
        
        if !card_ready {
            spi.cs_inactive()?;
            return Err(SdCardError::InitializationFailed);
        }
        
        // Step 5: Read OCR register (CMD58)
        let ocr_response = self.send_command(spi, SdCommand::new(CMD58_READ_OCR, 0))?;
        if ocr_response.is_valid() {
            self.info.ocr = u32::from_be_bytes([
                ocr_response.data[0],
                ocr_response.data[1],
                ocr_response.data[2],
                ocr_response.data[3],
            ]);
            
            // Determine card type based on OCR
            if is_v2_card {
                if (self.info.ocr & 0x40000000) != 0 {
                    self.info.card_type = SdCardType::Sdhc; // Could be SDHC or SDXC
                } else {
                    self.info.card_type = SdCardType::SdscV2;
                }
            } else {
                self.info.card_type = SdCardType::SdscV1;
            }
        }
        
        // Step 6: Read CSD register (CMD9)
        if let Ok(csd) = self.read_register(spi, CMD9_SEND_CSD) {
            self.info.csd = csd;
            self.calculate_capacity();
        }
        
        // Step 7: Read CID register (CMD10)
        if let Ok(cid) = self.read_register(spi, CMD10_SEND_CID) {
            self.info.cid = cid;
        }
        
        // Step 8: Set block size to 512 bytes (CMD16)
        if self.info.card_type == SdCardType::SdscV1 || self.info.card_type == SdCardType::SdscV2 {
            let block_response = self.send_command(spi, SdCommand::new(CMD16_SET_BLOCKLEN, 512))?;
            if block_response.has_error() {
                spi.cs_inactive()?;
                return Err(SdCardError::InitializationFailed);
            }
        }
        
        spi.cs_inactive()?;
        self.state = SdCardState::Ready;
        
        console_println!("[o] SD card initialized successfully");
        console_println!("    Type: {:?}", self.info.card_type);
        console_println!("    Capacity: {} sectors", self.info.capacity_sectors);
        console_println!("    Block size: {} bytes", self.info.block_size);
        
        Ok(())
    }
    
    /// Send command to SD card
    fn send_command<T: SpiInterface>(&self, spi: &mut T, cmd: SdCommand) -> SdCardResult<SdResponse> {
        let cmd_bytes = cmd.to_bytes();
        
        // Send command
        let mut dummy = [0u8; 6];
        spi.transfer(&cmd_bytes, &mut dummy)?;
        
        // Wait for response (R1)
        let mut response = 0xFF;
        let mut timeout = 100;
        
        while timeout > 0 && response == 0xFF {
            let tx = [0xFF];
            let mut rx = [0];
            spi.transfer(&tx, &mut rx)?;
            response = rx[0];
            timeout -= 1;
        }
        
        if response == 0xFF {
            return Err(SdCardError::CommandTimeout);
        }
        
        let mut sd_response = SdResponse::new(response);
        
        // For certain commands, read additional response bytes
        match cmd.cmd {
            CMD58_READ_OCR => {
                // R3 response: R1 + 32-bit OCR
                let mut data = [0u8; 4];
                let tx = [0xFF, 0xFF, 0xFF, 0xFF];
                spi.transfer(&tx, &mut data)?;
                sd_response = SdResponse::with_data(response, data);
            }
            _ => {}
        }
        
        Ok(sd_response)
    }
    
    /// Read a register from SD card
    fn read_register<T: SpiInterface>(&self, spi: &mut T, cmd: u8) -> SdCardResult<[u8; 16]> {
        let response = self.send_command(spi, SdCommand::new(cmd, 0))?;
        if response.has_error() {
            return Err(SdCardError::CommandError);
        }
        
        // Wait for data start token
        let mut token = 0xFF;
        let mut timeout = 1000;
        
        while timeout > 0 && token != DATA_START_TOKEN {
            let tx = [0xFF];
            let mut rx = [0];
            spi.transfer(&tx, &mut rx)?;
            token = rx[0];
            timeout -= 1;
        }
        
        if token != DATA_START_TOKEN {
            return Err(SdCardError::CommandTimeout);
        }
        
        // Read 16 bytes of data
        let mut data = [0u8; 16];
        let tx = [0xFF; 16];
        spi.transfer(&tx, &mut data)?;
        
        // Read CRC (2 bytes, ignored)
        let tx_crc = [0xFF, 0xFF];
        let mut rx_crc = [0u8; 2];
        spi.transfer(&tx_crc, &mut rx_crc)?;
        
        Ok(data)
    }
    
    /// Calculate card capacity from CSD register
    fn calculate_capacity(&mut self) {
        let csd = self.info.csd;
        
        match self.info.card_type {
            SdCardType::SdscV1 => {
                // CSD v1.0 structure
                let c_size = (((csd[6] & 0x03) as u32) << 10) | 
                           ((csd[7] as u32) << 2) | 
                           (((csd[8] & 0xC0) as u32) >> 6);
                let c_size_mult = (((csd[9] & 0x03) as u32) << 1) | 
                                (((csd[10] & 0x80) >> 7) as u32);
                let read_bl_len = csd[5] & 0x0F;
                
                let block_len = 1 << read_bl_len;
                let block_count = (c_size + 1) * (1 << (c_size_mult + 2));
                self.info.capacity_sectors = (block_count * block_len) / 512;
            }
            SdCardType::SdscV2 | SdCardType::Sdhc | SdCardType::Sdxc => {
                // CSD v2.0 structure
                let c_size = (((csd[7] & 0x3F) as u32) << 16) | 
                           ((csd[8] as u32) << 8) | 
                           (csd[9] as u32);
                self.info.capacity_sectors = (c_size + 1) * 1024; // 512KB units
            }
            _ => {
                self.info.capacity_sectors = 0;
            }
        }
    }
    
    /// Send clock cycles to SD card
    fn send_clocks<T: SpiInterface>(&self, spi: &mut T, count: usize) -> SdCardResult<()> {
        // Send clock cycles in chunks since we can't use vec in no_std
        let mut tx = [0xFF; 16];
        let mut rx = [0; 16];
        
        let mut remaining = count;
        while remaining > 0 {
            let chunk_size = core::cmp::min(remaining, 16);
            spi.transfer(&tx[..chunk_size], &mut rx[..chunk_size])?;
            remaining -= chunk_size;
        }
        Ok(())
    }
    
    /// Delay in milliseconds
    fn delay_ms(&self, ms: u32) {
        // Simple delay implementation
        for _ in 0..(ms * 1000) {
            core::hint::spin_loop();
        }
    }
    
    /// Get card information
    pub fn get_info(&self) -> &SdCardInfo {
        &self.info
    }
    
    /// Get card state
    pub fn get_state(&self) -> SdCardState {
        self.state
    }
    
    /// Check if card is ready
    pub fn is_ready(&self) -> bool {
        self.state == SdCardState::Ready
    }
}

/// SPI interface trait for SD card communication
pub trait SpiInterface {
    fn transfer(&mut self, tx_data: &[u8], rx_data: &mut [u8]) -> SpiResult<()>;
    fn cs_active(&mut self) -> SpiResult<()>;
    fn cs_inactive(&mut self) -> SpiResult<()>;
}

impl From<SpiError> for SdCardError {
    fn from(_: SpiError) -> Self {
        SdCardError::SpiError
    }
}