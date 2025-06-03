#![no_std]
#![no_main]
#![feature(panic_info_message)]

extern crate rlibc;

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
pub mod filesystem;
pub mod elf;
pub mod syscall;
pub mod virtio_block;

use crate::uart::Uart;

// Global UART instance
pub static UART: Mutex<Uart> = Mutex::new(Uart::new());

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    // Try to use console first, fallback to UART
    if let Some(location) = info.location() {
        console_println!("💥 KERNEL PANIC!");
        console_println!("Location: {}:{}", location.file(), location.line());
    } else {
        console_println!("💥 KERNEL PANIC at unknown location!");
    }
    
    let message = info.message();
    console_println!("Message: {}", message);
    
    console_println!("System halted.");
    
    // Fallback to UART if console fails
    {
        let mut uart = UART.lock();
        let _ = writeln!(uart, "💥 KERNEL PANIC!");
        if let Some(location) = info.location() {
            let _ = writeln!(uart, "Location: {}:{}", location.file(), location.line());
        }
        let _ = writeln!(uart, "Message: {}", message);
        let _ = writeln!(uart, "System halted.");
    }
    
    // Halt the system
    loop {
        unsafe {
            core::arch::asm!("wfi");  // Wait for interrupt
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

    // Initialize VirtIO block devices
    console_println!("🔌 Initializing VirtIO block devices...");
    {
        let mut virtio_mgr = virtio_block::VIRTIO_MANAGER.lock();
        if let Err(e) = virtio_mgr.init() {
            console_println!("❌ VirtIO initialization failed: {}", e);
        } else {
            console_println!("✅ VirtIO devices ready");
        }
    }

    // Initialize filesystem
    console_println!("🗂️  Initializing filesystem...");
    if let Err(e) = filesystem::init_filesystem() {
        console_println!("❌ Filesystem initialization failed: {}", e);
        panic!("💥 CRITICAL: Filesystem initialization failed - {}", e);
    } else {
        console_println!("✅ Filesystem ready");
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
    console_println!("A RISC-V64 Educational Operating System");
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