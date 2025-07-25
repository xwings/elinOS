#![no_std]
#![no_main]

use core::panic::PanicInfo;
use core::arch::asm;

// Import shared library components
use elinos_common as common;
use common::elf::{ElfLoader, Elf64Phdr};
use common::filesystem::FileSystem;

// Re-export commonly used macros and functions from shared library
pub use common::{console_print, console_println, debug_print, debug_println};

// Import modules from the bootloader library (only what bootloader needs)
// Note: Most functionality moved to kernel

// Global UART instance is now in the shared library
pub use common::uart::UART;

// Bootloader-specific ELF loader implementation
struct BootloaderElfLoader;

impl ElfLoader for BootloaderElfLoader {
    type Error = ();

    fn load_segment(&self, phdr: &Elf64Phdr, data: &[u8]) -> Result<(), Self::Error> {
        unsafe {
            // Source: ELF file + offset
            let src_addr = data.as_ptr().add(phdr.p_offset as usize);
            
            // Destination: Use virtual address (kernel is linked to run at its virtual addresses)
            let dest_addr = phdr.p_vaddr as *mut u8;
            
            console_println!("[i] LOAD segment: vaddr=0x{:x}, paddr=0x{:x}, filesz=0x{:x}, memsz=0x{:x}, offset=0x{:x}", 
                           phdr.p_vaddr, phdr.p_paddr, phdr.p_filesz, phdr.p_memsz, phdr.p_offset);
            
            // Copy file content to memory
            if phdr.p_filesz > 0 {
                console_println!("[i] Copying 0x{:x} bytes from 0x{:x} to 0x{:x}", 
                               phdr.p_filesz, src_addr as usize, dest_addr as usize);
                core::ptr::copy_nonoverlapping(src_addr, dest_addr, phdr.p_filesz as usize);
            }
            
            // Zero out BSS section if memsz > filesz
            if phdr.p_memsz > phdr.p_filesz {
                let bss_start = dest_addr.add(phdr.p_filesz as usize);
                let bss_size = phdr.p_memsz - phdr.p_filesz;
                console_println!("[i] Zeroing BSS: 0x{:x} bytes at 0x{:x}", 
                               bss_size, bss_start as usize);
                core::ptr::write_bytes(bss_start, 0, bss_size as usize);
            }
        }
        Ok(())
    }
}

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
    
    // Load kernel from SD card
    let kernel_base = locate_kernel_from_sdcard();
    
    // Use the known kernel entry point (since ELF header reading might be corrupted)
    let kernel_entry_point = 0x80400000_usize;
    
    console_println!("[i] Kernel base: 0x{:x}", kernel_base);
    console_println!("[i] Kernel entry point: 0x{:x}", kernel_entry_point);
    
    // Copy bootloader info to a safe memory location that kernel can access
    // Use memory just before the kernel base as temporary storage
    let safe_bootloader_info_addr = 0x803FF000_usize; // Just before kernel at 0x80400000
    unsafe {
        core::ptr::write(safe_bootloader_info_addr as *mut BootloaderInfo, bootloader_info);
    }
    
    console_println!("[i] Jumping to kernel...");
    console_println!();
    
    // Jump to kernel with bootloader info
    unsafe {
        // Use inline assembly to jump to kernel more directly
        asm!(
            "jr {kernel_entry}",
            kernel_entry = in(reg) kernel_entry_point,
            in("a0") safe_bootloader_info_addr,
            options(noreturn)
        );
    }
}



/// Locate the kernel binary from SD card using VirtIO and ext2
fn locate_kernel_from_sdcard() -> usize {
    let kernel_dest = 0x80400000_usize;   // Where kernel should be loaded
    
    console_println!("[i] Loading kernel from SD card...");
    
    // Initialize VirtIO memory management
    if let Err(e) = common::virtio::init_virtio_memory() {
        console_println!("[x] Failed to initialize VirtIO memory: {:?}", e);
        halt_system();
    }
    
    // Initialize VirtIO block device
    if let Err(e) = common::virtio::init_virtio_blk() {
        console_println!("[x] Failed to initialize VirtIO block device: {:?}", e);
        halt_system();
    }
    
    console_println!("[o] VirtIO block device initialized");
    
    // Initialize storage manager
    if let Err(e) = common::virtio::init_storage() {
        console_println!("[x] Failed to initialize storage manager: {:?}", e);
        halt_system();
    }
    
    console_println!("[o] Storage manager initialized");
    
    // Initialize ext2 filesystem
    let mut ext2_fs = common::filesystem::Ext2FileSystem::new();
    if let Err(e) = ext2_fs.init() {
        console_println!("[x] Failed to initialize ext2 filesystem: {:?}", e);
        halt_system();
    }
    
    console_println!("[o] ext2 filesystem mounted");
    
    // Read kernel file
    let kernel_content = match ext2_fs.read_file("/kernel") {
        Ok(content) => content,
        Err(e) => {
            console_println!("[x] Failed to read /kernel file: {:?}", e);
            halt_system();
        }
    };
    
    console_println!("[o] Kernel file read successfully ({} bytes)", kernel_content.len());
    
    // Copy kernel to destination address
    let kernel_slice = kernel_content.as_slice();
    if kernel_slice.len() == 0 {
        console_println!("[x] Kernel file is empty");
        halt_system();
    }
    
    // Validate ELF header
    if !common::elf::ElfUtils::validate_elf_header(kernel_slice) {
        console_println!("[x] Invalid ELF header in kernel file");
        halt_system();
    }
    
    // Load ELF segments
    if !load_elf_segments_from_buffer(kernel_slice) {
        console_println!("[x] Failed to load ELF segments from kernel file");
        halt_system();
    }
    
    console_println!("[o] Kernel loaded successfully from SD card");
    kernel_dest
}

/// Load ELF segments from a buffer
fn load_elf_segments_from_buffer(elf_data: &[u8]) -> bool {
    // Validate ELF header
    if !common::elf::ElfUtils::validate_elf_header(elf_data) {
        console_println!("[x] Invalid ELF header");
        return false;
    }
    
    // Get ELF header
    let header = match common::elf::ElfUtils::get_header(elf_data) {
        Some(h) => h,
        None => {
            console_println!("[x] Failed to get ELF header");
            return false;
        }
    };
    
    console_println!("[i] Loading ELF segments: phoff=0x{:x}, phentsize={}, phnum={}", 
                     header.e_phoff, header.e_phentsize, header.e_phnum);
    
    let loader = BootloaderElfLoader;
    
    // Process each program header
    for i in 0..header.e_phnum {
        if let Some(phdr) = common::elf::ElfUtils::get_program_header(elf_data, header, i as usize) {
            // Only process LOAD segments
            if common::elf::ElfUtils::is_loadable_segment(&phdr) {
                if loader.load_segment(&phdr, elf_data).is_err() {
                    console_println!("[x] Failed to load segment {}", i);
                    return false;
                }
            }
        }
    }
    
    console_println!("[o] All ELF segments loaded successfully");
    true
}

/// Halt the system with proper error handling
fn halt_system() -> ! {
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
