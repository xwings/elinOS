// Dynamic Memory Layout Manager for elinOS
// Replaces hardcoded memory allocations with intelligent detection

use crate::console_println;
use heapless::Vec;
use super::regions::{MemoryRegion, MemoryZone};
use super::hardware::{get_kernel_boundaries, get_stack_boundaries, detect_main_ram, get_fallback_ram, get_standard_mmio_regions, calculate_heap_start, validate_memory_layout};

/// Memory layout information calculated from linker symbols
#[derive(Debug, Clone)]
pub struct MemoryLayout {
    pub kernel_start: usize,
    pub kernel_end: usize,
    pub kernel_size: usize,
    pub stack_start: usize,
    pub stack_end: usize,
    pub stack_size: usize,
    pub total_kernel_footprint: usize,
    
    // Dynamic heap layout
    pub heap_start: usize,
    pub buddy_heap_start: usize,
    pub buddy_heap_size: usize,
    pub small_heap_start: usize,
    pub small_heap_size: usize,
    
    // Device memory region (for VirtIO and other devices)
    pub device_memory_start: usize,
    pub device_memory_size: usize,
    pub device_memory_used: usize,
    
    // Safety margins
    pub kernel_guard_size: usize,
    pub stack_guard_size: usize,
    
    // Memory regions
    pub regions: Vec<MemoryRegion, 8>,
    pub heap_size: usize,
    pub available_memory: usize,
}

impl MemoryLayout {
    /// Calculate memory layout dynamically from linker symbols
    pub fn detect() -> Self {
        console_println!("[i] Detecting memory layout via OpenSBI...");
        
        // Calculate kernel and stack boundaries using shared functions
        let (kernel_start, kernel_end, kernel_size) = get_kernel_boundaries();
        let (stack_start, stack_end, stack_size) = get_stack_boundaries();
        
        let mut layout = MemoryLayout {
            kernel_start,
            kernel_end,
            kernel_size,
            stack_start,
            stack_end,
            stack_size,
            total_kernel_footprint: kernel_size + stack_size,
            heap_start: 0,
            buddy_heap_start: 0,
            buddy_heap_size: 0,
            small_heap_start: 0,
            small_heap_size: 0,
            device_memory_start: 0,
            device_memory_size: 0,
            device_memory_used: 0,
            kernel_guard_size: 4096,
            stack_guard_size: 4096,
            regions: Vec::new(),
            heap_size: 0,
            available_memory: 0,
        };
        
        // Detect main RAM using shared hardware detection
        if let Some(ram_region) = detect_main_ram() {
            layout.add_region(ram_region.start, ram_region.size, ram_region.is_ram, ram_region.zone_type);
            console_println!("[o] Detected {} MB RAM at 0x{:x}", ram_region.size / (1024 * 1024), ram_region.start);
        } else {
            // Fallback to default QEMU layout
            let fallback = get_fallback_ram();
            layout.add_region(fallback.start, fallback.size, fallback.is_ram, fallback.zone_type);
            console_println!("[!]  Using fallback memory layout: 128MB at 0x80000000");
        }
        
        // Add standard MMIO regions using shared function
        for mmio_region in get_standard_mmio_regions() {
            layout.add_region(mmio_region.start, mmio_region.size, mmio_region.is_ram, mmio_region.zone_type);
        }
        
        // Set up heap areas after kernel using shared function
        let heap_start = calculate_heap_start(kernel_end, layout.kernel_guard_size);
        layout.heap_start = heap_start;
        layout.buddy_heap_start = heap_start;
        layout.buddy_heap_size = 256 * 1024; // 256KB for buddy allocator
        layout.small_heap_start = layout.buddy_heap_start + layout.buddy_heap_size;
        layout.small_heap_size = 64 * 1024; // 64KB for small allocator
        layout.heap_size = layout.buddy_heap_size + layout.small_heap_size;
        
        // Set up device memory region after regular heap
        // Device memory needs to be DMA-accessible and properly aligned
        // The linker places heap at 0x80400000 with 512KB size, so place device memory after that
        let linker_heap_start = 0x80400000;
        let linker_heap_size = 512 * 1024; // 512KB as defined by the memory manager
        let device_memory_size = 1024 * 1024; // 1MB for device operations (VirtIO, etc.)
        layout.device_memory_start = linker_heap_start + linker_heap_size;
        layout.device_memory_size = device_memory_size;
        layout.device_memory_used = 0;
        
        // Debug output to see the conflict
        console_println!("[i] Memory layout debug:");
        console_println!("   Kernel start: 0x{:08x}", kernel_start);
        console_println!("   Kernel end: 0x{:08x}", kernel_end);
        console_println!("   Kernel size: {} KB", kernel_size / 1024);
        console_println!("   Stack start: 0x{:08x}", stack_start);
        console_println!("   Stack end: 0x{:08x}", stack_end);
        console_println!("   Stack size: {} KB", stack_size / 1024);
        console_println!("   Total kernel footprint: {} KB", layout.total_kernel_footprint / 1024);
        console_println!("   Calculated heap start: 0x{:08x}", heap_start);
        console_println!("   Linker heap start: 0x80400000");
        
        layout
    }
    
    /// Add a memory region to the layout
    pub fn add_region(&mut self, start: usize, size: usize, is_ram: bool, zone_type: MemoryZone) {
        let region = MemoryRegion {
            start,
            size,
            is_ram,
            zone_type,
        };
        
        if self.regions.push(region).is_err() {
            // Handle error - regions vector is full
            return;
        }
        
        if is_ram {
            self.available_memory += size;
        }
    }
    
    /// Intelligently distribute heap memory based on available space
    fn calculate_heap_distribution(available_memory: usize) -> (usize, usize) {
        // Buddy allocator constraints:
        // - Size must be > 0
        // - Bitmap size must be <= 4096 bytes
        // - Bitmap size = (total_size / MIN_BLOCK_SIZE) / 8
        // - For MIN_BLOCK_SIZE = 1: bitmap size = total_size / 8
        // - Max safe buddy size = 4096 * 8 = 32KB
        // - Being extra conservative: use 16KB max to be safe
        
        const MAX_SAFE_BUDDY_SIZE: usize = 16 * 1024;  // 16KB max (well under 4KB bitmap limit)
        const MIN_BUDDY_SIZE: usize = 4 * 1024;        // 4KB minimum
        
        if available_memory < 1024 * 1024 {
            // Less than 1MB - minimal allocation
            let buddy = MIN_BUDDY_SIZE;                 // 4KB buddy (minimum safe)
            let small = 128 * 1024;                     // 128KB small
            (buddy, small)
        } else if available_memory < 8 * 1024 * 1024 {
            // 1-8MB - small system
            let buddy = MAX_SAFE_BUDDY_SIZE;            // 16KB buddy (safe)
            let small = 1024 * 1024;                    // 1MB small
            (buddy, small)
        } else if available_memory < 64 * 1024 * 1024 {
            // 8-64MB - medium system
            let buddy = MAX_SAFE_BUDDY_SIZE;            // 16KB buddy (safe)
            let small = 4 * 1024 * 1024;               // 4MB small
            (buddy, small)
        } else {
            // 64MB+ - large system
            let buddy = MAX_SAFE_BUDDY_SIZE;            // 16KB buddy (safe)
            let small = 8 * 1024 * 1024;               // 8MB small
            (buddy, small)
        }
    }
    
    /// Validate the memory layout
    pub fn validate(&self) -> Result<(), &'static str> {
        // Use shared validation for basic checks
        validate_memory_layout(self.kernel_start, self.kernel_end, self.heap_start)?;
        
        // The actual heap is placed by the linker at 0x80400000, not at our calculated position
        // So we need to check if the linker heap conflicts with kernel space
        let linker_heap_start = 0x80400000;
        let kernel_end_with_guard = self.kernel_end + self.kernel_guard_size;
        
        console_println!("[i] Validation check:");
        console_println!("   Kernel end + guard: 0x{:08x}", kernel_end_with_guard);
        console_println!("   Linker heap start: 0x{:08x}", linker_heap_start);
        
        if linker_heap_start <= kernel_end_with_guard {
            return Err("Linker heap overlaps with kernel space");
        }
        
        // Check device memory doesn't overlap with other regions
        let device_end = self.device_memory_start + self.device_memory_size;
        let linker_heap_end = linker_heap_start + 512 * 1024; // 512KB heap size
        if self.device_memory_start < linker_heap_end {
            return Err("Device memory overlaps with linker heap");
        }
        
        console_println!("[o] Memory layout validation passed");
        Ok(())
    }
    
    /// Allocate device memory from the reserved device memory region
    pub fn allocate_device_memory(&mut self, size: usize, alignment: usize) -> Result<usize, &'static str> {
        // Align the current used offset
        let aligned_used = (self.device_memory_used + alignment - 1) & !(alignment - 1);
        
        // Check if we have enough space
        if aligned_used + size > self.device_memory_size {
            return Err("Not enough device memory");
        }
        
        let allocated_addr = self.device_memory_start + aligned_used;
        self.device_memory_used = aligned_used + size;
        
        // Zero out the allocated memory
        unsafe {
            core::ptr::write_bytes(allocated_addr as *mut u8, 0, size);
        }
        
        Ok(allocated_addr)
    }
    
    /// Get device memory statistics
    pub fn get_device_memory_stats(&self) -> (usize, usize, usize) {
        (self.device_memory_start, self.device_memory_size, self.device_memory_used)
    }
    
    /// Display detailed memory layout information
    pub fn display(&self) {
        console_println!("=== Dynamic Memory Layout ===");
        console_println!("Kernel Image:");
        console_println!("  Start:  0x{:08x}", self.kernel_start);
        console_println!("  End:    0x{:08x}", self.kernel_end);
        console_println!("  Size:   {} KB", self.kernel_size / 1024);
        
        console_println!("Kernel Stack:");
        console_println!("  Start:  0x{:08x}", self.stack_start);
        console_println!("  End:    0x{:08x}", self.stack_end);
        console_println!("  Size:   {} KB", self.stack_size / 1024);
        
        console_println!("Safety Guards:");
        console_println!("  Kernel guard: {} KB", self.kernel_guard_size / 1024);
        console_println!("  Stack guard:  {} KB", self.stack_guard_size / 1024);
        
        console_println!("Heap Layout:");
        console_println!("  Buddy:   0x{:08x} - 0x{:08x} ({} KB)",
            self.buddy_heap_start, self.buddy_heap_start + self.buddy_heap_size,
            self.buddy_heap_size / 1024);
        
        console_println!("  Small:   0x{:08x} - 0x{:08x} ({} KB)",
            self.small_heap_start, self.small_heap_start + self.small_heap_size,
            self.small_heap_size / 1024);
        
        console_println!("Total kernel footprint: {} KB",
            (self.small_heap_start + self.small_heap_size - self.kernel_start) / 1024);
        
        console_println!("   Heap start: 0x{:08x}", self.heap_start);
        console_println!("   Heap size: {} KB", self.heap_size / 1024);
        console_println!("   Available memory: {} MB", self.available_memory / (1024 * 1024));
        console_println!("   Device memory: 0x{:08x} - 0x{:08x} ({} KB)", 
                        self.device_memory_start, 
                        self.device_memory_start + self.device_memory_size,
                        self.device_memory_size / 1024);
    }
}

/// Global memory layout instance
static mut MEMORY_LAYOUT: Option<MemoryLayout> = None;

/// Initialize and get the global memory layout
pub fn get_memory_layout() -> &'static MemoryLayout {
    unsafe {
        if MEMORY_LAYOUT.is_none() {
            MEMORY_LAYOUT = Some(MemoryLayout::detect());
            if let Some(ref layout) = MEMORY_LAYOUT {
                if let Err(e) = layout.validate() {
                    panic!("Invalid memory layout: {}", e);
                }
            }
        }
        MEMORY_LAYOUT.as_ref().unwrap()
    }
}

/// Get kernel information for debugging
pub fn get_kernel_info() -> (usize, usize, usize) {
    let layout = get_memory_layout();
    (layout.kernel_start, layout.kernel_end, layout.kernel_size)
}

 