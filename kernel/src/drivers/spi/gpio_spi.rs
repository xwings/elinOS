// GPIO-based SPI implementation for elinOS
// Provides SPI communication using GPIO bit-banging when no hardware SPI is available

use super::error::{SpiError, SpiResult};
use super::controller::{SpiConfig, SpiMode};
use crate::drivers::gpio::{GpioController, GpioSpiPins, GpioDirection, GpioState};
use elinos_common::console_println;

/// GPIO-based SPI implementation
pub struct GpioSpi {
    gpio: GpioController,
    pins: GpioSpiPins,
    config: SpiConfig,
    initialized: bool,
}

impl GpioSpi {
    /// Create a new GPIO-based SPI instance
    pub fn new(gpio_base: usize, pins: GpioSpiPins) -> Self {
        GpioSpi {
            gpio: GpioController::new(gpio_base),
            pins,
            config: SpiConfig::default(),
            initialized: false,
        }
    }
    
    /// Initialize GPIO SPI
    pub fn init(&mut self, config: SpiConfig) -> SpiResult<()> {
        self.config = config;
        
        // Initialize GPIO controller
        self.gpio.init()?;
        
        // Configure GPIO pins
        self.gpio.set_direction(self.pins.sclk, GpioDirection::Output)?;
        self.gpio.set_direction(self.pins.mosi, GpioDirection::Output)?;
        self.gpio.set_direction(self.pins.miso, GpioDirection::Input)?;
        self.gpio.set_direction(self.pins.cs, GpioDirection::Output)?;
        
        // Initialize pin states
        self.set_clock_idle()?;
        self.gpio.set_pin(self.pins.mosi, GpioState::Low)?;
        self.cs_inactive()?;
        
        self.initialized = true;
        console_println!("[o] GPIO SPI initialized (bit-banging mode)");
        Ok(())
    }
    
    /// Transfer data via GPIO SPI
    pub fn transfer(&mut self, tx_data: &[u8], rx_data: &mut [u8]) -> SpiResult<()> {
        if !self.initialized {
            return Err(SpiError::NotInitialized);
        }
        
        if tx_data.len() != rx_data.len() {
            return Err(SpiError::BufferTooSmall);
        }
        
        for i in 0..tx_data.len() {
            rx_data[i] = self.transfer_byte(tx_data[i])?;
        }
        
        Ok(())
    }
    
    /// Transfer a single byte via GPIO SPI
    fn transfer_byte(&mut self, tx_byte: u8) -> SpiResult<u8> {
        let mut rx_byte = 0u8;
        
        for bit in 0..8 {
            // Determine bit position based on bit order
            let bit_pos = match self.config.bit_order {
                super::controller::SpiBitOrder::MsbFirst => 7 - bit,
                super::controller::SpiBitOrder::LsbFirst => bit,
            };
            
            // Set MOSI line based on current bit
            let mosi_state = if (tx_byte & (1 << bit_pos)) != 0 {
                GpioState::High
            } else {
                GpioState::Low
            };
            self.gpio.set_pin(self.pins.mosi, mosi_state)?;
            
            // Clock edge 1 (setup)
            self.clock_edge_1()?;
            
            // Sample MISO line
            let miso_state = self.gpio.read_pin(self.pins.miso)?;
            if miso_state == GpioState::High {
                rx_byte |= 1 << bit_pos;
            }
            
            // Clock edge 2 (hold)
            self.clock_edge_2()?;
        }
        
        Ok(rx_byte)
    }
    
    /// First clock edge based on SPI mode
    fn clock_edge_1(&mut self) -> SpiResult<()> {
        match self.config.mode {
            SpiMode::Mode0 | SpiMode::Mode2 => {
                // Clock goes high on first edge
                self.gpio.set_pin(self.pins.sclk, GpioState::High)?;
            }
            SpiMode::Mode1 | SpiMode::Mode3 => {
                // Clock goes low on first edge
                self.gpio.set_pin(self.pins.sclk, GpioState::Low)?;
            }
        }
        
        self.delay_half_clock();
        Ok(())
    }
    
    /// Second clock edge based on SPI mode
    fn clock_edge_2(&mut self) -> SpiResult<()> {
        match self.config.mode {
            SpiMode::Mode0 | SpiMode::Mode2 => {
                // Clock goes low on second edge
                self.gpio.set_pin(self.pins.sclk, GpioState::Low)?;
            }
            SpiMode::Mode1 | SpiMode::Mode3 => {
                // Clock goes high on second edge
                self.gpio.set_pin(self.pins.sclk, GpioState::High)?;
            }
        }
        
        self.delay_half_clock();
        Ok(())
    }
    
    /// Set clock to idle state based on SPI mode
    fn set_clock_idle(&mut self) -> SpiResult<()> {
        match self.config.mode {
            SpiMode::Mode0 | SpiMode::Mode1 => {
                // Clock idle low (CPOL=0)
                self.gpio.set_pin(self.pins.sclk, GpioState::Low)?;
            }
            SpiMode::Mode2 | SpiMode::Mode3 => {
                // Clock idle high (CPOL=1)
                self.gpio.set_pin(self.pins.sclk, GpioState::High)?;
            }
        }
        Ok(())
    }
    
    /// Delay for half clock period
    fn delay_half_clock(&self) {
        // Calculate delay based on clock rate
        let delay_cycles = if self.config.clock_rate > 0 {
            // Rough approximation: assume 1GHz CPU clock
            1_000_000_000 / (self.config.clock_rate * 2)
        } else {
            1000 // Default delay
        };
        
        // Simple delay loop
        for _ in 0..delay_cycles {
            core::hint::spin_loop();
        }
    }
    
    /// Set chip select active
    pub fn cs_active(&mut self) -> SpiResult<()> {
        if !self.initialized {
            return Err(SpiError::NotInitialized);
        }
        
        let cs_state = if self.config.cs_polarity {
            GpioState::High
        } else {
            GpioState::Low
        };
        
        self.gpio.set_pin(self.pins.cs, cs_state)?;
        Ok(())
    }
    
    /// Set chip select inactive
    pub fn cs_inactive(&mut self) -> SpiResult<()> {
        if !self.initialized {
            return Err(SpiError::NotInitialized);
        }
        
        let cs_state = if self.config.cs_polarity {
            GpioState::Low
        } else {
            GpioState::High
        };
        
        self.gpio.set_pin(self.pins.cs, cs_state)?;
        Ok(())
    }
}