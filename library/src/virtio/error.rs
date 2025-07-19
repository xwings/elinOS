//! VirtIO error types and result definitions

use crate::console_println;
use core::fmt;

/// VirtIO disk operation errors
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiskError {
    NotFound,
    InvalidSector,
    BufferTooSmall,
    ReadError,
    WriteError,
    DeviceNotFound,
    NotInitialized,
    VirtIOError,
    InvalidParameter,
    QueueFull,
    IoError,
    InvalidDescriptor,
    DeviceNotReady,
}

impl fmt::Display for DiskError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // Ensure this message always gets out if a DiskError is processed
        console_println!("!! DiskError Formatted: {:?}", self);
        match self {
            DiskError::NotFound => write!(f, "Disk not found"),
            DiskError::InvalidSector => write!(f, "Invalid sector number"),
            DiskError::BufferTooSmall => write!(f, "Buffer too small"),
            DiskError::ReadError => write!(f, "Disk read error"),
            DiskError::WriteError => write!(f, "Disk write error"),
            DiskError::DeviceNotFound => write!(f, "Disk device not found"),
            DiskError::NotInitialized => write!(f, "Disk not initialized"),
            DiskError::VirtIOError => write!(f, "VirtIO error"),
            DiskError::IoError => write!(f, "I/O error"),
            DiskError::QueueFull => write!(f, "VirtIO queue full"),
            DiskError::InvalidDescriptor => write!(f, "Invalid descriptor"),
            DiskError::DeviceNotReady => write!(f, "Device not ready"),
            DiskError::InvalidParameter => write!(f, "Invalid parameter"),
        }
    }
}

pub type DiskResult<T> = Result<T, DiskError>; 