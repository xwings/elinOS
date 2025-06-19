// Dynamic Memory Layout Manager for elinOS
// Replaces hardcoded memory allocations with intelligent detection

use crate::sbi;
use core::fmt::Write;
use crate::{UART, console_println};
use heapless::Vec;

// Linker-provided symbols (defined in linker script)
extern "C" {
    static __text_start: u8;      // Start of kernel text section
    static __text_end: u8;        // End of kernel text section  
    static __rodata_start: u8;    // Start of read-only data
    static __rodata_end: u8;      // End of read-only data
    static __data_start: u8;      // Start of data section
    static __data_end: u8;        // End of data section
    static __bss_start: u8;       // Start of BSS section
    static __bss_end: u8;         // End of BSS section (end of kernel)
    static __stack_bottom: u8;    // Bottom of kernel stack
    static __stack_top: u8;       // Top of kernel stack
}

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
    
    // Safety margins
    pub kernel_guard_size: usize,
    pub stack_guard_size: usize,
    
    // Memory regions
    pub regions: Vec<crate::memory::MemoryRegion, 8>,
    pub heap_size: usize,
    pub available_memory: usize,
}

impl MemoryLayout {
    /// Calculate memory layout dynamically from linker symbols
    pub fn detect() -> Self {
        console_println!("[i] Detecting memory layout via OpenSBI...");
        
        // Calculate kernel boundaries
        let kernel_start = unsafe { &__text_start as *const _ as usize };
        let kernel_end = unsafe { &__bss_end as *const _ as usize };
        let kernel_size = kernel_end - kernel_start;
        
        let stack_start = unsafe { &__stack_bottom as *const _ as usize };
        let stack_end = unsafe { &__stack_top as *const _ as usize };
        let stack_size = stack_end - stack_start;
        
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
            kernel_guard_size: 4096,
            stack_guard_size: 4096,
            regions: Vec::new(),
            heap_size: 0,
            available_memory: 0,
        };
        
        // Use OpenSBI to get memory information
        let (base, size) = sbi::get_memory_info();
        
        if size > 0 {
            layout.add_region(base, size, true, crate::memory::MemoryZone::Normal);
            console_println!("[o] Detected {} MB RAM at 0x{:x}", size / (1024 * 1024), base);
        } else {
            // Fallback to default QEMU layout
            layout.add_region(0x80000000, 128 * 1024 * 1024, true, crate::memory::MemoryZone::Normal);
            console_println!("[!]  Using fallback memory layout: 128MB at 0x80000000");
        }
        
        // Add standard MMIO regions
        layout.add_region(0x10000000, 0x1000, false, crate::memory::MemoryZone::DMA); // UART
        layout.add_region(0x02000000, 0x10000, false, crate::memory::MemoryZone::DMA); // CLINT
        layout.add_region(0x0c000000, 0x400000, false, crate::memory::MemoryZone::DMA); // PLIC
        
        // Set up heap areas after kernel
        let heap_start = kernel_end + layout.kernel_guard_size;
        layout.heap_start = heap_start;
        layout.buddy_heap_start = heap_start;
        layout.buddy_heap_size = 256 * 1024; // 256KB for buddy allocator
        layout.small_heap_start = layout.buddy_heap_start + layout.buddy_heap_size;
        layout.small_heap_size = 64 * 1024; // 64KB for small allocator
        layout.heap_size = layout.buddy_heap_size + layout.small_heap_size;
        
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
    pub fn add_region(&mut self, start: usize, size: usize, is_ram: bool, zone_type: crate::memory::MemoryZone) {
        let region = crate::memory::MemoryRegion {
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
        
        // Check for reasonable sizes
        if self.kernel_size > 32 * 1024 * 1024 {
            return Err("Kernel size unreasonably large (>32MB)");
        }
        
        if self.total_kernel_footprint > 64 * 1024 * 1024 {
            return Err("Total kernel footprint too large (>64MB)");
        }
        
        console_println!("[o] Memory layout validation passed");
        Ok(())
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

 