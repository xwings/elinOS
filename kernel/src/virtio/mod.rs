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

// Modules
pub mod error;
pub mod mmio;
pub mod queue;
pub mod block;

use spin::Mutex;
use crate::memory::layout::get_memory_layout;

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
        
        // Get mutable access to memory layout
        // Note: This is a simplified approach - in a real system, you'd want
        // a more sophisticated memory manager
        let layout = get_memory_layout();
        
        // For now, we'll use the hardcoded approach but this shows the framework
        // for using the device memory region
        let (device_start, device_size, device_used) = layout.get_device_memory_stats();
        
        // Check if we have enough device memory available
        if device_used + size > device_size {
            return Err(DiskError::VirtIOError);
        }
        
        // For now, return a safe address in the device memory region
        // In the future, this would call layout.allocate_device_memory()
        let page_size = 4096;
        let aligned_addr = (device_start + page_size - 1) & !(page_size - 1);
        
        Ok(aligned_addr)
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