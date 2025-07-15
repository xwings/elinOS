// Hardware SPI controller driver for elinOS
// Provides interface to hardware SPI peripherals found on real RISC-V boards

use super::error::{SpiError, SpiResult};
use elinos_common::console_println;

/// SPI controller configuration
#[derive(Debug, Clone, Copy)]
pub struct SpiConfig {
    pub clock_rate: u32,        // SPI clock frequency in Hz
    pub mode: SpiMode,          // SPI mode (0-3)
    pub bit_order: SpiBitOrder, // MSB or LSB first
    pub cs_polarity: bool,      // Chip select polarity (true = active high)
}

impl Default for SpiConfig {
    fn default() -> Self {
        SpiConfig {
            clock_rate: 400_000,    // 400kHz - safe for SD card initialization
            mode: SpiMode::Mode0,
            bit_order: SpiBitOrder::MsbFirst,
            cs_polarity: false,     // Active low CS
        }
    }
}

/// SPI mode configuration
#[derive(Debug, Clone, Copy)]
pub enum SpiMode {
    Mode0,  // CPOL=0, CPHA=0
    Mode1,  // CPOL=0, CPHA=1
    Mode2,  // CPOL=1, CPHA=0
    Mode3,  // CPOL=1, CPHA=1
}

/// SPI bit order
#[derive(Debug, Clone, Copy)]
pub enum SpiBitOrder {
    MsbFirst,
    LsbFirst,
}

/// Hardware SPI controller abstraction
pub struct SpiController {
    base_addr: usize,
    config: SpiConfig,
    initialized: bool,
}

impl SpiController {
    /// Create a new SPI controller instance
    pub fn new(base_addr: usize) -> Self {
        SpiController {
            base_addr,
            config: SpiConfig::default(),
            initialized: false,
        }
    }
    
    /// Initialize the SPI controller with configuration
    pub fn init(&mut self, config: SpiConfig) -> SpiResult<()> {
        self.config = config;
        
        // Try to detect hardware SPI controller
        if !self.detect_hardware() {
            return Err(SpiError::DeviceNotFound);
        }
        
        // Configure SPI controller registers
        self.configure_hardware()?;
        
        self.initialized = true;
        console_println!("[o] SPI controller initialized at 0x{:x}", self.base_addr);
        Ok(())
    }
    
    /// Detect if hardware SPI controller is present
    fn detect_hardware(&self) -> bool {
        // This would check for SPI controller presence via device tree or direct probing
        // For now, assume no hardware SPI controller (will fall back to GPIO)
        false
    }
    
    /// Configure SPI controller hardware registers
    fn configure_hardware(&mut self) -> SpiResult<()> {
        // Hardware-specific SPI controller configuration
        // This would be implemented based on the specific RISC-V SoC's SPI controller
        
        // Example register layout (varies by SoC):
        // - CTRL register: Enable, mode, bit order
        // - CLOCK register: Clock divider
        // - STATUS register: Transfer status
        // - DATA register: TX/RX data
        
        Ok(())
    }
    
    /// Transfer data via SPI
    pub fn transfer(&mut self, tx_data: &[u8], rx_data: &mut [u8]) -> SpiResult<()> {
        if !self.initialized {
            return Err(SpiError::NotInitialized);
        }
        
        if tx_data.len() != rx_data.len() {
            return Err(SpiError::BufferTooSmall);
        }
        
        // Hardware SPI transfer implementation
        for i in 0..tx_data.len() {
            rx_data[i] = self.transfer_byte(tx_data[i])?;
        }
        
        Ok(())
    }
    
    /// Transfer a single byte
    fn transfer_byte(&mut self, tx_byte: u8) -> SpiResult<u8> {
        // Write to TX register
        self.write_reg(SPI_DATA_REG, tx_byte as u32);
        
        // Wait for transfer complete
        let mut timeout = 10000;
        while timeout > 0 {
            let status = self.read_reg(SPI_STATUS_REG);
            if (status & SPI_STATUS_TRANSFER_COMPLETE) != 0 {
                // Read from RX register
                return Ok(self.read_reg(SPI_DATA_REG) as u8);
            }
            timeout -= 1;
            core::hint::spin_loop();
        }
        
        Err(SpiError::TransferTimeout)
    }
    
    /// Set chip select active
    pub fn cs_active(&mut self) -> SpiResult<()> {
        if !self.initialized {
            return Err(SpiError::NotInitialized);
        }
        
        // Set CS pin according to polarity
        let cs_value = if self.config.cs_polarity { 1 } else { 0 };
        let ctrl = self.read_reg(SPI_CTRL_REG);
        self.write_reg(SPI_CTRL_REG, (ctrl & !SPI_CTRL_CS_MASK) | (cs_value << SPI_CTRL_CS_SHIFT));
        
        Ok(())
    }
    
    /// Set chip select inactive
    pub fn cs_inactive(&mut self) -> SpiResult<()> {
        if !self.initialized {
            return Err(SpiError::NotInitialized);
        }
        
        // Set CS pin according to polarity
        let cs_value = if self.config.cs_polarity { 0 } else { 1 };
        let ctrl = self.read_reg(SPI_CTRL_REG);
        self.write_reg(SPI_CTRL_REG, (ctrl & !SPI_CTRL_CS_MASK) | (cs_value << SPI_CTRL_CS_SHIFT));
        
        Ok(())
    }
    
    /// Read from SPI controller register
    fn read_reg(&self, offset: usize) -> u32 {
        unsafe {
            core::ptr::read_volatile((self.base_addr + offset) as *const u32)
        }
    }
    
    /// Write to SPI controller register
    fn write_reg(&self, offset: usize, value: u32) {
        unsafe {
            core::ptr::write_volatile((self.base_addr + offset) as *mut u32, value);
        }
    }
}

// Example SPI controller register offsets (varies by SoC)
const SPI_CTRL_REG: usize = 0x00;
const SPI_STATUS_REG: usize = 0x04;
const SPI_DATA_REG: usize = 0x08;
const SPI_CLOCK_REG: usize = 0x0C;

// Control register bits
const SPI_CTRL_ENABLE: u32 = 1 << 0;
const SPI_CTRL_MODE_MASK: u32 = 0x3 << 1;
const SPI_CTRL_MODE_SHIFT: u32 = 1;
const SPI_CTRL_CS_MASK: u32 = 0x1 << 3;
const SPI_CTRL_CS_SHIFT: u32 = 3;

// Status register bits
const SPI_STATUS_TRANSFER_COMPLETE: u32 = 1 << 0;
const SPI_STATUS_TX_READY: u32 = 1 << 1;
const SPI_STATUS_RX_READY: u32 = 1 << 2;