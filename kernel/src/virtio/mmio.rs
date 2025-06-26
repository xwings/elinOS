//! VirtIO MMIO register definitions and constants
//! Based on VirtIO 1.1 specification

// === VIRTIO MMIO REGISTER OFFSETS ===
pub const VIRTIO_MMIO_MAGIC_VALUE: usize = 0x000;      // 0x74726976
pub const VIRTIO_MMIO_VERSION: usize = 0x004;          // Version (1=legacy, 2=modern)
pub const VIRTIO_MMIO_DEVICE_ID: usize = 0x008;        // Device ID (2=block)
pub const VIRTIO_MMIO_VENDOR_ID: usize = 0x00c;        // Vendor ID
pub const VIRTIO_MMIO_DEVICE_FEATURES: usize = 0x010;  // Device features
pub const VIRTIO_MMIO_DEVICE_FEATURES_SEL: usize = 0x014; // Device features select
pub const VIRTIO_MMIO_DRIVER_FEATURES: usize = 0x020;  // Driver features
pub const VIRTIO_MMIO_DRIVER_FEATURES_SEL: usize = 0x024; // Driver features select
pub const VIRTIO_MMIO_GUEST_PAGE_SIZE: usize = 0x028;  // Guest page size (legacy only)
pub const VIRTIO_MMIO_QUEUE_SEL: usize = 0x030;        // Queue select
pub const VIRTIO_MMIO_QUEUE_NUM_MAX: usize = 0x034;    // Queue size max
pub const VIRTIO_MMIO_QUEUE_NUM: usize = 0x038;        // Queue size
pub const VIRTIO_MMIO_QUEUE_ALIGN: usize = 0x03c;      // Queue alignment (legacy only)
pub const VIRTIO_MMIO_QUEUE_PFN: usize = 0x040;        // Queue PFN (legacy only)
pub const VIRTIO_MMIO_QUEUE_READY: usize = 0x044;      // Queue ready
pub const VIRTIO_MMIO_QUEUE_NOTIFY: usize = 0x050;     // Queue notify
pub const VIRTIO_MMIO_INTERRUPT_STATUS: usize = 0x060; // Interrupt status
pub const VIRTIO_MMIO_INTERRUPT_ACK: usize = 0x064;    // Interrupt acknowledge
pub const VIRTIO_MMIO_STATUS: usize = 0x070;           // Device status
pub const VIRTIO_MMIO_QUEUE_DESC_LOW: usize = 0x080;   // Queue descriptor low
pub const VIRTIO_MMIO_QUEUE_DESC_HIGH: usize = 0x084;  // Queue descriptor high
pub const VIRTIO_MMIO_QUEUE_DRIVER_LOW: usize = 0x090; // Queue driver low
pub const VIRTIO_MMIO_QUEUE_DRIVER_HIGH: usize = 0x094; // Queue driver high
pub const VIRTIO_MMIO_QUEUE_DEVICE_LOW: usize = 0x0a0; // Queue device low
pub const VIRTIO_MMIO_QUEUE_DEVICE_HIGH: usize = 0x0a4; // Queue device high
pub const VIRTIO_MMIO_CONFIG: usize = 0x100;           // Configuration space

// === VIRTIO STATUS BITS ===
pub const VIRTIO_STATUS_ACKNOWLEDGE: u32 = 1;
pub const VIRTIO_STATUS_DRIVER: u32 = 2;
pub const VIRTIO_STATUS_DRIVER_OK: u32 = 4;
pub const VIRTIO_STATUS_FEATURES_OK: u32 = 8;
pub const VIRTIO_STATUS_DEVICE_NEEDS_RESET: u32 = 64;
pub const VIRTIO_STATUS_FAILED: u32 = 128;

// === MEMORY CONSTANTS ===
pub const PAGE_SIZE: usize = 4096;

/// Align up to the next page boundary
pub fn align_up(size: usize) -> usize {
    (size + PAGE_SIZE - 1) & !(PAGE_SIZE - 1)
}

// === VIRTQ DESCRIPTOR FLAGS ===
pub const VIRTQ_DESC_F_NEXT: u16 = 1;       // This descriptor continues via next field
pub const VIRTQ_DESC_F_WRITE: u16 = 2;      // Device writes to this descriptor
pub const VIRTQ_DESC_F_INDIRECT: u16 = 4;   // Points to indirect table

// === VIRTQ RING FLAGS ===
pub const VIRTQ_AVAIL_F_NO_INTERRUPT: u16 = 1;
pub const VIRTQ_USED_F_NO_NOTIFY: u16 = 1; 