#![no_std]
#![no_main]

use core::panic::PanicInfo;
use core::fmt::Write;
use core::arch::asm;
use spin::Mutex;

// Memory layout constants
const UART0: usize = 0x10000000;
const KERNEL_START: usize = 0x80200000;
const KERNEL_SIZE: usize = 2 * 1024 * 1024;  // 2MB
const HEAP_START: usize = 0x80400000;
const HEAP_SIZE: usize = 64 * 1024 * 1024;   // 64MB
const STACK_SIZE: usize = 2 * 1024 * 1024;   // 2MB per hart
const MAX_HARTS: usize = 4;

// Memory management structure
#[allow(dead_code)]
struct MemoryManager {
    heap_start: usize,
    heap_end: usize,
    current_heap: usize,
}

#[allow(dead_code)]
impl MemoryManager {
    const fn new() -> Self {
        MemoryManager {
            heap_start: HEAP_START,
            heap_end: HEAP_START + HEAP_SIZE,
            current_heap: HEAP_START,
        }
    }

    fn allocate(&mut self, size: usize) -> Option<usize> {
        let aligned_size = (size + 7) & !7;  // 8-byte alignment
        if self.current_heap + aligned_size > self.heap_end {
            None
        } else {
            let ptr = self.current_heap;
            self.current_heap += aligned_size;
            Some(ptr)
        }
    }
}

#[allow(dead_code)]
static MEMORY_MANAGER: Mutex<MemoryManager> = Mutex::new(MemoryManager::new());

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

static UART: Mutex<Uart> = Mutex::new(Uart::new());

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

#[no_mangle]
pub extern "C" fn main() -> ! {
    // Initialize UART
    let mut uart = UART.lock();
    uart.init();
    
    // Send test message
    let _ = write!(uart, "\n\nWelcome to ElinOS\n");
    let _ = write!(uart, "Memory Layout:\n");
    let _ = write!(uart, "Kernel: 0x{:x} - 0x{:x}\n", KERNEL_START, KERNEL_START + KERNEL_SIZE);
    let _ = write!(uart, "Heap: 0x{:x} - 0x{:x}\n", HEAP_START, HEAP_START + HEAP_SIZE);
    let _ = write!(uart, "Stack: 0x{:x} - 0x{:x}\n", HEAP_START + HEAP_SIZE, HEAP_START + HEAP_SIZE + (STACK_SIZE * MAX_HARTS));
    let _ = write!(uart, "Starting shell...\n\n");
    
    // Initialize command buffer
    let mut buffer = [0u8; 256];
    
    loop {
        // Simple shell loop
        let _ = write!(uart, "> ");
        let mut i = 0;
        
        while i < buffer.len() {
            if let Some(c) = uart.getchar() {
                if c == b'\r' || c == b'\n' {
                    uart.putchar(b'\n');
                    break;
                }
                uart.putchar(c);
                buffer[i] = c;
                i += 1;
            }
        }
        
        // Echo back the command
        if i > 0 {
            let _ = write!(uart, "You typed: ");
            for j in 0..i {
                uart.putchar(buffer[j]);
            }
            uart.putchar(b'\n');
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