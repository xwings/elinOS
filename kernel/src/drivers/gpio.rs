// GPIO driver for elinOS
// Provides GPIO pin control for bit-banging protocols like SPI

use super::spi::error::{SpiError, SpiResult};
use elinos_common::console_println;

/// GPIO pin configuration
#[derive(Debug, Clone, Copy)]
pub struct GpioPin {
    pub port: u8,
    pub pin: u8,
}

impl GpioPin {
    pub fn new(port: u8, pin: u8) -> Self {
        GpioPin { port, pin }
    }
}

/// GPIO pin direction
#[derive(Debug, Clone, Copy)]
pub enum GpioDirection {
    Input,
    Output,
}

/// GPIO pin state
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GpioState {
    Low,
    High,
}

/// GPIO controller for managing GPIO pins
pub struct GpioController {
    base_addr: usize,
    initialized: bool,
}

impl GpioController {
    /// Create a new GPIO controller
    pub fn new(base_addr: usize) -> Self {
        GpioController {
            base_addr,
            initialized: false,
        }
    }
    
    /// Initialize GPIO controller
    pub fn init(&mut self) -> SpiResult<()> {
        // Try to detect GPIO controller
        if !self.detect_hardware() {
            return Err(SpiError::DeviceNotFound);
        }
        
        self.initialized = true;
        console_println!("[o] GPIO controller initialized at 0x{:x}", self.base_addr);
        Ok(())
    }
    
    /// Detect GPIO controller hardware
    fn detect_hardware(&self) -> bool {
        // This would check for GPIO controller presence via device tree
        // For now, assume GPIO is available (most RISC-V boards have GPIO)
        true
    }
    
    /// Configure GPIO pin direction
    pub fn set_direction(&mut self, pin: GpioPin, direction: GpioDirection) -> SpiResult<()> {
        if !self.initialized {
            return Err(SpiError::NotInitialized);
        }
        
        let port_offset = (pin.port as usize) * GPIO_PORT_SIZE;
        let current_dir = self.read_reg(port_offset + GPIO_DIR_REG);
        
        let new_dir = match direction {
            GpioDirection::Input => current_dir & !(1 << pin.pin),
            GpioDirection::Output => current_dir | (1 << pin.pin),
        };
        
        self.write_reg(port_offset + GPIO_DIR_REG, new_dir);
        Ok(())
    }
    
    /// Set GPIO pin state (for output pins)
    pub fn set_pin(&mut self, pin: GpioPin, state: GpioState) -> SpiResult<()> {
        if !self.initialized {
            return Err(SpiError::NotInitialized);
        }
        
        let port_offset = (pin.port as usize) * GPIO_PORT_SIZE;
        let current_output = self.read_reg(port_offset + GPIO_OUT_REG);
        
        let new_output = match state {
            GpioState::Low => current_output & !(1 << pin.pin),
            GpioState::High => current_output | (1 << pin.pin),
        };
        
        self.write_reg(port_offset + GPIO_OUT_REG, new_output);
        Ok(())
    }
    
    /// Read GPIO pin state
    pub fn read_pin(&self, pin: GpioPin) -> SpiResult<GpioState> {
        if !self.initialized {
            return Err(SpiError::NotInitialized);
        }
        
        let port_offset = (pin.port as usize) * GPIO_PORT_SIZE;
        let input_reg = self.read_reg(port_offset + GPIO_IN_REG);
        
        if (input_reg & (1 << pin.pin)) != 0 {
            Ok(GpioState::High)
        } else {
            Ok(GpioState::Low)
        }
    }
    
    /// Read from GPIO controller register
    fn read_reg(&self, offset: usize) -> u32 {
        unsafe {
            core::ptr::read_volatile((self.base_addr + offset) as *const u32)
        }
    }
    
    /// Write to GPIO controller register
    fn write_reg(&self, offset: usize, value: u32) {
        unsafe {
            core::ptr::write_volatile((self.base_addr + offset) as *mut u32, value);
        }
    }
}

// GPIO register offsets (example layout)
const GPIO_DIR_REG: usize = 0x04;    // Direction register
const GPIO_OUT_REG: usize = 0x08;    // Output register
const GPIO_IN_REG: usize = 0x0C;     // Input register
const GPIO_PORT_SIZE: usize = 0x10;  // Size of each GPIO port

/// GPIO-based SPI configuration
pub struct GpioSpiPins {
    pub sclk: GpioPin,  // SPI clock
    pub mosi: GpioPin,  // Master out, slave in
    pub miso: GpioPin,  // Master in, slave out
    pub cs: GpioPin,    // Chip select
}

impl GpioSpiPins {
    pub fn new(sclk: GpioPin, mosi: GpioPin, miso: GpioPin, cs: GpioPin) -> Self {
        GpioSpiPins { sclk, mosi, miso, cs }
    }
}