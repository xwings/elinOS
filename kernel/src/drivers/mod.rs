// Hardware drivers for elinOS
// Provides low-level hardware abstraction for real hardware support

pub mod spi;
pub mod gpio;
pub mod sdcard;

pub use spi::{SpiController, GpioSpi, SpiConfig, SpiMode, SpiBitOrder, SpiError, SpiResult};
pub use gpio::{GpioController, GpioPin, GpioDirection, GpioState, GpioSpiPins};
pub use sdcard::{SdCardDevice, SdCardError, SdCardResult, init_sdcard_gpio, init_sdcard_hardware};