//! VirtIO device drivers for elinOS
//! 
//! This module provides a modular VirtIO implementation with separate
//! components for different device types and shared infrastructure.

// Re-export core types for backward compatibility
pub use error::{DiskError, DiskResult};
pub use queue::{VirtqDesc, VirtqAvail, VirtqUsed, VirtqUsedElem, VirtioQueue};

// Re-export from sub-modules
pub use block::{RustVmmVirtIOBlock, VirtioBlkReq, VIRTIO_BLK};
pub use block::{init_virtio_blk, init_with_address};
pub use gpu::{VIRTIO_GPU, init_virtio_gpu, flush_display};

// Modules
pub mod error;
pub mod mmio;
pub mod queue;
pub mod block;
pub mod gpu;

use spin::Mutex;

/// VirtIO Device Memory Manager
/// Provides DMA-safe memory allocation for VirtIO devices
pub struct VirtioMemoryManager {
    initialized: bool,
}

impl VirtioMemoryManager {
    pub const fn new() -> Self {
        VirtioMemoryManager {
            initialized: false,
        }
    }
    
    pub fn init(&mut self) -> Result<(), DiskError> {
        self.initialized = true;
        Ok(())
    }
    
    /// Allocate DMA-safe memory for VirtIO queue operations
    pub fn allocate_queue_memory(&self, size: usize) -> Result<usize, DiskError> {
        if !self.initialized {
            return Err(DiskError::NotInitialized);
        }
        
        // Use the new memory mapping API to allocate DMA buffer
        match crate::memory::mapping::map_virtual_memory(
            size,
            crate::memory::mapping::MemoryPermissions::READ_WRITE,
            "VirtIO-Queue"
        ) {
            Ok(addr) => Ok(addr),
            Err(_) => Err(DiskError::VirtIOError),
        }
    }
}

// Global VirtIO memory manager
static VIRTIO_MEMORY: Mutex<VirtioMemoryManager> = Mutex::new(VirtioMemoryManager::new());

/// Initialize VirtIO memory management
pub fn init_virtio_memory() -> Result<(), DiskError> {
    let mut memory_mgr = VIRTIO_MEMORY.lock();
    memory_mgr.init()
}

/// Allocate memory for VirtIO operations
pub fn allocate_virtio_memory(size: usize) -> Result<usize, DiskError> {
    let memory_mgr = VIRTIO_MEMORY.lock();
    memory_mgr.allocate_queue_memory(size)
}

/// Register a VirtIO device MMIO region
pub fn register_virtio_device(base_addr: usize, size: usize, device_name: &str) -> Result<(), DiskError> {
    match crate::memory::mapping::map_device_memory(base_addr, size, device_name) {
        Ok(_) => Ok(()),
        Err(_) => Err(DiskError::VirtIOError),
    }
}

/// Unregister a VirtIO device MMIO region
pub fn unregister_virtio_device(base_addr: usize, size: usize) -> Result<(), DiskError> {
    // TODO: Implement device memory unmapping
    // For now, just return success as devices typically aren't unregistered
    Ok(())
} 