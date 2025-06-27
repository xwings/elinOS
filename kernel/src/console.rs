// Console Management System for elinOS
// Framebuffer-focused with minimal UART fallback

use core::fmt::{self, Write};
use spin::Mutex;
use lazy_static::lazy_static;

// === CONSOLE MACROS ===

#[macro_export]
macro_rules! console_print {
    ($($arg:tt)*) => {{
        let console = $crate::console::CONSOLE_MANAGER.lock();
        let _ = console.print(format_args!($($arg)*));
    }};
}

#[macro_export]
macro_rules! console_println {
    () => {
        $crate::console_print!("\r\n")
    };
    ($($arg:tt)*) => {{
        $crate::console_print!($($arg)*);
        $crate::console_print!("\r\n");
    }};
}

#[macro_export]
macro_rules! debug_print {
    ($($arg:tt)*) => {{
        // Always goes to UART for debugging
        let mut uart = $crate::UART.lock();
        let _ = uart.write_fmt(format_args!($($arg)*));
    }};
}

#[macro_export]
macro_rules! debug_println {
    ($($arg:tt)*) => {{
        $crate::debug_print!($($arg)*);
        $crate::debug_print!("\r\n");
    }};
}

// === SIMPLE OUTPUT DEVICES ===

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum OutputDevice {
    Framebuffer,   // Primary: Text/graphics output (can be redirected to UART in QEMU)
    DebugUart,     // Secondary: Simple UART for debugging only
}

// === MINIMAL CONSOLE MANAGER ===

pub struct ConsoleManager {
    primary_device: OutputDevice,
}

impl ConsoleManager {
    pub const fn new() -> Self {
        ConsoleManager {
            primary_device: OutputDevice::Framebuffer,
        }
    }

    pub fn init(&mut self) -> Result<(), &'static str> {
        // For now, we'll use UART as framebuffer implementation
        // This lets us see output in QEMU terminal while developing framebuffer
        Ok(())
    }

    pub fn print(&self, args: fmt::Arguments) -> fmt::Result {
        match self.primary_device {
            OutputDevice::Framebuffer => {
                // TODO: Implement actual framebuffer text rendering
                // For now, output to UART so we can see it in QEMU terminal
                // This will be replaced with actual framebuffer rendering later
                let mut uart = crate::UART.lock();
                uart.write_fmt(args)
            }
            OutputDevice::DebugUart => {
                let mut uart = crate::UART.lock();
                uart.write_fmt(args)
            }
        }
    }

    pub fn set_primary_device(&mut self, device: OutputDevice) {
        self.primary_device = device;
    }
}

// Global console manager instance
lazy_static! {
    pub static ref CONSOLE_MANAGER: Mutex<ConsoleManager> = Mutex::new(ConsoleManager::new());
}

// === INITIALIZATION ===

pub fn init_console() -> Result<(), &'static str> {
    let mut console = CONSOLE_MANAGER.lock();
    console.init()
}

// === HIGH-LEVEL FUNCTIONS ===

pub fn print(s: &str) {
    let console = CONSOLE_MANAGER.lock();
    let _ = console.print(format_args!("{}", s));
}

pub fn println(s: &str) {
    let console = CONSOLE_MANAGER.lock();
    let _ = console.print(format_args!("{}\r\n", s));
}

pub fn print_to_device(device: OutputDevice, s: &str) {
    // For simplified approach, just use debug output for now
    match device {
        OutputDevice::Framebuffer => {
            let console = CONSOLE_MANAGER.lock();
            let _ = console.print(format_args!("{}", s));
        }
        OutputDevice::DebugUart => {
            let mut uart = crate::UART.lock();
            let _ = uart.write_fmt(format_args!("{}", s));
        }
    }
} 