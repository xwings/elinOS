#![no_std]

//! elinOS Common Library
//! 
//! Shared components between bootloader and kernel

use core::fmt::Write;

pub mod sbi;
pub mod uart;
pub mod console;
pub mod memory;
pub mod elf;
pub mod devicetree;
pub mod virtio;
pub mod filesystem;

// Re-export commonly used items
pub use sbi::*;
pub use uart::Uart;
pub use console::*;
pub use elf::*;
pub use devicetree::*;