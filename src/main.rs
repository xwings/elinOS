#![no_std]
#![no_main]
#![feature(panic_info_message)]

use core::panic::PanicInfo;
use core::fmt::Write;
use core::arch::asm;
use spin::Mutex;

// Module declarations
pub mod console;
pub mod uart;
pub mod commands;
pub mod sbi;
pub mod memory;
pub mod filesystem;  // Now points to filesystem/mod.rs
pub mod elf;
pub mod syscall;
pub mod virtio_blk;

use crate::uart::Uart;

// Global UART instance
pub static UART: Mutex<Uart> = Mutex::new(Uart::new());

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    // Print the panic message
    println!("KERNEL PANIC: {}", info.message());
    
    if let Some(location) = info.location() {
        println!("  at {}:{}:{}", location.file(), location.line(), location.column());
    }
    
    loop {
        unsafe {
            asm!("wfi");
        }
    }
}

// Add missing panic functions for no_std environment
#[no_mangle]
extern "C" fn rust_begin_unwind(_info: &PanicInfo) -> ! {
    panic(_info)
}

#[no_mangle]
pub extern "C" fn _Unwind_Resume() -> ! {
    loop {}
}

#[no_mangle]
pub extern "C" fn __rust_start_panic(_payload: usize) -> u32 {
    0
}

// Add the missing panic_nounwind_fmt function
#[no_mangle]
extern "C" fn rust_panic_nounwind_fmt() -> ! {
    println!("KERNEL PANIC: panic_nounwind_fmt called");
    loop {
        unsafe {
            asm!("wfi");
        }
    }
}

// Add the specific function that the linker is looking for
#[no_mangle]
pub extern "C" fn _ZN4core9panicking19panic_nounwind_fmt17h53f76bdb9f05922fE() -> ! {
    println!("KERNEL PANIC: core::panicking::panic_nounwind_fmt");
    loop {
        unsafe {
            asm!("wfi");
        }
    }
}

#[link_section = ".text.boot"]
#[no_mangle]
pub extern "C" fn _start() -> ! {
    unsafe {
        asm!(
            "la sp, {stack_top}",
            "li t0, 0x80200000",
            "mv sp, t0",
            "j {main}",
            stack_top = sym _STACK_TOP,
            main = sym main,
            options(noreturn)
        );
    }
}

#[no_mangle]
pub extern "C" fn main() -> ! {
    // Initialize UART first (for debugging output)
    {
        let mut uart = UART.lock();
        uart.init();
        let _ = writeln!(uart, "\n🚀 elinOS Starting...");
    }

    // Initialize console system
    if let Err(e) = console::init_console() {
        panic!("Failed to initialize console: {}", e);
    }
    
    // Now use console macros for output
    console_println!("✅ Console system initialized");
    
    // Initialize memory management
    console_println!("🧠 Initializing memory management...");
    {
        let mut memory_mgr = memory::MEMORY_MANAGER.lock();
        memory_mgr.init();
    }
    console_println!("✅ Memory management ready");

    // Initialize VirtIO disk interface
    console_println!("💾 Initializing VirtIO disk...");
    if let Err(e) = virtio_blk::init_virtio_blk() {
        console_println!("❌ VirtIO disk initialization failed: {}", e);
    } else {
        console_println!("✅ VirtIO disk ready");
    }

    // SKIP filesystem initialization for now to isolate the hang
    console_println!("⚠️ Skipping filesystem initialization to debug hang");
    
    // But let's try a minimal test to see where exactly it hangs
    console_println!("🧪 Testing minimal filesystem operations...");
    
    // Test 1: Try to read boot sector (we know this works)
    console_println!("🔍 Test 1: Reading boot sector...");
    {
        let mut disk_device = virtio_blk::VIRTIO_BLK.lock();
        let mut buffer = [0u8; 512];
        match disk_device.read_blocks(0, &mut buffer) {
            Ok(()) => {
                console_println!("✅ Boot sector read successful");
            }
            Err(e) => {
                console_println!("❌ Boot sector read failed: {:?}", e);
            }
        }
    }
    
    // Test 2: Try to read root directory sector (this might hang)
    console_println!("🔍 Test 2: Reading root directory sector 2080...");
    {
        let mut disk_device = virtio_blk::VIRTIO_BLK.lock();
        let mut buffer = [0u8; 512];
        match disk_device.read_blocks(2080, &mut buffer) {
            Ok(()) => {
                console_println!("✅ Root directory sector read successful");
                console_println!("🔍 First 16 bytes: {:02x?}", &buffer[0..16]);
                
                // Let's check if there's any non-zero data in this sector
                let mut has_data = false;
                for i in 0..512 {
                    if buffer[i] != 0 {
                        has_data = true;
                        break;
                    }
                }
                console_println!("🔍 Sector 2080 has data: {}", has_data);
                
                // Let's also check the boot sector to understand the FAT32 layout
                match disk_device.read_blocks(0, &mut buffer) {
                    Ok(()) => {
                        console_println!("📊 Boot sector analysis:");
                        console_println!("  Signature: 0x{:02x}{:02x}", buffer[511], buffer[510]);
                        
                        // Parse key FAT32 fields
                        let sectors_per_cluster = buffer[13];
                        let reserved_sectors = u16::from_le_bytes([buffer[14], buffer[15]]);
                        let num_fats = buffer[16];
                        let sectors_per_fat = u32::from_le_bytes([buffer[36], buffer[37], buffer[38], buffer[39]]);
                        let root_cluster = u32::from_le_bytes([buffer[44], buffer[45], buffer[46], buffer[47]]);
                        
                        console_println!("  Sectors per cluster: {}", sectors_per_cluster);
                        console_println!("  Reserved sectors: {}", reserved_sectors);
                        console_println!("  Number of FATs: {}", num_fats);
                        console_println!("  Sectors per FAT: {}", sectors_per_fat);
                        console_println!("  Root cluster: {}", root_cluster);
                        
                        // Calculate the actual root directory sector
                        let fat_start = reserved_sectors as u32;
                        let data_start = fat_start + (num_fats as u32 * sectors_per_fat);
                        let actual_root_sector = data_start + ((root_cluster - 2) * sectors_per_cluster as u32);
                        
                        console_println!("  FAT starts at sector: {}", fat_start);
                        console_println!("  Data starts at sector: {}", data_start);
                        console_println!("  Calculated root sector: {} (was using {})", actual_root_sector, 2080);
                        
                        // Try reading the calculated root directory
                        if actual_root_sector != 2080 {
                            match disk_device.read_blocks(actual_root_sector as u64, &mut buffer) {
                                Ok(()) => {
                                    console_println!("✅ Actual root directory sector read successful");
                                    console_println!("🔍 First 32 bytes: {:02x?}", &buffer[0..32]);
                                    
                                    // Check for directory entries
                                    if buffer[0] != 0 && buffer[0] != 0xE5 {
                                        console_println!("🎉 Found potential directory entry!");
                                        let name_bytes = &buffer[0..8];
                                        let ext_bytes = &buffer[8..11];
                                        console_println!("  Name: {:?}", name_bytes);
                                        console_println!("  Ext: {:?}", ext_bytes);
                                        console_println!("  Attributes: 0x{:02x}", buffer[11]);
                                    }
                                }
                                Err(e) => {
                                    console_println!("❌ Failed to read calculated root sector: {:?}", e);
                                }
                            }
                        }
                    }
                    Err(e) => {
                        console_println!("❌ Failed to read boot sector: {:?}", e);
                    }
                }
            }
            Err(e) => {
                console_println!("❌ Root directory sector read failed: {:?}", e);
            }
        }
    }
    
    console_println!("✅ Minimal filesystem tests complete");
    
    // Now let's try a minimal filesystem initialization that just does the essentials
    console_println!("🔍 Testing minimal filesystem initialization...");
    
    // Try to do filesystem init with minimal logging to avoid potential console buffer issues
    match filesystem::init_filesystem() {
        Ok(()) => {
            console_println!("✅ Filesystem initialization successful!");
        }
        Err(e) => {
            console_println!("❌ Filesystem initialization failed: {:?}", e);
        }
    }
    
    console_println!("🎉 elinOS initialization complete!");
    console_println!();
    
    // Show welcome message and enter shell
    show_welcome();
    shell_loop();
}

// === WELCOME MESSAGE ===
fn show_welcome() {
    console_println!("=====================================");
    console_println!("       🦀 Welcome to elinOS! 🦀      ");
    console_println!("=====================================");
    console_println!("A RISC-V64 Experimental Operating System");
    console_println!("Written in Rust for learning purposes");
    console_println!();
    console_println!("Type 'help' for available commands");
    console_println!("Type 'version' for system information");
    console_println!("Type 'memory' for memory layout");
    console_println!("Type 'shutdown' to exit");
    console_println!();
}

// === INTERACTIVE SHELL ===
fn shell_loop() -> ! {
    let mut command_buffer = [0u8; 256];
    let mut buffer_pos = 0;
    
    loop {
        // Show prompt
        console_print!("elinOS> ");
        
        // Read command character by character
        buffer_pos = 0;
        loop {
            let ch = read_char();
            
            match ch {
                b'\r' | b'\n' => {
                    console_println!();
                    break;
                }
                b'\x08' | b'\x7f' => {  // Backspace or DEL
                    if buffer_pos > 0 {
                        buffer_pos -= 1;
                        console_print!("\x08 \x08");  // Move back, print space, move back
                    }
                }
                b' '..=b'~' => {  // Printable ASCII
                    if buffer_pos < command_buffer.len() - 1 {
                        command_buffer[buffer_pos] = ch;
                        buffer_pos += 1;
                        console_print!("{}", ch as char);
                    }
                }
                _ => {
                    // Ignore other characters
                }
            }
        }
        
        // Null-terminate the command
        command_buffer[buffer_pos] = 0;
        
        // Convert to string and execute
        if buffer_pos > 0 {
            let command_str = core::str::from_utf8(&command_buffer[..buffer_pos])
                .unwrap_or("invalid");
            
            execute_command(command_str);
        }
        
        console_println!();
    }
}

fn read_char() -> u8 {
    let uart = UART.lock();
    uart.getc()
}

fn execute_command(command: &str) {
    let trimmed = command.trim();
    
    if trimmed.is_empty() {
        return;
    }
    
    commands::process_command(trimmed);
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ({
        use core::fmt::Write;
        let mut uart = $crate::UART.lock();
        let _ = write!(uart, $($arg)*);
    });
}

#[macro_export]
macro_rules! println {
    () => ($crate::print!("\r\n"));
    ($fmt:expr) => ($crate::print!(concat!($fmt, "\r\n")));
    ($fmt:expr, $($arg:tt)*) => ($crate::print!(concat!($fmt, "\r\n"), $($arg)*));
}

// Stack top symbol
#[link_section = ".bss"]
static mut _STACK_TOP: [u8; 4096 * 4] = [0; 4096 * 4];