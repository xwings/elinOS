// SPI (Serial Peripheral Interface) driver for elinOS
// Supports both hardware SPI controllers and GPIO bit-banging

pub mod controller;
pub mod gpio_spi;
pub mod error;

pub use controller::*;
pub use gpio_spi::*;
pub use error::*;