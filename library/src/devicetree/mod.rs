// Device Tree parsing module for elinOS
// Provides capability to parse Flattened Device Tree (FDT) blobs
// for hardware discovery on real RISC-V boards

pub mod parser;
pub mod node;
pub mod property;
pub mod memory;
pub mod sbi_integration;

pub use parser::DeviceTreeParser;
pub use node::{DeviceTreeNode, NodeIterator};
pub use property::{DeviceTreeProperty, PropertyType};
pub use memory::DeviceTreeMemoryRegion;
pub use sbi_integration::*;

// FDT magic number in big-endian format
pub const FDT_MAGIC: u32 = 0xd00dfeed;

// FDT version we support
pub const FDT_SUPPORTED_VERSION: u32 = 17;

// Common device tree error types
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DeviceTreeError {
    InvalidMagic,
    UnsupportedVersion,
    InvalidOffset,
    InvalidString,
    NodeNotFound,
    PropertyNotFound,
    BufferTooSmall,
    InvalidAlignment,
}

pub type DeviceTreeResult<T> = Result<T, DeviceTreeError>;