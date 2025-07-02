// Hardware memory detection for elinOS
// Shared between bootloader and kernel

use crate::sbi;
use super::regions::{MemoryRegion, MemoryZone};

// Linker-provided symbols (defined in linker script)
extern "C" {
    pub static __text_start: u8;      // Start of kernel text section
    pub static __text_end: u8;        // End of kernel text section  
    pub static __rodata_start: u8;    // Start of read-only data
    pub static __rodata_end: u8;      // End of read-only data
    pub static __data_start: u8;      // Start of data section
    pub static __data_end: u8;        // End of data section
    pub static __bss_start: u8;       // Start of BSS section
    pub static __bss_end: u8;         // End of BSS section (end of kernel)
    pub static __stack_bottom: u8;    // Bottom of kernel stack
    pub static __stack_top: u8;       // Top of kernel stack
}

/// Get basic kernel boundaries from linker symbols
pub fn get_kernel_boundaries() -> (usize, usize, usize) {
    let kernel_start = unsafe { &__text_start as *const _ as usize };
    let kernel_end = unsafe { &__bss_end as *const _ as usize };
    let kernel_size = kernel_end - kernel_start;
    (kernel_start, kernel_end, kernel_size)
}

/// Get stack boundaries from linker symbols  
pub fn get_stack_boundaries() -> (usize, usize, usize) {
    let stack_start = unsafe { &__stack_bottom as *const _ as usize };
    let stack_end = unsafe { &__stack_top as *const _ as usize };
    let stack_size = stack_end - stack_start;
    (stack_start, stack_end, stack_size)
}

/// Detect main RAM using OpenSBI
pub fn detect_main_ram() -> Option<MemoryRegion> {
    let (base, size) = sbi::get_memory_info();
    
    if size > 0 {
        Some(MemoryRegion::new(base, size, true, MemoryZone::Normal))
    } else {
        None
    }
}

/// Get fallback memory layout for QEMU
pub fn get_fallback_ram() -> MemoryRegion {
    MemoryRegion::new(0x80000000, 128 * 1024 * 1024, true, MemoryZone::Normal)
}

/// Get standard MMIO regions for RISC-V QEMU
pub fn get_standard_mmio_regions() -> [MemoryRegion; 3] {
    [
        MemoryRegion::new(0x10000000, 0x1000, false, MemoryZone::DMA),    // UART
        MemoryRegion::new(0x02000000, 0x10000, false, MemoryZone::DMA),   // CLINT  
        MemoryRegion::new(0x0c000000, 0x400000, false, MemoryZone::DMA),  // PLIC
    ]
}

/// Calculate safe heap start address after kernel with guard
pub fn calculate_heap_start(kernel_end: usize, guard_size: usize) -> usize {
    kernel_end + guard_size
}

/// Validate memory layout for safety
pub fn validate_memory_layout(
    kernel_start: usize, 
    kernel_end: usize, 
    heap_start: usize
) -> Result<(), &'static str> {
    // Check kernel size is reasonable
    let kernel_size = kernel_end - kernel_start;
    if kernel_size > 32 * 1024 * 1024 {
        return Err("Kernel size unreasonably large (>32MB)");
    }
    
    // Check heap doesn't overlap with kernel
    if heap_start <= kernel_end {
        return Err("Heap overlaps with kernel space");
    }
    
    // Check for reasonable alignment
    if kernel_start % 4096 != 0 {
        return Err("Kernel not properly aligned");
    }
    
    Ok(())
}