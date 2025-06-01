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
    let parts: heapless::Vec<&str, 8> = command.split_whitespace().collect();
    
    if parts.is_empty() {
        return;
    }

    match parts[0] {
        "help" => {
            let mut uart = UART.lock();
            let _ = writeln!(uart, "Available commands:");
            let _ = writeln!(uart, "  help     - Show this help");
            let _ = writeln!(uart, "  memory   - Show memory information");
            let _ = writeln!(uart, "  devices  - Probe for VirtIO devices");
            let _ = writeln!(uart, "  ls       - List files");
            let _ = writeln!(uart, "  cat <file> - Show file contents");
            let _ = writeln!(uart, "  touch <file> - Create empty file");
            let _ = writeln!(uart, "  rm <file> - Delete file");
            let _ = writeln!(uart, "  clear    - Clear screen");
        },
        "memory" => {
            let mem_mgr = memory::MEMORY_MANAGER.lock();
            let mut uart = UART.lock();
            let _ = writeln!(uart, "Memory regions:");
            for (i, region) in mem_mgr.get_memory_info().iter().enumerate() {
                let _ = writeln!(uart, "  Region {}: 0x{:x} - 0x{:x} ({} MB) {}",
                    i,
                    region.start,
                    region.start + region.size,
                    region.size / (1024 * 1024),
                    if region.is_ram { "RAM" } else { "MMIO" }
                );
            }
        },
        "devices" => {
            virtio_blk::probe_virtio_devices();
        },
        "ls" => {
            filesystem::cmd_ls();
        },
        "cat" => {
            if parts.len() > 1 {
                filesystem::cmd_cat(parts[1]);
            } else {
                let mut uart = UART.lock();
                let _ = writeln!(uart, "Usage: cat <filename>");
            }
        },
        "touch" => {
            if parts.len() > 1 {
                filesystem::cmd_touch(parts[1]);
            } else {
                let mut uart = UART.lock();
                let _ = writeln!(uart, "Usage: touch <filename>");
            }
        },
        "rm" => {
            if parts.len() > 1 {
                filesystem::cmd_rm(parts[1]);
            } else {
                let mut uart = UART.lock();
                let _ = writeln!(uart, "Usage: rm <filename>");
            }
        },
        "clear" => {
            let mut uart = UART.lock();
            let _ = write!(uart, "\x1b[2J\x1b[H"); // ANSI escape codes to clear screen
        },
        _ => {
            let mut uart = UART.lock();
            let _ = writeln!(uart, "Unknown command: {}. Type 'help' for available commands.", parts[0]);
        }
    }
}

#[no_mangle]
pub extern "C" fn main() -> ! {
    // Initialize UART
    let mut uart = UART.lock();
    uart.init();
    
    // Send test message
    let _ = write!(uart, "\n\n");
    drop(uart);
    
    // Initialize dynamic memory detection
    {
        let mut mem_mgr = memory::MEMORY_MANAGER.lock();
        mem_mgr.init();
        
        // Display detected memory regions
        let mut uart = UART.lock();
        let _ = write!(uart, "\nDetected Memory Regions:\n");
        for (i, region) in mem_mgr.get_memory_info().iter().enumerate() {
            let _ = write!(uart, "Region {}: 0x{:x} - 0x{:x} ({} MB) {}\n",
                i,
                region.start,
                region.start + region.size,
                region.size / (1024 * 1024),
                if region.is_ram { "RAM" } else { "MMIO" }
            );
        }
        drop(uart);
    }
    
    // Initialize VirtIO devices
    virtio_blk::probe_virtio_devices();
    
    // Initialize filesystem
    filesystem::init_filesystem();
    
    {
        let mut uart = UART.lock();
        let _ = write!(uart, "\n\nWelcome to ElinOS\n");
        let _ = write!(uart, "Starting shell... Type 'help' for commands.\n\n");
        drop(uart);
    }
    
    // Initialize command buffer
    let mut buffer = [0u8; 256];
    
    loop {
        // Simple shell loop
        let mut uart = UART.lock();
        let _ = write!(uart, "elinOS> ");
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