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

// === DYNAMIC MEMORY MANAGER ===

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

// Dynamic heap configuration - calculated at runtime
static mut HEAP_SPACE: Option<&'static mut [u8]> = None;
static mut DYNAMIC_HEAP_SIZE: usize = 0;

// Enhanced Memory Manager with automatic hardware detection
pub struct MemoryManager {
    // Detected memory configuration
    detected_ram_size: usize,
    detected_regions: heapless::Vec<MemoryRegion, 16>,
    
    // Dynamic allocation sizes (calculated from detected RAM)
    heap_size: usize,
    stack_size: usize,
    buddy_heap_size: usize,
    max_file_buffer_size: usize,
    
    // Memory statistics
    allocated_bytes: usize,
    allocation_count: usize,
    
    // Heap management (fallback)
    heap_start: usize,
    heap_used: usize,
    
    // Advanced allocators
    fallible_allocator: Option<FallibleAllocator>,
    allocator_mode: AllocatorMode,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AllocatorMode {
    /// Simple heap-only mode (for low-memory systems)
    SimpleHeap,
    /// Two-tier mode: buddy + slab allocators with fallible operations
    TwoTier,
    /// Hybrid mode: fallback between allocators
    Hybrid,
}

impl MemoryManager {
    pub const fn new() -> Self {
        MemoryManager {
            // Will be detected at runtime
            detected_ram_size: 0,
            detected_regions: heapless::Vec::new(),
            
            // Will be calculated based on detected RAM
            heap_size: 0,
            stack_size: 0,
            buddy_heap_size: 0,
            max_file_buffer_size: 0,
            
            // Statistics
            allocated_bytes: 0,
            allocation_count: 0,
            
            // Heap
            heap_start: 0,
            heap_used: 0,
            
            // Advanced allocators
            fallible_allocator: None,
            allocator_mode: AllocatorMode::SimpleHeap,
        }
    }

    /// Initialize memory manager with full hardware detection and dynamic sizing
    pub fn init(&mut self) {
        console_println!("🧠 Enhanced Memory Manager - Auto-detecting hardware...");
        
        // Step 1: Detect all available RAM
        self.detect_memory_hardware();
        
        // Step 2: Calculate optimal sizes based on detected hardware
        self.calculate_dynamic_sizes();
        
        // Step 3: Initialize appropriate allocator based on available memory
        self.initialize_allocators();
        
        console_println!("✅ Memory manager initialized dynamically!");
        self.show_detection_summary();
    }
    
    /// Auto-detect memory hardware using SBI and direct probing
    fn detect_memory_hardware(&mut self) {
        console_println!("🔍 Detecting memory hardware...");
        
        // Get memory regions from SBI
        let sbi_regions = sbi::get_memory_regions();
        let mut total_ram = 0;
        
        console_println!("📋 Memory regions detected:");
        for i in 0..sbi_regions.count {
            let sbi_region = &sbi_regions.regions[i];
            
            // Determine memory zone based on address
            let zone_type = if sbi_region.start < 16 * 1024 * 1024 {
                MemoryZone::DMA
            } else if sbi_region.start < 896 * 1024 * 1024 {
                MemoryZone::Normal
            } else {
                MemoryZone::High
            };
            
            let region = MemoryRegion {
                start: sbi_region.start,
                size: sbi_region.size,
                is_ram: (sbi_region.flags & 1) != 0,
                zone_type,
            };
            
            if region.is_ram {
                total_ram += region.size;
                console_println!("  RAM  Region {}: 0x{:08x} - 0x{:08x} ({} MB) {:?}",
                    i,
                    region.start,
                    region.start + region.size,
                    region.size / (1024 * 1024),
                    zone_type
                );
            } else {
                console_println!("  MMIO Region {}: 0x{:08x} - 0x{:08x} ({} KB) {:?}",
                    i,
                    region.start,
                    region.start + region.size,
                    region.size / 1024,
                    zone_type
                );
            }
            
            let _ = self.detected_regions.push(region);
        }
        
        // Store detected RAM size
        self.detected_ram_size = total_ram;
        
        if total_ram == 0 {
            console_println!("⚠️  No RAM detected via SBI - using conservative fallback");
            // Create a fallback memory layout
            self.detected_ram_size = 128 * 1024 * 1024; // 128MB fallback
            let fallback_region = MemoryRegion {
                start: 0x80000000,
                size: 128 * 1024 * 1024,
                is_ram: true,
                zone_type: MemoryZone::Normal,
            };
            let _ = self.detected_regions.push(fallback_region);
        }
        
        console_println!("📊 Total RAM detected: {} MB", self.detected_ram_size / (1024 * 1024));
    }
    
    /// Calculate optimal memory allocation sizes based on detected hardware
    fn calculate_dynamic_sizes(&mut self) {
        console_println!("🧮 Calculating optimal memory allocation sizes...");
        
        let ram_mb = self.detected_ram_size / (1024 * 1024);
        
        // Calculate heap size (scale with available RAM)
        self.heap_size = match ram_mb {
            0..=8    => 32 * 1024,      // 32KB for very small systems
            9..=32   => 128 * 1024,     // 128KB for small systems  
            33..=128 => 512 * 1024,     // 512KB for medium systems
            129..=512 => 2 * 1024 * 1024, // 2MB for large systems
            _ => 8 * 1024 * 1024,       // 8MB for very large systems
        };
        
        // Calculate stack size (smaller scaling)
        self.stack_size = match ram_mb {
            0..=8    => 8 * 1024,       // 8KB stack
            9..=32   => 16 * 1024,      // 16KB stack
            33..=128 => 32 * 1024,      // 32KB stack
            _ => 64 * 1024,             // 64KB stack for large systems
        };
        
        // Calculate buddy allocator size (for advanced memory management)
        self.buddy_heap_size = match ram_mb {
            0..=8    => 0,              // No buddy allocator for tiny systems
            9..=32   => 1 * 1024 * 1024, // 1MB
            33..=128 => 4 * 1024 * 1024, // 4MB
            129..=512 => 16 * 1024 * 1024, // 16MB
            _ => 64 * 1024 * 1024,      // 64MB for very large systems
        };
        
        // Calculate maximum file buffer size
        self.max_file_buffer_size = match ram_mb {
            0..=8    => 4 * 1024,       // 4KB max file
            9..=32   => 16 * 1024,      // 16KB max file
            33..=128 => 64 * 1024,      // 64KB max file
            129..=512 => 256 * 1024,    // 256KB max file
            _ => 1024 * 1024,           // 1MB max file for large systems
        };
        
        console_println!("📏 Calculated sizes:");
        console_println!("  Heap: {} KB", self.heap_size / 1024);
        console_println!("  Stack: {} KB", self.stack_size / 1024);
        console_println!("  Buddy heap: {} KB", self.buddy_heap_size / 1024);
        console_println!("  Max file buffer: {} KB", self.max_file_buffer_size / 1024);
    }
    
    /// Initialize the most appropriate allocator based on available memory
    fn initialize_allocators(&mut self) {
        console_println!("🏗️  Initializing allocators...");
        
        // Always initialize basic heap allocator
        self.init_dynamic_heap();
        
        // Try to initialize advanced allocators if we have enough memory
        if self.detected_ram_size >= 16 * 1024 * 1024 && self.buddy_heap_size > 0 {
            console_println!("🚀 Sufficient memory for advanced allocators");
            match self.init_advanced_allocators() {
                Ok(_) => {
                    self.allocator_mode = AllocatorMode::TwoTier;
                    console_println!("✅ Two-tier allocator system ready (Buddy + Slab)");
                }
                Err(e) => {
                    console_println!("⚠️  Advanced allocator init failed: {:?}", e);
                    console_println!("🔄 Using simple heap allocator");
                    self.allocator_mode = AllocatorMode::SimpleHeap;
                }
            }
        } else {
            console_println!("💡 Using simple heap allocator for limited memory system");
            self.allocator_mode = AllocatorMode::SimpleHeap;
        }
    }
    
    /// Initialize heap allocator with dynamically calculated size
    fn init_dynamic_heap(&mut self) {
        console_println!("🏗️  Initializing dynamic heap: {} KB", self.heap_size / 1024);
        
        // Find a suitable memory region for our heap
        let heap_region = self.detected_regions.iter()
            .find(|r| r.is_ram && r.zone_type == MemoryZone::Normal && r.size >= self.heap_size);
            
        if let Some(region) = heap_region {
            // Use part of this region for our heap
            let heap_start = region.start + 1024 * 1024; // Leave 1MB for kernel
            
            unsafe {
                // Store heap info before creating slice
                self.heap_start = heap_start;
                self.heap_used = 0;
                DYNAMIC_HEAP_SIZE = self.heap_size;
                
                // Create a slice for our heap space
                let heap_slice = core::slice::from_raw_parts_mut(
                    heap_start as *mut u8,
                    self.heap_size
                );
                HEAP_SPACE = Some(heap_slice);
                
                // Initialize the global allocator with raw pointer to avoid borrow issues
                ALLOCATOR.lock().init(heap_start as *mut u8, self.heap_size);
                
                console_println!("✅ Dynamic heap: 0x{:08x} - 0x{:08x} ({} KB)",
                    heap_start,
                    heap_start + self.heap_size,
                    self.heap_size / 1024
                );
            }
        } else {
            console_println!("❌ Could not find suitable memory region for heap");
        }
    }
    
    /// Initialize advanced allocators if memory permits
    fn init_advanced_allocators(&mut self) -> Result<(), AllocError> {
        // Find a large memory region for advanced allocators
        let suitable_region = self.detected_regions.iter()
            .find(|r| r.is_ram && r.zone_type == MemoryZone::Normal && r.size >= self.buddy_heap_size * 2);
            
        if let Some(region) = suitable_region {
            // Use a portion for advanced allocation (leave space for other uses)
            let allocator_start = region.start + region.size - self.buddy_heap_size;
            
            console_println!("🏗️  Initializing fallible allocator:");
            console_println!("  Region: 0x{:08x} - 0x{:08x} ({} MB)", 
                allocator_start, 
                allocator_start + self.buddy_heap_size,
                self.buddy_heap_size / (1024 * 1024));
            
            let fallible_allocator = FallibleAllocator::new(allocator_start, self.buddy_heap_size)
                .map_err(|e| AllocError::from(e))?;
            
            self.fallible_allocator = Some(fallible_allocator);
            Ok(())
        } else {
            Err(AllocError::OutOfMemory)
        }
    }
    
    /// Show summary of hardware detection and allocation decisions
    fn show_detection_summary(&self) {
        console_println!("📊 Dynamic Memory Manager Summary:");
        console_println!("=====================================");
        console_println!("Detected Hardware:");
        console_println!("  Total RAM: {} MB", self.detected_ram_size / (1024 * 1024));
        console_println!("  Memory Regions: {}", self.detected_regions.len());
        console_println!();
        console_println!("Calculated Allocations:");
        console_println!("  Heap Size: {} KB", self.heap_size / 1024);
        console_println!("  Stack Size: {} KB", self.stack_size / 1024);
        console_println!("  Max File Buffer: {} KB", self.max_file_buffer_size / 1024);
        console_println!("  Allocator Mode: {:?}", self.allocator_mode);
        console_println!();
        
        // Show memory efficiency
        let allocated_for_kernel = self.heap_size + self.stack_size + self.buddy_heap_size;
        let efficiency = (allocated_for_kernel as f32 / self.detected_ram_size as f32) * 100.0;
        console_println!("Memory Efficiency:");
        console_println!("  Kernel Usage: {} MB ({:.1}% of total RAM)", 
            allocated_for_kernel / (1024 * 1024), efficiency);
        console_println!("  Available for Programs: {} MB", 
            (self.detected_ram_size - allocated_for_kernel) / (1024 * 1024));
    }

    /// Get dynamic buffer size for different use cases
    pub fn get_optimal_buffer_size(&self, usage: BufferUsage) -> usize {
        match usage {
            BufferUsage::SectorIO => 512, // Always 512 for disk sectors
            BufferUsage::FileRead => self.max_file_buffer_size.min(64 * 1024), // Cap at 64KB for safety
            BufferUsage::Command => {
                if self.detected_ram_size < 8 * 1024 * 1024 {
                    128 // Small command buffer for limited memory
                } else {
                    512 // Larger command buffer for systems with more memory
                }
            }
            BufferUsage::Network => {
                if self.detected_ram_size < 32 * 1024 * 1024 {
                    1500 // MTU size for small systems
                } else {
                    8192 // Larger network buffers for bigger systems
                }
            }
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
        if size == 0 {
            return Err(AllocError::InvalidSize);
        }
        
        // Check if we have space in our heap
        if self.heap_used + size > self.heap_size {
            return Err(AllocError::OutOfMemory);
        }
        
        // For now, this is a placeholder since we can't directly control the global allocator
        // In a real implementation, we'd have our own heap allocator we can query
        console_println!("⚠️  Heap allocation requested: {} bytes", size);
        
        // Simulate allocation tracking
        self.heap_used += size;
        self.allocation_count += 1;
        
        Err(AllocError::OutOfMemory) // Return error for now since we can't allocate directly
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
        
        // Update heap tracking
        self.heap_used = self.heap_used.saturating_sub(size);
    }

    /// Get comprehensive memory statistics
    pub fn get_stats(&self) -> MemoryStats {
        MemoryStats {
            detected_ram_size: self.detected_ram_size,
            allocated_bytes: self.allocated_bytes,
            allocation_count: self.allocation_count,
            allocator_mode: self.allocator_mode,
            heap_size: self.heap_size,
            heap_used: self.heap_used,
            regions_detected: self.detected_regions.len(),
        }
    }

    /// Get memory region information
    pub fn get_memory_info(&self) -> &[MemoryRegion] {
        &self.detected_regions
    }
    
    /// Check if the memory manager is in a healthy state
    pub fn is_healthy(&self) -> bool {
        match &self.fallible_allocator {
            Some(allocator) => allocator.is_healthy(),
            None => self.heap_used < (self.heap_size * 9 / 10), // Less than 90% heap usage
        }
    }
    
    /// Get the maximum file size this system can handle
    pub fn get_max_file_size(&self) -> usize {
        self.max_file_buffer_size
    }
}

#[derive(Debug)]
pub enum BufferUsage {
    SectorIO,   // For disk sector operations
    FileRead,   // For reading files
    Command,    // For command line input
    Network,    // For network operations
}

/// Enhanced memory usage statistics
#[derive(Debug)]
pub struct MemoryStats {
    pub detected_ram_size: usize,
    pub allocated_bytes: usize,
    pub allocation_count: usize,
    pub allocator_mode: AllocatorMode,
    pub heap_size: usize,
    pub heap_used: usize,
    pub regions_detected: usize,
}

// Global memory manager instance
pub static MEMORY_MANAGER: Mutex<MemoryManager> = Mutex::new(MemoryManager::new());

/// Get the optimal buffer size for a specific usage
pub fn get_optimal_buffer_size(usage: BufferUsage) -> usize {
    let manager = MEMORY_MANAGER.lock();
    manager.get_optimal_buffer_size(usage)
}

/// Get the maximum file size the system can handle
pub fn get_max_file_size() -> usize {
    let manager = MEMORY_MANAGER.lock();
    manager.get_max_file_size()
}

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

/// Check memory manager health
pub fn is_memory_healthy() -> bool {
    let manager = MEMORY_MANAGER.lock();
    manager.is_healthy()
} 