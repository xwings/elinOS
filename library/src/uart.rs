// Minimal UART Driver for RISC-V
// Simple implementation for early boot and debugging only

use core::fmt::{self, Write};
use spin::Mutex;

// UART memory-mapped register addresses for QEMU virt machine
pub const UART_BASE: usize = 0x10000000;

pub struct Uart {
    base_addr: usize,
}

impl Uart {
    pub const fn new() -> Self {
        Uart {
            base_addr: UART_BASE,
        }
    }

    pub fn init(&self) {
        // Minimal UART initialization for QEMU
        // QEMU's UART is already mostly configured by firmware
    }

    // Write a single character (minimal implementation)
    pub fn putchar(&self, ch: u8) {
        unsafe {
            let ptr = self.base_addr as *mut u8;
            // Simple write - QEMU handles the rest
            ptr.write_volatile(ch);
        }
    }

    // Read a single character (blocking)
    pub fn getc(&self) -> u8 {
        unsafe {
            let ptr = self.base_addr as *mut u8;
            // Simple polling read
            loop {
                let status = ptr.add(5).read_volatile();
                if status & 1 != 0 {
                    return ptr.read_volatile();
                }
            }
        }
    }

    // Try to read a character (non-blocking)
    pub fn getchar(&self) -> Option<u8> {
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
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for byte in s.bytes() {
            self.putchar(byte);
        }
        Ok(())
    }
}

// Global UART instance
pub static UART: Mutex<Uart> = Mutex::new(Uart::new()); 