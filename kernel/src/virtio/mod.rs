//! VirtIO device support for elinOS
//! 
//! This module provides a modular implementation of VirtIO devices,
//! starting with block devices but designed to be extensible for
//! network, console, and other VirtIO device types.

pub mod mmio;
pub mod queue;
pub mod error;
pub mod block;

// Re-export commonly used types
pub use error::{DiskError, DiskResult};
pub use queue::{VirtqDesc, VirtqAvail, VirtqUsed, VirtqUsedElem, VirtioQueue};
pub use block::{RustVmmVirtIOBlock, VIRTIO_BLK};

// Re-export initialization functions for backward compatibility
pub use block::{init_virtio_blk, init_with_address}; 