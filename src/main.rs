#![no_std]
#![no_main]

use core::panic::PanicInfo;
use core::fmt::Write;
use core::arch::asm;
use spin::Mutex;

mod memory;
mod sbi;
mod virtio_blk;
mod filesystem;
mod syscall;
mod commands;
mod elf;

// Memory layout constants (fallback values)
const UART0: usize = 0x10000000;
const KERNEL_START: usize = 0x80200000;

struct Uart {
    base_addr: usize,
}

impl Uart {
    const fn new() -> Self {
        Uart { base_addr: UART0 }
    }

    fn init(&self) {
        unsafe {
            let ptr = self.base_addr as *mut u8;
            // Disable interrupts
            ptr.add(1).write_volatile(0x00);
            // Enable FIFO, clear them, with 14-byte threshold
            ptr.add(2).write_volatile(0xC7);
            // Enable interrupts
            ptr.add(1).write_volatile(0x01);
            // Set baud rate divisor
            ptr.add(3).write_volatile(0x80);
            ptr.add(0).write_volatile(0x01);
            ptr.add(1).write_volatile(0x00);
            ptr.add(3).write_volatile(0x03);
        }
    }

    fn putchar(&self, c: u8) {
        unsafe {
            let ptr = self.base_addr as *mut u8;
            // Wait until UART is ready to receive a byte
            while ptr.add(5).read_volatile() & 0x20 == 0 {}
            ptr.write_volatile(c);
        }
    }

    fn getchar(&self) -> Option<u8> {
        unsafe {
            let ptr = self.base_addr as *mut u8;
            let status = ptr.add(5).read_volatile();
            if status & 1 != 0 {
                Some(ptr.read_volatile())
            } else {
                None
            }
        }
    }
}

impl Write for Uart {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        for c in s.bytes() {
            self.putchar(c);
        }
        Ok(())
    }
}

pub static UART: Mutex<Uart> = Mutex::new(Uart::new());

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
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

fn process_command(command: &str) {
    // Delegate all command processing to the commands module
    commands::process_command(command);
}

#[no_mangle]
pub extern "C" fn main() -> ! {
    // Initialize UART
    let mut uart = UART.lock();
    uart.init();
    
    // Send test message
    let _ = write!(uart, "\n\n");
    drop(uart);
    
    // Initialize memory management
    {
        let mut uart = UART.lock();
        let _ = writeln!(uart, "Initializing memory management...");
    }
    
    {
        let mut mem_mgr = memory::MEMORY_MANAGER.lock();
        mem_mgr.init();
    }
    
    // Initialize VirtIO devices
    virtio_blk::probe_virtio_devices();
    
    // Initialize filesystem
    filesystem::init_filesystem();
    
    {
        let mut uart = UART.lock();
        let _ = write!(uart, "\n\nWelcome to elinKernel\n");
        let _ = write!(uart, "Type 'help' for commands or 'syscall' for system call info.\n\n");
        drop(uart);
    }
    
    // Initialize command buffer
    let mut buffer = [0u8; 256];
    
    loop {
        // Simple shell loop
        let mut uart = UART.lock();
        let _ = write!(uart, "elinKernel> ");
        drop(uart);
        
        let mut i = 0;
        
        while i < buffer.len() {
            let mut uart = UART.lock();
            if let Some(c) = uart.getchar() {
                if c == b'\r' || c == b'\n' {
                    uart.putchar(b'\n');
                    drop(uart);
                    break;
                }
                if c == b'\x08' || c == b'\x7f' { // Backspace
                    if i > 0 {
                        i -= 1;
                        uart.putchar(b'\x08');
                        uart.putchar(b' ');
                        uart.putchar(b'\x08');
                    }
                    drop(uart);
                    continue;
                }
                uart.putchar(c);
                buffer[i] = c;
                i += 1;
                drop(uart);
            } else {
                drop(uart);
            }
        }
        
        // Process the command
        if i > 0 {
            let command = core::str::from_utf8(&buffer[..i]).unwrap_or("");
            process_command(command);
        }
    }
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