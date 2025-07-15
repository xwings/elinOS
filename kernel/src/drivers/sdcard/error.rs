// SD card error types for elinOS

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SdCardError {
    NotInitialized,
    InitializationFailed,
    CommandTimeout,
    CommandError,
    InvalidResponse,
    CrcError,
    ReadError,
    WriteError,
    CardNotSupported,
    SpiError,
    InvalidSector,
    WriteProtected,
    CardNotPresent,
}

pub type SdCardResult<T> = Result<T, SdCardError>;

// Note: From<SpiError> is implemented at the end of protocol.rs to avoid conflicts