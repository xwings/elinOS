// Memory Management Module for elinOS
// Implements Maestro-inspired design with MIT-licensed buddy allocator

pub mod buddy;
pub mod small_alloc;
pub mod vmm;
pub mod layout;

use core::fmt::Write;
use core::option::Option::{self, Some, None};
use core::writeln;
use spin::Mutex;
use crate::UART;
use crate::sbi;

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

// Advanced Memory Manager with hybrid allocation engine
pub struct MemoryManager {
    // Memory region information
    regions: [MemoryRegion; 8],
    region_count: usize,
    
    // Advanced allocator components
    buddy_allocator: Option<buddy::BuddyAllocator>,
    small_allocator: Option<small_alloc::SmallAllocator>,
    
    // Memory statistics
    allocated_bytes: usize,
    free_bytes: usize,
    allocation_count: usize,
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
            
            // Advanced allocators
            buddy_allocator: None,
            small_allocator: None,
            
            // Statistics
            allocated_bytes: 0,
            free_bytes: 0,
            allocation_count: 0,
        }
    }

    /// Initialize memory regions and all allocators
    pub fn init(&mut self) {
        {
            let mut uart = UART.lock();
            let _ = writeln!(uart, "🚀 Initializing elinOS Memory Management System...");
        }
        
        // Initialize memory layout detection
        self.init_memory_regions();
        
        // Initialize all allocators  
        self.init_buddy_allocator();
        self.init_small_allocator();
        
        {
            let mut uart = UART.lock();
            let _ = writeln!(uart, "✅ Memory management system ready!");
            let _ = writeln!(uart, "   🚀 Hybrid Allocation Engine: Small → Buddy");
        }
    }
    
    /// Initialize memory regions using dynamic layout
    fn init_memory_regions(&mut self) {
        {
            let mut uart = UART.lock();
            let _ = writeln!(uart, "🔍 Detecting memory layout...");
        }
        
        // Get dynamic memory layout (SBI issue now fixed!)
        let memory_layout = layout::get_memory_layout();
        
        // Display the dynamic layout
        memory_layout.display();
        
        // Get memory regions from SBI (now working properly)
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
            
            // Print region information
            {
                let mut uart = UART.lock();
                let _ = writeln!(uart, "Region {}: 0x{:x} - 0x{:x} ({} MB) {} {:?}",
                    i,
                    sbi_region.start,
                    sbi_region.start + sbi_region.size,
                    sbi_region.size / (1024 * 1024),
                    if (sbi_region.flags & 1) != 0 { "RAM" } else { "MMIO" },
                    zone_type
                );
            }
        }
    }
    
    /// Initialize buddy allocator for large allocations
    fn init_buddy_allocator(&mut self) {
        // Get dynamic memory layout (SBI issue now fixed!)
        let memory_layout = layout::get_memory_layout();
        
        // Use the calculated buddy allocator region
        let buddy_start = memory_layout.buddy_heap_start;
        let buddy_size = memory_layout.buddy_heap_size;
        
        {
            let mut uart = UART.lock();
            let _ = writeln!(uart, "🔧 Buddy allocator parameters:");
            let _ = writeln!(uart, "   Start: 0x{:x}", buddy_start);
            let _ = writeln!(uart, "   Size: {} bytes ({} KB)", buddy_size, buddy_size / 1024);
        }
        
        if buddy_size >= 4 * 1024 {  // Only if we have at least 4KB (minimum for buddy)
            match buddy::BuddyAllocator::new(buddy_start, buddy_size) {
                Ok(allocator) => {
                    self.buddy_allocator = Some(allocator);
                    self.free_bytes = buddy_size;
                    
                    let mut uart = UART.lock();
                    let _ = writeln!(uart, "🧩 Buddy allocator ready: 0x{:x} - 0x{:x} ({} KB)",
                        buddy_start,
                        buddy_start + buddy_size,
                        buddy_size / 1024
                    );
                }
                Err(e) => {
                    let mut uart = UART.lock();
                    let _ = writeln!(uart, "❌ Buddy allocator failed: {:?}", e);
                }
            }
        } else {
            let mut uart = UART.lock();
            let _ = writeln!(uart, "⚠️  Insufficient memory for buddy allocator: {} KB available", 
                buddy_size / 1024);
        }
    }
    
    /// Initialize small allocator for small objects
    fn init_small_allocator(&mut self) {
        // Get dynamic memory layout (SBI issue now fixed!)
        let memory_layout = layout::get_memory_layout();
        
        // Use the calculated small allocator region
        let small_start = memory_layout.small_heap_start;
        let small_size = memory_layout.small_heap_size;
        
        if small_size >= 128 * 1024 {  // Only if we have at least 128KB
            let allocator = small_alloc::SmallAllocator::new(small_start, small_size);
            self.small_allocator = Some(allocator);
            
            let mut uart = UART.lock();
            let _ = writeln!(uart, "🔍 Small allocator ready: 0x{:x} - 0x{:x} ({} KB)",
                small_start,
                small_start + small_size,
                small_size / 1024
            );
        } else {
            let mut uart = UART.lock();
            let _ = writeln!(uart, "⚠️  Insufficient memory for small allocator: {} KB available", 
                small_size / 1024);
        }
    }

    /// Smart allocation using hybrid allocation engine
    pub fn allocate(&mut self, size: usize) -> Option<usize> {
        self.allocation_count += 1;
        
        // Hybrid allocation engine strategy
        if size >= 4096 {
            // Large allocation (≥4KB): Use buddy allocator
            if let Some(ref mut buddy) = self.buddy_allocator {
                if let Some(addr) = buddy.allocate(size) {
                    self.allocated_bytes += size;
                    self.free_bytes -= size;
                    return Some(addr);
                }
            }
        } else {
            // Small allocation (<4KB): Use small allocator
            if let Some(ref mut small) = self.small_allocator {
                if let Some(ptr) = small.allocate(size) {
                    let addr = ptr as usize;
                    self.allocated_bytes += size;
                    return Some(addr);
                }
            }
        }
        
        // No suitable allocator available
        None
    }
    
    /// Smart deallocation using hybrid allocation engine
    pub fn deallocate(&mut self, addr: usize, size: usize) {
        // Try small allocator first for small allocations
        if size < 4096 {
            if let Some(ref mut small) = self.small_allocator {
                if small.owns_address(addr) {
                    small.deallocate(addr as *mut u8, size);
                    self.allocated_bytes -= size;
                    return;
                }
            }
        }
        
        // Try buddy allocator for large allocations
        if size >= 4096 {
            if let Some(ref mut buddy) = self.buddy_allocator {
                if buddy.owns_address(addr) {
                    buddy.deallocate(addr, size);
                    self.allocated_bytes -= size;
                    self.free_bytes += size;
                    return;
                }
            }
        }
        
        // For fallback allocations, we can't deallocate (bump allocator limitation)
        // This is acceptable for kernel allocations that typically live for the system lifetime
    }

    /// Get comprehensive memory usage statistics
    pub fn get_stats(&self) -> MemoryStats {
        let mut small_alloc_stats = None;
        if let Some(ref small) = self.small_allocator {
            small_alloc_stats = Some(small.get_stats());
        }
        
        // Calculate total memory from our allocators
        let memory_layout = layout::get_memory_layout();
        let total_memory = memory_layout.buddy_heap_size + memory_layout.small_heap_size;
        
        MemoryStats {
            total_memory,
            allocated_bytes: self.allocated_bytes,
            free_bytes: self.free_bytes,
            allocation_count: self.allocation_count,
            buddy_enabled: self.buddy_allocator.is_some(),
            small_alloc_enabled: self.small_allocator.is_some(),
            small_alloc_stats,
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
    pub free_bytes: usize,
    pub allocation_count: usize,
    pub buddy_enabled: bool,
    pub small_alloc_enabled: bool,
    pub small_alloc_stats: Option<small_alloc::SmallAllocatorStats>,
}

// Global memory manager instance (unified)
pub static MEMORY_MANAGER: Mutex<MemoryManager> = Mutex::new(MemoryManager::new());

// Helper functions for easy access
pub fn allocate_memory(size: usize) -> Option<usize> {
    MEMORY_MANAGER.lock().allocate(size)
}

pub fn deallocate_memory(addr: usize, size: usize) {
    MEMORY_MANAGER.lock().deallocate(addr, size);
}

pub fn get_memory_stats() -> MemoryStats {
    MEMORY_MANAGER.lock().get_stats()
} 