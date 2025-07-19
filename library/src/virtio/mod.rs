//! VirtIO device drivers for elinOS
//! 
//! This module provides a modular VirtIO implementation with separate
//! components for different device types and shared infrastructure.

// Re-export core types for backward compatibility
pub use error::{DiskError, DiskResult};
pub use queue::{VirtqDesc, VirtqAvail, VirtqUsed, VirtqUsedElem, VirtioQueue};

// Re-export from sub-modules
pub use block::{RustVmmVirtIOBlock, VirtioBlkReq, VIRTIO_BLK};
pub use block::{init_virtio_blk, init_with_address, is_virtio_blk_initialized};
pub use gpu::{VIRTIO_GPU, init_virtio_gpu, flush_display};
pub use storage::{StorageType, init_storage, storage_read_blocks, storage_write_blocks, storage_get_capacity, storage_is_available};

// Modules
pub mod error;
pub mod mmio;
pub mod queue;
pub mod block;
pub mod gpu;
pub mod storage;

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
        
        // VirtIO memory pool - must be page-aligned for legacy VirtIO
        const PAGE_SIZE: usize = 4096;
        
        // Ensure the pool itself is page-aligned
        #[repr(align(4096))]
        struct AlignedPool([u8; 1024 * 1024]);
        
        static mut VIRTIO_MEMORY_POOL: AlignedPool = AlignedPool([0; 1024 * 1024]); // 1MB pool
        static mut VIRTIO_MEMORY_OFFSET: usize = 0;
        
        unsafe {
            // Start from page-aligned pool base
            let pool_base = VIRTIO_MEMORY_POOL.0.as_mut_ptr() as usize;
            
            // Ensure the base is page-aligned (should be due to #[repr(align(4096))])
            if pool_base % PAGE_SIZE != 0 {
                return Err(DiskError::VirtIOError);
            }
            
            // Align current offset to page boundary
            let aligned_offset = (VIRTIO_MEMORY_OFFSET + PAGE_SIZE - 1) & !(PAGE_SIZE - 1);
            
            if aligned_offset + size > VIRTIO_MEMORY_POOL.0.len() {
                return Err(DiskError::VirtIOError);
            }
            
            let ptr = VIRTIO_MEMORY_POOL.0.as_mut_ptr().add(aligned_offset);
            let aligned_addr = ptr as usize;
            
            // Verify the returned address is page-aligned
            if aligned_addr % PAGE_SIZE != 0 {
                return Err(DiskError::VirtIOError);
            }
            
            VIRTIO_MEMORY_OFFSET = aligned_offset + size;
            
            // Zero the memory
            core::ptr::write_bytes(ptr, 0, size);
            
            Ok(aligned_addr)
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
    // In the common library, we assume MMIO regions are identity-mapped
    // The kernel or bootloader should handle the actual memory mapping
    Ok(())
}

/// Unregister a VirtIO device MMIO region
pub fn unregister_virtio_device(base_addr: usize, size: usize) -> Result<(), DiskError> {
    // TODO: Implement device memory unmapping
    // For now, just return success as devices typically aren't unregistered
    Ok(())
} 