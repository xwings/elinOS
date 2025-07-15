// SPI error types for elinOS

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SpiError {
    NotInitialized,
    TransferTimeout,
    BusError,
    InvalidPin,
    InvalidClockRate,
    DeviceNotFound,
    TransferFailed,
    BufferTooSmall,
}

pub type SpiResult<T> = Result<T, SpiError>;