#![no_std]
#![no_main]

use core::panic::PanicInfo;
use core::fmt::Write;
use core::arch::asm;
use spin::Mutex;
use heapless;

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

    // Initialize filesystem
    console_println!("🗂️ Initializing filesystem...");
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
    // Use dynamic command buffer size based on detected memory
    let command_buffer_size = crate::memory::get_optimal_buffer_size(crate::memory::BufferUsage::Command);
    
    // Use heapless::Vec since we're in no_std environment
    let mut command_buffer = heapless::Vec::<u8, 1024>::new();
    command_buffer.resize(command_buffer_size.min(1024), 0).ok();
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
                        command_buffer[buffer_pos] = 0;
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
        if buffer_pos < command_buffer.len() {
            command_buffer[buffer_pos] = 0;
        }
        
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