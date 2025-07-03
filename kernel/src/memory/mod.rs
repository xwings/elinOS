// Memory Management Module for elinOS  
// Now uses unified memory manager from shared library
// Keeps only kernel-specific memory management features

pub mod mmu;
pub mod mapping;

// Re-export the unified memory management from shared library
pub use elinos_common::memory::*;

// Keep only kernel-specific global allocator compatibility
use linked_list_allocator::LockedHeap;
use core::alloc::{GlobalAlloc, Layout};

// Simple heap allocator for kernel (fallback) - will be replaced by unified manager
#[global_allocator]
pub static ALLOCATOR: LockedHeap = LockedHeap::empty();

/// Custom global allocator that uses the unified memory manager
pub struct UnifiedGlobalAllocator;

unsafe impl GlobalAlloc for UnifiedGlobalAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        match allocate_memory(layout.size(), layout.align()) {
            Ok(ptr) => ptr.as_ptr(),
            Err(_) => core::ptr::null_mut(),
        }
    }
    
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        if let Some(non_null_ptr) = core::ptr::NonNull::new(ptr) {
            deallocate_memory(non_null_ptr, layout.size());
        }
    }
}

/// Initialize memory allocator compatibility layer
pub fn init_allocator_compatibility() {
    // Initialize the old global allocator with a small region for compatibility
    // Most allocations will go through the unified manager
    let heap_start = 0x80500000usize; // Small compatibility heap
    let heap_size = 64 * 1024;   // 64KB for compatibility
    
    unsafe {
        ALLOCATOR.lock().init(heap_start as *mut u8, heap_size);
    }
    
    elinos_common::console_println!("[o] Memory allocator compatibility layer initialized");
}

/// Kernel-specific memory functions that use the unified manager

/// Allocate kernel memory with alignment
pub fn allocate_kernel_memory(size: usize, align: usize) -> Option<usize> {
    match allocate_memory(size, align) {
        Ok(ptr) => Some(ptr.as_ptr() as usize),
        Err(_) => None,
    }
}

/// Deallocate kernel memory
pub fn deallocate_kernel_memory(addr: usize, size: usize) {
    if let Some(ptr) = core::ptr::NonNull::new(addr as *mut u8) {
        deallocate_memory(ptr, size);
    }
}

/// Check if a memory range is available for kernel use
pub fn is_kernel_range_available(addr: usize, size: usize) -> bool {
    is_memory_range_free(addr, size)
}

/// Get kernel memory statistics
pub fn get_kernel_memory_stats() -> (usize, usize) {
    let free = get_total_free_memory();
    // Total memory can be obtained from the unified manager
    (free, 0) // TODO: Get total from unified manager
}

/// Display kernel memory information
pub fn display_kernel_memory_info() {
    elinos_common::console_println!("[i] Kernel Memory Information:");
    display_memory_layout();
}