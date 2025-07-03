//! VirtIO block device implementation

pub mod device;

// Re-export main types
pub use device::{RustVmmVirtIOBlock, VirtioBlkReq, VIRTIO_BLK};

// Re-export initialization functions
pub use device::{init_virtio_blk, init_with_address};

// Block device specific constants
pub const VIRTIO_BLK_T_IN: u32 = 0;     // Read
pub const VIRTIO_BLK_T_OUT: u32 = 1;    // Write  
pub const VIRTIO_BLK_T_FLUSH: u32 = 4;  // Flush
pub const VIRTIO_BLK_S_OK: u8 = 0;      // Success
pub const VIRTIO_BLK_S_IOERR: u8 = 1;   // I/O error
pub const VIRTIO_BLK_S_UNSUPP: u8 = 2;  // Unsupported

pub const VIRTIO_BLK_REQUEST_QUEUE_IDX: u16 = 0; 