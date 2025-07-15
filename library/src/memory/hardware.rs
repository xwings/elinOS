// Hardware memory detection for elinOS
// Shared between bootloader and kernel

use crate::sbi;
use crate::devicetree::*;
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

/// Detect main RAM using device tree or OpenSBI fallback
pub fn detect_main_ram() -> Option<MemoryRegion> {
    // Try device tree first if available
    if let Some(region) = detect_main_ram_from_dt() {
        return Some(region);
    }
    
    // Fallback to SBI
    let (base, size) = sbi::get_memory_info();
    
    if size > 0 {
        Some(MemoryRegion::new(base, size, true, MemoryZone::Normal))
    } else {
        None
    }
}

/// Try to detect main RAM from device tree
fn detect_main_ram_from_dt() -> Option<MemoryRegion> {
    // This would need the DTB address passed from boot protocol
    // For now, return None to use SBI fallback
    // TODO: Get DTB address from boot protocol and parse memory regions
    None
}

/// Get fallback memory layout for QEMU - use smallest safe default
pub fn get_fallback_ram() -> MemoryRegion {
    // Use smallest reasonable default - can be expanded if more memory is available
    let min_ram = 32 * 1024 * 1024; // 32MB minimum for basic operation
    MemoryRegion::new(0x80000000, min_ram, true, MemoryZone::Normal)
}

/// Get fallback memory for specific system types
pub fn get_fallback_ram_for_system(system_type: SystemType) -> MemoryRegion {
    let ram_size = match system_type {
        SystemType::Minimal => 16 * 1024 * 1024,   // 16MB for very minimal systems
        SystemType::QEMU => 128 * 1024 * 1024,     // 128MB for QEMU default
        SystemType::Hardware => 64 * 1024 * 1024,  // 64MB conservative for real hardware
    };
    MemoryRegion::new(0x80000000, ram_size, true, MemoryZone::Normal)
}

/// System type for dynamic memory configuration
#[derive(Debug, Clone, Copy)]
pub enum SystemType {
    Minimal,    // Very constrained environment
    QEMU,       // QEMU virtual machine
    Hardware,   // Real hardware
}

/// Get standard MMIO regions for RISC-V (device tree aware)
pub fn get_standard_mmio_regions() -> [MemoryRegion; 3] {
    // Try to get MMIO regions from device tree
    if let Some(regions) = get_mmio_regions_from_dt() {
        return regions;
    }
    
    // Fallback to standard QEMU MMIO layout
    [
        MemoryRegion::new(0x10000000, 0x1000, false, MemoryZone::DMA),    // UART
        MemoryRegion::new(0x02000000, 0x10000, false, MemoryZone::DMA),   // CLINT  
        MemoryRegion::new(0x0c000000, 0x400000, false, MemoryZone::DMA),  // PLIC
    ]
}

/// Try to get MMIO regions from device tree
fn get_mmio_regions_from_dt() -> Option<[MemoryRegion; 3]> {
    // This would parse device tree to find UART, CLINT, PLIC addresses
    // For now, return None to use standard layout
    // TODO: Parse device tree for MMIO region discovery
    None
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

/// Search for a byte pattern in a memory range
/// Returns the address where the pattern was found, or None if not found
/// 
/// # Safety
/// This function performs raw memory access and should only be used with valid memory ranges
pub unsafe fn search_memory_pattern(start_addr: usize, end_addr: usize, pattern: &[u8], alignment: usize) -> Option<usize> {
    let mut current_addr = start_addr;
    
    // Align the start address
    current_addr = (current_addr + alignment - 1) & !(alignment - 1);
    
    while current_addr + pattern.len() <= end_addr {
        let mut found = true;
        
        // Check if pattern matches at current address
        for (i, &expected_byte) in pattern.iter().enumerate() {
            let actual_byte = core::ptr::read_volatile((current_addr + i) as *const u8);
            if actual_byte != expected_byte {
                found = false;
                break;
            }
        }
        
        if found {
            return Some(current_addr);
        }
        
        current_addr += alignment;
    }
    
    None
}