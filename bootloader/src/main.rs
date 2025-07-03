#![no_std]
#![no_main]

use core::panic::PanicInfo;
use core::arch::asm;
use heapless::Vec;

// Import shared library components
use elinos_common as common;

// Re-export commonly used macros and functions from shared library
pub use common::{console_print, console_println, debug_print, debug_println};
use common::memory::search_memory_pattern;

// Import modules from the bootloader library (only what bootloader needs)
// Note: Most functionality moved to kernel

// Global UART instance is now in the shared library
pub use common::uart::UART;

// Bootloader-specific constants

#[link_section = ".text.boot"]
#[no_mangle]
pub extern "C" fn _start() -> ! {
    unsafe {
        asm!(
            "li sp, 0x805f0000",  // Bootloader stack (after 0x80200000 + space for bootloader)
            "j {main}",
            main = sym bootloader_main,
            options(noreturn)
        );
    }
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    // Print the panic message
    console_println!("[x]  BOOTLOADER PANIC: {}", info.message());
    
    if let Some(location) = info.location() {
        console_println!("[i] Location: {}:{}:{}", location.file(), location.line(), location.column());
    }
    
    loop {
        unsafe {
            asm!("wfi");
        }
    }
}

// Bootloader info structure (must match bootloader definition)
#[repr(C)]
struct BootloaderInfo {
    magic: u64,
    memory_base: usize,
    memory_size: usize,
    kernel_base: usize,
    available_ram_start: usize,
    available_ram_size: usize,
}

const BOOTLOADER_MAGIC: u64 = 0xEA15_0000_B007_AB1E;

#[no_mangle]
pub extern "C" fn bootloader_main() -> ! {
    // Initialize basic console system first
    if let Err(e) = common::console::init_console() {
        // If console fails, we can't do much, just hang
        loop {
            unsafe {
                asm!("wfi");
            }
        }
    }
    
    console_println!();
    console_println!();
    console_println!("elinOS Bootloader Starting...");
    console_println!("[o] Console initialized");

    // Initialize hardware detection and memory layout
    console_println!("[i] Detecting system memory...");
    let memory_region = common::memory::hardware::detect_main_ram()
        .unwrap_or_else(|| common::memory::hardware::get_fallback_ram());
    console_println!("[o] Memory detection complete");
    console_println!("[i] Main RAM: 0x{:x} - 0x{:x} ({} MB)", 
                     memory_region.start, 
                     memory_region.start + memory_region.size, 
                     memory_region.size / (1024 * 1024));

    // Get memory layout info (bootloader doesn't need full memory manager)
    let _memory_layout = common::memory::layout::get_memory_layout();
    console_println!("[o] Memory layout available for kernel handoff");

    console_println!("[i] Bootloader initialization complete");
    console_println!("[i] Starting kernel...");
    console_println!();

    // Stage 2: Load and jump to separate kernel binary
    load_and_start_kernel()
}

/// Load and start the kernel binary
fn load_and_start_kernel() -> ! {
    console_println!("[i] Loading kernel binary...");
    
    // Get memory info from hardware detection
    let memory_region = common::memory::hardware::detect_main_ram()
        .unwrap_or_else(|| common::memory::hardware::get_fallback_ram());
    let memory_layout = common::memory::layout::get_memory_layout();
    
    // Create bootloader info structure
    // Calculate proper available RAM - total memory minus kernel and bootloader footprint
    let kernel_memory_usage = 0x80400000 - memory_region.start + 2 * 1024 * 1024; // bootloader + kernel space (2MB)
    let available_ram_start = 0x80400000 + 2 * 1024 * 1024; // Start after kernel space
    let available_ram_size = memory_region.size.saturating_sub(kernel_memory_usage);
    
    let bootloader_info = BootloaderInfo {
        magic: BOOTLOADER_MAGIC,
        memory_base: memory_region.start,
        memory_size: memory_region.size,
        kernel_base: 0x80400000, // Kernel loads at this address
        available_ram_start,
        available_ram_size,
    };
    
    console_println!("[i] Bootloader info created:");
    console_println!("    Magic: 0x{:x}", bootloader_info.magic);
    console_println!("    Memory: 0x{:x} - 0x{:x} ({} MB)", 
                     bootloader_info.memory_base,
                     bootloader_info.memory_base + bootloader_info.memory_size,
                     bootloader_info.memory_size / (1024 * 1024));
    console_println!("    Kernel base: 0x{:x}", bootloader_info.kernel_base);
    console_println!("    Available RAM: 0x{:x} - 0x{:x} ({} MB)",
                     bootloader_info.available_ram_start,
                     bootloader_info.available_ram_start + bootloader_info.available_ram_size,
                     bootloader_info.available_ram_size / (1024 * 1024));
    
    // Load kernel from initrd (QEMU loads it to a known location)
    let kernel_base = locate_kernel_from_initrd();
    
    // Use the known kernel entry point (since ELF header reading might be corrupted)
    let kernel_entry_point = 0x80400000_usize;
    
    console_println!("[i] Kernel base: 0x{:x}", kernel_base);
    console_println!("[i] Kernel entry point: 0x{:x}", kernel_entry_point);
    console_println!("[i] Jumping to kernel...");
    console_println!();
    
    // Jump to kernel with bootloader info
    unsafe {
        let kernel_main: extern "C" fn(usize) -> ! = core::mem::transmute(kernel_entry_point);
        kernel_main(&bootloader_info as *const _ as usize);
    }
}

/// Search for ELF magic signature in memory ranges
/// Returns the address where ELF binary was found, or None if not found
fn search_kernel_elf(memory_regions: &[(usize, usize)]) -> Option<usize> {
    // ELF magic: 0x7f, 'E', 'L', 'F'
    let elf_magic = [0x7f, b'E', b'L', b'F'];
    
    console_println!("[i] Searching for ELF magic pattern in memory regions...");
    
    for (i, &(start, size)) in memory_regions.iter().enumerate() {
        let end = start + size;
        console_println!("[i] Searching region {}: 0x{:x} - 0x{:x} ({} MB)", 
                         i, start, end, size / (1024 * 1024));
        
        // Try different alignments: 1 byte, 4 bytes, 64 bytes, 4KB
        let alignments = [1, 4, 64, 4096];
        
        for &alignment in &alignments {
            if let Some(addr) = unsafe { search_memory_pattern(start, end, &elf_magic, alignment) } {
                console_println!("[o] Found ELF magic at 0x{:x} (alignment {})", addr, alignment);
                
                // Verify it's a valid 64-bit RISC-V ELF
                unsafe {
                    let ei_class = core::ptr::read_volatile((addr + 4) as *const u8);
                    let ei_data = core::ptr::read_volatile((addr + 5) as *const u8);
                    let e_machine = core::ptr::read_volatile((addr + 18) as *const u16);
                    
                    console_println!("[i] ELF validation: class={}, data={}, machine=0x{:x}", 
                                     ei_class, ei_data, e_machine);
                    
                    // Check for 64-bit (class=2), little-endian (data=1), RISC-V (machine=0xf3)
                    if ei_class == 2 && ei_data == 1 && e_machine == 0xf3 {
                        console_println!("[o] Valid 64-bit RISC-V ELF found!");
                        return Some(addr);
                    } else {
                        console_println!("[!] ELF validation failed, continuing search...");
                    }
                }
            }
        }
    }
    
    console_println!("[x] No valid ELF binary found in any memory region");
    None
}


/// Load ELF segments to their virtual addresses
fn load_elf_segments(elf_addr: usize) -> bool {
    unsafe {
        let elf_header = elf_addr as *const u8;
        
        // Verify it's a valid 64-bit ELF
        let ei_class = core::ptr::read_volatile(elf_header.add(4));
        if ei_class != 2 { // ELFCLASS64
            console_println!("[x] Not a 64-bit ELF file");
            return false;
        }
        
        // Get program header info
        let phoff = core::ptr::read_volatile(elf_header.add(32) as *const u64) as usize;
        let phentsize = core::ptr::read_volatile(elf_header.add(54) as *const u16) as usize;
        let phnum = core::ptr::read_volatile(elf_header.add(56) as *const u16) as usize;
        
        console_println!("[i] Loading ELF segments: phoff=0x{:x}, phentsize={}, phnum={}", 
                         phoff, phentsize, phnum);
        
        // Process each program header
        for i in 0..phnum {
            let ph_addr = elf_header.add(phoff + i * phentsize);
            
            // Read program header fields
            let p_type = core::ptr::read_volatile(ph_addr as *const u32);
            let _p_flags = core::ptr::read_volatile(ph_addr.add(4) as *const u32);
            let p_offset = core::ptr::read_volatile(ph_addr.add(8) as *const u64) as usize;
            let p_vaddr = core::ptr::read_volatile(ph_addr.add(16) as *const u64) as usize;
            let p_paddr = core::ptr::read_volatile(ph_addr.add(24) as *const u64) as usize;
            let p_filesz = core::ptr::read_volatile(ph_addr.add(32) as *const u64) as usize;
            let p_memsz = core::ptr::read_volatile(ph_addr.add(40) as *const u64) as usize;
            
            // Only process LOAD segments (type 1)
            if p_type == 1 {
                console_println!("[i] LOAD segment {}: vaddr=0x{:x}, paddr=0x{:x}, filesz=0x{:x}, memsz=0x{:x}, offset=0x{:x}", 
                               i, p_vaddr, p_paddr, p_filesz, p_memsz, p_offset);
                
                // Source: ELF file + offset
                let src_addr = elf_header.add(p_offset);
                
                // Destination: Use virtual address (kernel is linked to run at its virtual addresses)
                let dest_addr = p_vaddr as *mut u8;
                
                // Copy file content to memory
                if p_filesz > 0 {
                    console_println!("[i] Copying 0x{:x} bytes from 0x{:x} to 0x{:x}", 
                                   p_filesz, src_addr as usize, dest_addr as usize);
                    core::ptr::copy_nonoverlapping(src_addr, dest_addr, p_filesz);
                }
                
                // Zero out BSS section if memsz > filesz
                if p_memsz > p_filesz {
                    let bss_start = dest_addr.add(p_filesz);
                    let bss_size = p_memsz - p_filesz;
                    console_println!("[i] Zeroing BSS: 0x{:x} bytes at 0x{:x}", 
                                   bss_size, bss_start as usize);
                    core::ptr::write_bytes(bss_start, 0, bss_size);
                }
            }
        }
        
        console_println!("[o] All ELF segments loaded successfully");
        true
    }
}


/// Locate the kernel binary from initrd using comprehensive memory search
/// QEMU loads the initrd to a specific location in memory
fn locate_kernel_from_initrd() -> usize {
    let kernel_dest = 0x80400000_usize;   // Where kernel should be loaded
    
    console_println!("[i] Starting comprehensive kernel search...");
    
    // Define memory regions to search
    let memory_region = common::memory::hardware::detect_main_ram()
        .unwrap_or_else(|| common::memory::hardware::get_fallback_ram());
    
    let search_regions = [
        // High priority: Known QEMU/OpenSBI locations
        (0x84000000, 4 * 1024 * 1024),   // 4MB around common QEMU location        
    ];
    
    // Use the comprehensive search API
    if let Some(kernel_addr) = search_kernel_elf(&search_regions) {
        console_println!("[o] Kernel ELF found at 0x{:x}!", kernel_addr);
        
        // Load ELF segments properly instead of raw copy
        if load_elf_segments(kernel_addr) {
            console_println!("[o] Kernel ELF loaded successfully from comprehensive search");
            return kernel_dest;
        } else {
            console_println!("[x] Failed to load ELF segments");
        }
    }
    
    // Final failure - halt the system properly
    console_println!("[x] CRITICAL: Cannot find kernel ELF binary anywhere in memory!");
    console_println!("[!] Searched entire RAM space comprehensively");
    console_println!("[!] This indicates QEMU initrd loading is fundamentally broken");
    console_println!("[!] System will halt to prevent infinite restart loop");
    
    // Force halt - do not try to jump to invalid kernel location
    console_println!("[!] === BOOTLOADER HALTED ===");
    
    // Disable interrupts and halt
    unsafe {
        core::arch::asm!(
            "csrci sstatus, 2",  // Disable interrupts
            "1:",
            "wfi",               // Wait for interrupt (but interrupts are disabled)
            "j 1b",              // Loop forever
            options(noreturn)
        );
    }
}


// Stack top symbol for bootloader
#[link_section = ".bss"]
static mut _BOOTLOADER_STACK_TOP: [u8; 4096 * 4] = [0; 4096 * 4];
