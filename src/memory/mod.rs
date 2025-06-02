// Memory Management Module for elinKernel
// Implements Maestro-inspired design with MIT-licensed buddy allocator

pub mod buddy;
pub mod small_alloc;
pub mod vmm;

use core::fmt::Write;
use core::option::Option::{self, Some, None};
use core::writeln;
use spin::Mutex;
use crate::UART;
use crate::sbi;

// === LEGACY MEMORY MANAGER FOR BACKWARD COMPATIBILITY ===

// Legacy memory region structure
#[derive(Debug, Clone, Copy)]
pub struct LegacyMemoryRegion {
    pub start: usize,
    pub size: usize,
    pub is_ram: bool,
}

// Legacy memory manager structure
pub struct LegacyMemoryManager {
    regions: [LegacyMemoryRegion; 8],  // Support up to 8 memory regions
    region_count: usize,
    heap_start: usize,
    heap_end: usize,
    current_heap: usize,
}

impl LegacyMemoryManager {
    pub const fn new() -> Self {
        LegacyMemoryManager {
            regions: [LegacyMemoryRegion { start: 0, size: 0, is_ram: false }; 8],
            region_count: 0,
            heap_start: 0,
            heap_end: 0,
            current_heap: 0,
        }
    }

    // Initialize memory regions from OpenSBI
    pub fn init(&mut self) {
        
        let sbi_regions = sbi::get_memory_regions();
        
        // Convert SBI regions to our format
        self.region_count = sbi_regions.count;
        for i in 0..self.region_count {
            let sbi_region = &sbi_regions.regions[i];
            self.regions[i] = LegacyMemoryRegion {
                start: sbi_region.start,
                size: sbi_region.size,
                is_ram: (sbi_region.flags & 1) != 0,  // Check if region is RAM
            };
        }
        
        // Set up heap in the first RAM region
        if self.region_count > 0 {
            for region in &self.regions[..self.region_count] {
                if region.is_ram {
                    self.heap_start = region.start + 2 * 1024 * 1024;  // Leave 2MB for kernel
                    self.heap_end = region.start + region.size;
                    self.current_heap = self.heap_start;
                    
                    {
                        let mut uart = UART.lock();
                        let _ = writeln!(uart, "Legacy heap configured: 0x{:x} - 0x{:x}",
                            self.heap_start,
                            self.heap_end
                        );
                    }
                    break;
                }
            }
        }
    }

    pub fn allocate(&mut self, size: usize) -> Option<usize> {
        let aligned_size = (size + 7) & !7;  // 8-byte alignment
        if self.current_heap + aligned_size > self.heap_end {
            None
        } else {
            let ptr = self.current_heap;
            self.current_heap += aligned_size;
            Some(ptr)
        }
    }

    pub fn get_memory_info(&self) -> &[LegacyMemoryRegion] {
        &self.regions[..self.region_count]
    }
}

// Global legacy memory manager instance
pub static MEMORY_MANAGER: Mutex<LegacyMemoryManager> = Mutex::new(LegacyMemoryManager::new());

// === ADVANCED MEMORY MANAGER ===

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

// Advanced Memory Manager with buddy allocator
pub struct AdvancedMemoryManager {
    // Legacy compatibility
    regions: [MemoryRegion; 8],
    region_count: usize,
    heap_start: usize,
    heap_end: usize,
    current_heap: usize,
    
    // New buddy allocator components
    buddy_allocator: Option<buddy::BuddyAllocator>,
    small_allocator: Option<small_alloc::SmallAllocator>,
    
    // Memory statistics
    allocated_bytes: usize,
    free_bytes: usize,
    allocation_count: usize,
}

impl AdvancedMemoryManager {
    pub const fn new() -> Self {
        AdvancedMemoryManager {
            // Legacy fields
            regions: [MemoryRegion { 
                start: 0, 
                size: 0, 
                is_ram: false, 
                zone_type: MemoryZone::Normal 
            }; 8],
            region_count: 0,
            heap_start: 0,
            heap_end: 0,
            current_heap: 0,
            
            // New fields
            buddy_allocator: None,
            small_allocator: None,
            allocated_bytes: 0,
            free_bytes: 0,
            allocation_count: 0,
        }
    }

    /// Initialize memory regions and buddy allocator
    pub fn init(&mut self) {
        {
            let mut uart = UART.lock();
            let _ = writeln!(uart, "Initializing advanced memory management...");
        }
        
        // First, do the legacy initialization
        self.init_legacy();
        
        // Then initialize the buddy allocator
        self.init_buddy_allocator();
        
        // Initialize small allocator
        self.init_small_allocator();
        
        {
            let mut uart = UART.lock();
            let _ = writeln!(uart, "Advanced memory management initialized successfully!");
        }
    }
    
    /// Legacy initialization (for backward compatibility)
    fn init_legacy(&mut self) {
        {
            let mut uart = UART.lock();
            let _ = writeln!(uart, "Detecting memory regions through OpenSBI...");
        }
        
        // Get memory regions from OpenSBI
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
        
        // Set up legacy heap in the first RAM region
        if self.region_count > 0 {
            for region in &self.regions[..self.region_count] {
                if region.is_ram {
                    self.heap_start = region.start + 2 * 1024 * 1024;  // Leave 2MB for kernel
                    self.heap_end = region.start + region.size;
                    self.current_heap = self.heap_start;
                    
                    {
                        let mut uart = UART.lock();
                        let _ = writeln!(uart, "Legacy heap configured: 0x{:x} - 0x{:x}",
                            self.heap_start,
                            self.heap_end
                        );
                    }
                    break;
                }
            }
        }
    }
    
    /// Initialize buddy allocator for large allocations
    fn init_buddy_allocator(&mut self) {
        if self.region_count > 0 {
            for region in &self.regions[..self.region_count] {
                if region.is_ram && region.zone_type == MemoryZone::Normal {
                    // Use part of normal RAM for buddy allocator
                    let buddy_start = region.start + 4 * 1024 * 1024;  // Leave 4MB for kernel and legacy heap
                    let buddy_size = region.size - 4 * 1024 * 1024;
                    
                    if buddy_size >= 1024 * 1024 {  // Only if we have at least 1MB
                        match buddy::BuddyAllocator::new(buddy_start, buddy_size) {
                            Ok(allocator) => {
                                self.buddy_allocator = Some(allocator);
                                self.free_bytes = buddy_size;
                                
                                let mut uart = UART.lock();
                                let _ = writeln!(uart, "Buddy allocator initialized: 0x{:x} - 0x{:x} ({} MB)",
                                    buddy_start,
                                    buddy_start + buddy_size,
                                    buddy_size / (1024 * 1024)
                                );
                            }
                            Err(e) => {
                                let mut uart = UART.lock();
                                let _ = writeln!(uart, "Failed to initialize buddy allocator: {:?}", e);
                            }
                        }
                    }
                    break;
                }
            }
        }
    }
    
    /// Initialize small allocator for small objects
    fn init_small_allocator(&mut self) {
        if self.region_count > 0 {
            for region in &self.regions[..self.region_count] {
                if region.is_ram && region.zone_type == MemoryZone::Normal {
                    // Use part of normal RAM for small allocator (after buddy allocator)
                    let small_start = region.start + 6 * 1024 * 1024;  // Leave 6MB for kernel, legacy heap, and buddy
                    let small_size = 2 * 1024 * 1024; // 2MB for small allocator
                    
                    if small_start + small_size <= region.start + region.size {
                        let allocator = small_alloc::SmallAllocator::new(small_start, small_size);
                        self.small_allocator = Some(allocator);
                        
                        let mut uart = UART.lock();
                        let _ = writeln!(uart, "Small allocator initialized: 0x{:x} - 0x{:x} ({} MB)",
                            small_start,
                            small_start + small_size,
                            small_size / (1024 * 1024)
                        );
                    }
                    break;
                }
            }
        }
    }

    /// Allocate memory using two-tier strategy
    pub fn allocate(&mut self, size: usize) -> Option<usize> {
        self.allocation_count += 1;
        
        // Two-tier allocation strategy inspired by Maestro
        if size >= 4096 {
            // Large allocation: Use buddy allocator
            if let Some(ref mut buddy) = self.buddy_allocator {
                if let Some(addr) = buddy.allocate(size) {
                    self.allocated_bytes += size;
                    self.free_bytes -= size;
                    return Some(addr);
                }
            }
        } else {
            // Small allocation: Use small allocator first
            if let Some(ref mut small) = self.small_allocator {
                if let Some(ptr) = small.allocate(size) {
                    let addr = ptr as usize;
                    self.allocated_bytes += size;
                    return Some(addr);
                }
            }
        }
        
        // Fallback: Use legacy allocator
        self.allocate_legacy(size)
    }
    
    /// Deallocate memory using two-tier strategy
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
        
        // For legacy allocations, we can't deallocate (bump allocator limitation)
        // TODO: Track legacy allocations for proper deallocation
    }

    /// Legacy allocation for backward compatibility
    pub fn allocate_legacy(&mut self, size: usize) -> Option<usize> {
        let aligned_size = (size + 7) & !7;  // 8-byte alignment
        if self.current_heap + aligned_size > self.heap_end {
            None
        } else {
            let ptr = self.current_heap;
            self.current_heap += aligned_size;
            self.allocated_bytes += aligned_size;
            Some(ptr)
        }
    }

    /// Get memory usage statistics
    pub fn get_stats(&self) -> MemoryStats {
        let mut small_alloc_stats = None;
        if let Some(ref small) = self.small_allocator {
            small_alloc_stats = Some(small.get_stats());
        }
        
        MemoryStats {
            total_memory: self.heap_end - self.heap_start,
            allocated_bytes: self.allocated_bytes,
            free_bytes: self.free_bytes,
            allocation_count: self.allocation_count,
            buddy_enabled: self.buddy_allocator.is_some(),
            small_alloc_enabled: self.small_allocator.is_some(),
            small_alloc_stats,
        }
    }

    /// Get memory info (legacy compatibility)
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

// Global advanced memory manager instance
pub static ADVANCED_MEMORY_MANAGER: Mutex<AdvancedMemoryManager> = Mutex::new(AdvancedMemoryManager::new());

// Helper functions for compatibility
pub fn allocate_memory(size: usize) -> Option<usize> {
    ADVANCED_MEMORY_MANAGER.lock().allocate(size)
}

pub fn deallocate_memory(addr: usize, size: usize) {
    ADVANCED_MEMORY_MANAGER.lock().deallocate(addr, size);
}

pub fn get_memory_stats() -> MemoryStats {
    ADVANCED_MEMORY_MANAGER.lock().get_stats()
} 