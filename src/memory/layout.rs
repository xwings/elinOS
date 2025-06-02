// Dynamic Memory Layout Manager for elinKernel
// Replaces hardcoded memory allocations with intelligent detection

use crate::sbi;
use core::fmt::Write;
use crate::UART;

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
#[derive(Debug, Clone, Copy)]
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
}

impl MemoryLayout {
    /// Calculate memory layout dynamically from linker symbols
    pub fn detect() -> Self {
        // Safe fallback values in case linker symbols are invalid
        let mut kernel_start = 0x80200000;  // Standard RISC-V kernel start
        let mut kernel_end = 0x80400000;    // Conservative 2MB kernel
        let mut kernel_size = 2 * 1024 * 1024;
        
        let mut stack_start = 0x80400000;
        let mut stack_end = 0x80500000;
        let mut stack_size = 1024 * 1024;
        
        // Try to get real linker symbols, but use fallbacks if they're invalid
        unsafe {
            let text_start = &__text_start as *const u8 as usize;
            let bss_end = &__bss_end as *const u8 as usize;
            let stack_bottom = &__stack_bottom as *const u8 as usize;
            let stack_top = &__stack_top as *const u8 as usize;
            
            // Validate linker symbols are reasonable
            if text_start >= 0x80000000 && text_start < 0x90000000 && 
               bss_end > text_start && bss_end < 0x90000000 {
                kernel_start = text_start;
                kernel_end = bss_end;
                kernel_size = kernel_end - kernel_start;
            }
            
            if stack_bottom >= 0x80000000 && stack_bottom < 0x90000000 &&
               stack_top > stack_bottom && stack_top < 0x90000000 {
                stack_start = stack_bottom;
                stack_end = stack_top;
                stack_size = stack_end - stack_start;
            }
        }
        
        // Calculate safety margins (16KB each for safety)
        let kernel_guard_size = 16 * 1024;  // 16KB guard after kernel
        let stack_guard_size = 16 * 1024;   // 16KB guard after stack
        
        // Total kernel footprint including guards
        let total_kernel_footprint = kernel_size + stack_size + kernel_guard_size + stack_guard_size;
        
        // Align to page boundaries (4KB)
        let aligned_footprint = (total_kernel_footprint + 4095) & !4095;
        
        // Calculate heap layout based on available memory
        let memory_regions = sbi::get_memory_regions();
        let mut heap_start = 0x80400000;  // Fallback heap start
        let mut total_available = 120 * 1024 * 1024;  // Fallback 120MB
        
        // Try to get real memory info
        if memory_regions.count > 0 {
            for i in 0..memory_regions.count {
                let region = &memory_regions.regions[i];
                if (region.flags & 1) != 0 && region.size > aligned_footprint { // RAM region
                    heap_start = region.start + aligned_footprint;
                    total_available = region.size - aligned_footprint;
                    break;
                }
            }
        }
        
        // Distribute memory intelligently based on available space
        let (buddy_size, small_size) = Self::calculate_heap_distribution(total_available);
        
        let buddy_heap_start = heap_start;
        let small_heap_start = buddy_heap_start + buddy_size;
        
        MemoryLayout {
            kernel_start,
            kernel_end,
            kernel_size,
            stack_start,
            stack_end, 
            stack_size,
            total_kernel_footprint: aligned_footprint,
            heap_start,
            buddy_heap_start,
            buddy_heap_size: buddy_size,
            small_heap_start,
            small_heap_size: small_size,
            kernel_guard_size,
            stack_guard_size,
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
        // Check for overlaps
        if self.heap_start <= self.kernel_end + self.kernel_guard_size {
            return Err("Heap overlaps with kernel space");
        }
        
        // Check for reasonable sizes
        if self.kernel_size > 32 * 1024 * 1024 {
            return Err("Kernel size unreasonably large (>32MB)");
        }
        
        if self.total_kernel_footprint > 64 * 1024 * 1024 {
            return Err("Total kernel footprint too large (>64MB)");
        }
        
        Ok(())
    }
    
    /// Display detailed memory layout information
    pub fn display(&self) {
        let mut uart = UART.lock();
        
        let _ = writeln!(uart, "=== Dynamic Memory Layout ===");
        let _ = writeln!(uart, "Kernel Image:");
        let _ = writeln!(uart, "  Start:  0x{:08x}", self.kernel_start);
        let _ = writeln!(uart, "  End:    0x{:08x}", self.kernel_end);
        let _ = writeln!(uart, "  Size:   {} KB", self.kernel_size / 1024);
        
        let _ = writeln!(uart, "Kernel Stack:");
        let _ = writeln!(uart, "  Start:  0x{:08x}", self.stack_start);
        let _ = writeln!(uart, "  End:    0x{:08x}", self.stack_end);
        let _ = writeln!(uart, "  Size:   {} KB", self.stack_size / 1024);
        
        let _ = writeln!(uart, "Safety Guards:");
        let _ = writeln!(uart, "  Kernel guard: {} KB", self.kernel_guard_size / 1024);
        let _ = writeln!(uart, "  Stack guard:  {} KB", self.stack_guard_size / 1024);
        
        let _ = writeln!(uart, "Professional Heap Layout:");
        let _ = writeln!(uart, "  Buddy:   0x{:08x} - 0x{:08x} ({} KB)",
            self.buddy_heap_start,
            self.buddy_heap_start + self.buddy_heap_size, 
            self.buddy_heap_size / 1024);
        let _ = writeln!(uart, "  Small:   0x{:08x} - 0x{:08x} ({} KB)",
            self.small_heap_start,
            self.small_heap_start + self.small_heap_size,
            self.small_heap_size / 1024);
        
        let _ = writeln!(uart, "Total kernel footprint: {} KB", 
            self.total_kernel_footprint / 1024);
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