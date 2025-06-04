// Memory Management Module for elinOS
// Enhanced memory management with buddy allocator, slab allocator, and fallible operations
// Inspired by Maestro OS and Linux kernel memory management

pub mod layout;
pub mod buddy;
pub mod slab;
pub mod fallible;

use spin::Mutex;
use crate::{sbi, console_println};
use linked_list_allocator::LockedHeap;
use fallible::{FallibleAllocator, AllocResult, AllocError};

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

// Simple heap allocator for kernel (fallback)
#[global_allocator]
static ALLOCATOR: LockedHeap = LockedHeap::empty();

// Heap configuration 
const HEAP_SIZE: usize = 64 * 1024; // 64KB heap
static mut HEAP_SPACE: [u8; HEAP_SIZE] = [0; HEAP_SIZE];

// Enhanced Memory Manager with multiple allocator tiers
pub struct MemoryManager {
    // Memory region information
    regions: [MemoryRegion; 8],
    region_count: usize,
    
    // Memory statistics
    allocated_bytes: usize,
    allocation_count: usize,
    
    // Heap management (fallback)
    heap_start: usize,
    heap_size: usize,
    heap_used: usize,
    
    // Advanced allocators
    fallible_allocator: Option<FallibleAllocator>,
    allocator_mode: AllocatorMode,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AllocatorMode {
    /// Simple heap-only mode (current default)
    SimpleHeap,
    /// Two-tier mode: buddy + slab allocators with fallible operations
    TwoTier,
    /// Hybrid mode: fallback between allocators
    Hybrid,
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
            
            // Advanced allocators
            fallible_allocator: None,
            allocator_mode: AllocatorMode::SimpleHeap,
        }
    }

    /// Initialize memory regions and allocators
    pub fn init(&mut self) {
        self.detect_memory_layout();
        
        console_println!("🧠 Enhanced Memory Manager Initialization");
        console_println!("Memory layout detection complete");
        
        if self.region_count > 0 {
            let total_ram = self.regions[..self.region_count].iter()
                .filter(|r| r.is_ram)
                .map(|r| r.size)
                .sum::<usize>();
            
            console_println!("Total RAM detected: {} MB", total_ram / (1024 * 1024));
            
            // Try to initialize advanced allocators if we have enough memory
            if total_ram >= 4 * 1024 * 1024 { // At least 4MB
                match self.init_advanced_allocators() {
                    Ok(_) => {
                        self.allocator_mode = AllocatorMode::TwoTier;
                        console_println!("✅ Two-tier allocator system ready (Buddy + Slab)");
                    }
                    Err(e) => {
                        console_println!("⚠️  Advanced allocator init failed: {:?}", e);
                        console_println!("🔄 Falling back to simple heap allocator");
                    }
                }
            }
        } else {
            console_println!("⚠️  No memory regions detected, using defaults");
        }
        
        // Always initialize heap allocator as fallback
        self.init_heap();
        console_println!("✅ Heap allocator ready (fallback)");
        console_println!("🎉 Memory manager ready! Mode: {:?}", self.allocator_mode);
    }
    
    /// Initialize the advanced two-tier allocator system
    fn init_advanced_allocators(&mut self) -> Result<(), AllocError> {
        // Find the largest RAM region for our advanced allocators
        let largest_ram_region = self.regions[..self.region_count]
            .iter()
            .filter(|r| r.is_ram && r.zone_type == MemoryZone::Normal)
            .max_by_key(|r| r.size);
        
        if let Some(region) = largest_ram_region {
            // Use a portion of the largest region for advanced allocation
            // Reserve space for kernel heap and other uses
            let allocator_size = region.size / 2; // Use half the region
            let allocator_start = region.start + region.size - allocator_size;
            
            console_println!("🏗️  Initializing fallible allocator:");
            console_println!("  Region: 0x{:x} - 0x{:x} ({} MB)", 
                allocator_start, 
                allocator_start + allocator_size,
                allocator_size / (1024 * 1024));
            
            let fallible_allocator = FallibleAllocator::new(allocator_start, allocator_size)
                .map_err(|e| AllocError::from(e))?;
            
            self.fallible_allocator = Some(fallible_allocator);
            
            Ok(())
        } else {
            Err(AllocError::OutOfMemory)
        }
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
    
    /// Try to allocate memory using the best available allocator
    pub fn try_allocate(&mut self, size: usize) -> AllocResult<*mut u8> {
        match self.allocator_mode {
            AllocatorMode::TwoTier | AllocatorMode::Hybrid => {
                if let Some(ref mut allocator) = self.fallible_allocator {
                    match allocator.try_allocate(size) {
                        Ok(ptr) => {
                            self.allocated_bytes += size;
                            self.allocation_count += 1;
                            return Ok(ptr.as_ptr());
                        }
                        Err(e) => {
                            if self.allocator_mode == AllocatorMode::Hybrid {
                                console_println!("🔄 Fallible allocator failed, trying heap");
                                // Fallback to heap allocator
                                return self.try_allocate_from_heap(size);
                            } else {
                                return Err(e);
                            }
                        }
                    }
                }
            }
            AllocatorMode::SimpleHeap => {
                return self.try_allocate_from_heap(size);
            }
        }
        
        Err(AllocError::OutOfMemory)
    }
    
    /// Try to allocate from the simple heap (fallback)
    fn try_allocate_from_heap(&mut self, size: usize) -> AllocResult<*mut u8> {
        // For now, this is a placeholder since we can't directly control the global allocator
        // In a real implementation, we'd have our own heap allocator we can query
        
        if size == 0 {
            return Err(AllocError::InvalidSize);
        }
        
        // This is a simplified approach - we can't actually allocate from the heap here
        // without using the global allocator, which doesn't return Result types
        console_println!("⚠️  Heap allocation requested but not directly supported in fallible context");
        Err(AllocError::OutOfMemory)
    }
    
    /// Deallocate memory
    pub fn deallocate(&mut self, ptr: *mut u8, size: usize) {
        if ptr.is_null() || size == 0 {
            return;
        }
        
        if let Some(ref mut allocator) = self.fallible_allocator {
            if let Some(non_null_ptr) = core::ptr::NonNull::new(ptr) {
                allocator.deallocate(non_null_ptr, size);
                self.allocated_bytes = self.allocated_bytes.saturating_sub(size);
            }
        }
    }
    
    /// Set the allocator mode
    pub fn set_allocator_mode(&mut self, mode: AllocatorMode) {
        if mode != AllocatorMode::SimpleHeap && self.fallible_allocator.is_none() {
            console_println!("⚠️  Cannot set mode {:?} - advanced allocators not initialized", mode);
            return;
        }
        
        self.allocator_mode = mode;
        console_println!("🔧 Allocator mode changed to: {:?}", mode);
    }

    pub fn show_stats(&self) {
        let stats = self.get_stats();
        console_println!("=== Enhanced Memory Manager Statistics ===");
        console_println!("Mode: {:?}", self.allocator_mode);
        console_println!("Total Memory: {} MB", stats.total_memory / (1024 * 1024));
        console_println!("Allocated: {} bytes", stats.allocated_bytes);
        console_println!("Allocations: {}", stats.allocation_count);
        
        // Show fallible allocator stats if available
        if let Some(ref allocator) = self.fallible_allocator {
            let fallible_stats = allocator.get_stats();
            console_println!("--- Fallible Allocator Stats ---");
            console_println!("Total allocations: {}", fallible_stats.slab_stats.total_allocations);
            console_println!("Total deallocations: {}", fallible_stats.slab_stats.total_deallocations);
            console_println!("Allocation failures: {}", fallible_stats.allocation_failures);
            console_println!("OOM events: {}", fallible_stats.oom_events);
            console_println!("Failure rate: {:.2}%", fallible_stats.failure_rate * 100.0);
            console_println!("Health status: {}", if allocator.is_healthy() { "✅ Healthy" } else { "⚠️ Degraded" });
            console_println!("Fragmentation: {:.2}%", fallible_stats.slab_stats.fragmentation_ratio * 100.0);
        }
        
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
            allocator_mode: self.allocator_mode,
        }
    }

    /// Get memory region information
    pub fn get_memory_info(&self) -> &[MemoryRegion] {
        &self.regions[..self.region_count]
    }
    
    /// Check if the memory manager is in a healthy state
    pub fn is_healthy(&self) -> bool {
        match &self.fallible_allocator {
            Some(allocator) => allocator.is_healthy(),
            None => true, // Simple heap is always "healthy"
        }
    }
}

/// Enhanced memory usage statistics
#[derive(Debug)]
pub struct MemoryStats {
    pub total_memory: usize,
    pub allocated_bytes: usize,
    pub allocation_count: usize,
    pub allocator_mode: AllocatorMode,
}

// Global memory manager instance
pub static MEMORY_MANAGER: Mutex<MemoryManager> = Mutex::new(MemoryManager::new());

/// Convenience functions for memory allocation
pub fn allocate_memory(size: usize) -> Option<usize> {
    let mut manager = MEMORY_MANAGER.lock();
    match manager.try_allocate(size) {
        Ok(ptr) => Some(ptr as usize),
        Err(_) => None,
    }
}

pub fn deallocate_memory(addr: usize, size: usize) {
    let mut manager = MEMORY_MANAGER.lock();
    manager.deallocate(addr as *mut u8, size);
}

pub fn get_memory_stats() -> MemoryStats {
    let manager = MEMORY_MANAGER.lock();
    manager.get_stats()
}

/// Try to allocate memory with fallible semantics
pub fn try_allocate_memory(size: usize) -> AllocResult<*mut u8> {
    let mut manager = MEMORY_MANAGER.lock();
    manager.try_allocate(size)
}

/// Set the memory allocator mode
pub fn set_allocator_mode(mode: AllocatorMode) {
    let mut manager = MEMORY_MANAGER.lock();
    manager.set_allocator_mode(mode);
}

/// Check memory manager health
pub fn is_memory_healthy() -> bool {
    let manager = MEMORY_MANAGER.lock();
    manager.is_healthy()
} 