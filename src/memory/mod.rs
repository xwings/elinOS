// Memory Management Module for elinOS
// Simple heap-based memory management

pub mod layout;

use core::ptr;
use core::mem;
use spin::Mutex;
use crate::{sbi, console_println};
use linked_list_allocator::LockedHeap;

// === MEMORY MANAGER ===

// Enhanced memory region structure
#[derive(Debug, Clone, Copy)]
pub struct MemoryRegion {
    pub start: usize,
    pub size: usize,
    pub is_ram: bool,
    pub zone_type: MemoryZone,
}

// Memory zones similar to Linux
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MemoryZone {
    DMA,        // Direct Memory Access zone (first 16MB)
    Normal,     // Normal memory zone
    High,       // High memory zone (if applicable)
}

// Simple heap allocator for kernel
#[global_allocator]
static ALLOCATOR: LockedHeap = LockedHeap::empty();

// Heap configuration 
const HEAP_SIZE: usize = 64 * 1024; // 64KB heap
static mut HEAP_SPACE: [u8; HEAP_SIZE] = [0; HEAP_SIZE];

// Simplified Memory Manager (heap-only)
pub struct MemoryManager {
    // Memory region information
    regions: [MemoryRegion; 8],
    region_count: usize,
    
    // Memory statistics
    allocated_bytes: usize,
    allocation_count: usize,
    
    // Heap management
    heap_start: usize,
    heap_size: usize,
    heap_used: usize,
}

impl MemoryManager {
    pub const fn new() -> Self {
        MemoryManager {
            // Memory regions
            regions: [MemoryRegion { 
                start: 0, 
                size: 0, 
                is_ram: false, 
                zone_type: MemoryZone::Normal 
            }; 8],
            region_count: 0,
            
            // Statistics
            allocated_bytes: 0,
            allocation_count: 0,
            
            // Heap
            heap_start: 0,
            heap_size: 0,
            heap_used: 0,
        }
    }

    /// Initialize memory regions and heap allocator
    pub fn init(&mut self) {
        self.detect_memory_layout();
        
        console_println!("🧠 Memory Manager Initialization (heap-only)");
        console_println!("Memory layout detection complete");
        
        if self.region_count > 0 {
            let total_ram = self.regions[..self.region_count].iter()
                .filter(|r| r.is_ram)
                .map(|r| r.size)
                .sum::<usize>();
            
            console_println!("Total RAM detected: {} MB", total_ram / (1024 * 1024));
        } else {
            console_println!("⚠️  No memory regions detected, using defaults");
        }
        
        // Initialize heap allocator
        self.init_heap();
        console_println!("✅ Heap allocator ready");
        console_println!("🎉 Memory manager ready!");
    }
    
    /// Detect memory layout using SBI
    fn detect_memory_layout(&mut self) {
        console_println!("🔍 Detecting memory layout...");
        
        // Get memory regions from SBI
        let sbi_regions = sbi::get_memory_regions();
        
        // Convert SBI regions to our enhanced format
        self.region_count = sbi_regions.count;
        for i in 0..self.region_count {
            let sbi_region = &sbi_regions.regions[i];
            
            // Determine memory zone based on address
            let zone_type = if sbi_region.start < 16 * 1024 * 1024 {
                MemoryZone::DMA
            } else {
                MemoryZone::Normal
            };
            
            self.regions[i] = MemoryRegion {
                start: sbi_region.start,
                size: sbi_region.size,
                is_ram: (sbi_region.flags & 1) != 0,
                zone_type,
            };
            
            console_println!("Region {}: 0x{:x} - 0x{:x} ({} MB) {} {:?}",
                i,
                sbi_region.start,
                sbi_region.start + sbi_region.size,
                sbi_region.size / (1024 * 1024),
                if (sbi_region.flags & 1) != 0 { "RAM" } else { "MMIO" },
                zone_type
            );
        }
    }

    fn init_heap(&mut self) {
        console_println!("Initializing heap allocator...");
        
        unsafe {
            let heap_start = HEAP_SPACE.as_mut_ptr() as usize;
            let heap_size = HEAP_SIZE;
            
            self.heap_start = heap_start;
            self.heap_size = heap_size;
            self.heap_used = 0;
            
            console_println!("Heap: 0x{:x} - 0x{:x} ({} KB)", 
                heap_start, 
                heap_start + heap_size,
                heap_size / 1024
            );
            
            ALLOCATOR.lock().init(HEAP_SPACE.as_mut_ptr(), heap_size);
        }
    }

    pub fn show_stats(&self) {
        let stats = self.get_stats();
        console_println!("=== Memory Manager Statistics ===");
        console_println!("Total Memory: {} MB", stats.total_memory / (1024 * 1024));
        console_println!("Allocated: {} bytes", stats.allocated_bytes);
        console_println!("Allocations: {}", stats.allocation_count);
        
        console_println!("Memory Regions:");
        for (i, region) in self.get_memory_info().iter().enumerate() {
            console_println!("  Region {}: 0x{:x} - 0x{:x} ({} MB) {} {:?}",
                i,
                region.start,
                region.start + region.size,
                region.size / (1024 * 1024),
                if region.is_ram { "RAM" } else { "MMIO" },
                region.zone_type
            );
        }
    }

    /// Get comprehensive memory usage statistics
    pub fn get_stats(&self) -> MemoryStats {
        let total_memory = self.regions[..self.region_count].iter()
            .filter(|r| r.is_ram)
            .map(|r| r.size)
            .sum();
        
        MemoryStats {
            total_memory,
            allocated_bytes: self.allocated_bytes,
            allocation_count: self.allocation_count,
        }
    }

    /// Get memory region information
    pub fn get_memory_info(&self) -> &[MemoryRegion] {
        &self.regions[..self.region_count]
    }
}

/// Memory usage statistics
#[derive(Debug)]
pub struct MemoryStats {
    pub total_memory: usize,
    pub allocated_bytes: usize,
    pub allocation_count: usize,
}

// Global memory manager instance (unified)
pub static MEMORY_MANAGER: Mutex<MemoryManager> = Mutex::new(MemoryManager::new());

// Helper functions for easy access
pub fn allocate_memory(size: usize) -> Option<usize> {
    let mut manager = MEMORY_MANAGER.lock();
    manager.allocation_count += 1;
    
    unsafe {
        let layout = core::alloc::Layout::from_size_align(size, 8).ok()?;
        let ptr = core::alloc::GlobalAlloc::alloc(&ALLOCATOR, layout);
        
        if !ptr.is_null() {
            manager.allocated_bytes += size;
            let addr = ptr as usize;
            console_println!("🔍 Small allocation: {} bytes at 0x{:x}", size, addr);
            Some(addr)
        } else {
            console_println!("❌ Allocation failed: {} bytes", size);
            None
        }
    }
}

pub fn deallocate_memory(addr: usize, size: usize) {
    let mut manager = MEMORY_MANAGER.lock();
    
    unsafe {
        if let Ok(layout) = core::alloc::Layout::from_size_align(size, 8) {
            core::alloc::GlobalAlloc::dealloc(&ALLOCATOR, addr as *mut u8, layout);
            manager.allocated_bytes = manager.allocated_bytes.saturating_sub(size);
            console_println!("🔍 Small deallocation: {} bytes at 0x{:x}", size, addr);
        }
    }
}

pub fn get_memory_stats() -> MemoryStats {
    MEMORY_MANAGER.lock().get_stats()
} 