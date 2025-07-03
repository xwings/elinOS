#![no_std]
#![no_main]


// Import shared library components
use elinos_common as common;

// Re-export commonly used macros and functions from shared library
pub use common::{console_print, console_println, debug_print, debug_println};

// Minimal bootloader modules - most functionality moved to kernel
// Use shared memory management from library
pub use common::memory;

// Global UART instance is now in the shared library
pub use common::uart::UART;

