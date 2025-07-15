// Storage abstraction layer for elinOS
// Provides unified interface for VirtIO block devices and SD cards

use super::error::{DiskError, DiskResult};
use super::block::VIRTIO_BLK;
use crate::drivers::sdcard::{read_sdcard_blocks, write_sdcard_blocks, get_sdcard_capacity};
use elinos_common::console_println;
use spin::Mutex;

/// Storage device type
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum StorageType {
    VirtIO,
    SdCard,
}

/// Unified storage interface
pub struct StorageManager {
    active_device: Option<StorageType>,
    virtio_available: bool,
    sdcard_available: bool,
}

impl StorageManager {
    pub const fn new() -> Self {
        StorageManager {
            active_device: None,
            virtio_available: false,
            sdcard_available: false,
        }
    }
    
    /// Initialize storage manager and detect available devices
    pub fn init(&mut self) -> DiskResult<()> {
        // Check VirtIO block device
        {
            let virtio_blk = VIRTIO_BLK.lock();
            if virtio_blk.is_initialized() {
                self.virtio_available = true;
                console_println!("[o] VirtIO block device detected");
            }
        }
        
        // Check SD card
        let sdcard_capacity = get_sdcard_capacity();
        if sdcard_capacity > 0 {
            self.sdcard_available = true;
            console_println!("[o] SD card detected (capacity: {} sectors)", sdcard_capacity);
        }
        
        // Set priority: VirtIO first (for QEMU), then SD card (for real hardware)
        if self.virtio_available {
            self.active_device = Some(StorageType::VirtIO);
            console_println!("[o] Using VirtIO block device as primary storage");
        } else if self.sdcard_available {
            self.active_device = Some(StorageType::SdCard);
            console_println!("[o] Using SD card as primary storage");
        } else {
            console_println!("[!] No storage devices available");
            return Err(DiskError::DeviceNotFound);
        }
        
        Ok(())
    }
    
    /// Read blocks from active storage device
    pub fn read_blocks(&self, start_block: u32, buffer: &mut [u8]) -> DiskResult<()> {
        match self.active_device {
            Some(StorageType::VirtIO) => {
                let mut virtio_blk = VIRTIO_BLK.lock();
                virtio_blk.read_blocks(start_block as u64, buffer)
            }
            Some(StorageType::SdCard) => {
                read_sdcard_blocks(start_block, buffer)
                    .map_err(|_| DiskError::ReadError)
            }
            None => Err(DiskError::DeviceNotFound),
        }
    }
    
    /// Write blocks to active storage device
    pub fn write_blocks(&self, start_block: u32, buffer: &[u8]) -> DiskResult<()> {
        match self.active_device {
            Some(StorageType::VirtIO) => {
                let mut virtio_blk = VIRTIO_BLK.lock();
                virtio_blk.write_blocks(start_block as u64, buffer)
            }
            Some(StorageType::SdCard) => {
                write_sdcard_blocks(start_block, buffer)
                    .map_err(|_| DiskError::WriteError)
            }
            None => Err(DiskError::DeviceNotFound),
        }
    }
    
    /// Get capacity of active storage device
    pub fn get_capacity(&self) -> u64 {
        match self.active_device {
            Some(StorageType::VirtIO) => {
                let virtio_blk = VIRTIO_BLK.lock();
                virtio_blk.get_capacity()
            }
            Some(StorageType::SdCard) => {
                get_sdcard_capacity() as u64
            }
            None => 0,
        }
    }
    
    /// Get active storage device type
    pub fn get_active_device(&self) -> Option<StorageType> {
        self.active_device
    }
    
    /// Check if storage is available
    pub fn is_available(&self) -> bool {
        self.active_device.is_some()
    }
    
    /// Switch to different storage device
    pub fn switch_device(&mut self, device_type: StorageType) -> DiskResult<()> {
        match device_type {
            StorageType::VirtIO => {
                if self.virtio_available {
                    self.active_device = Some(StorageType::VirtIO);
                    console_println!("[o] Switched to VirtIO block device");
                    Ok(())
                } else {
                    Err(DiskError::DeviceNotFound)
                }
            }
            StorageType::SdCard => {
                if self.sdcard_available {
                    self.active_device = Some(StorageType::SdCard);
                    console_println!("[o] Switched to SD card");
                    Ok(())
                } else {
                    Err(DiskError::DeviceNotFound)
                }
            }
        }
    }
}

// Global storage manager
static STORAGE_MANAGER: Mutex<StorageManager> = Mutex::new(StorageManager::new());

/// Initialize storage manager
pub fn init_storage() -> DiskResult<()> {
    let mut storage = STORAGE_MANAGER.lock();
    storage.init()
}

/// Read blocks from storage
pub fn storage_read_blocks(start_block: u32, buffer: &mut [u8]) -> DiskResult<()> {
    let storage = STORAGE_MANAGER.lock();
    storage.read_blocks(start_block, buffer)
}

/// Write blocks to storage
pub fn storage_write_blocks(start_block: u32, buffer: &[u8]) -> DiskResult<()> {
    let storage = STORAGE_MANAGER.lock();
    storage.write_blocks(start_block, buffer)
}

/// Get storage capacity
pub fn storage_get_capacity() -> u64 {
    let storage = STORAGE_MANAGER.lock();
    storage.get_capacity()
}

/// Get active storage device type
pub fn storage_get_active_device() -> Option<StorageType> {
    let storage = STORAGE_MANAGER.lock();
    storage.get_active_device()
}

/// Check if storage is available
pub fn storage_is_available() -> bool {
    let storage = STORAGE_MANAGER.lock();
    storage.is_available()
}