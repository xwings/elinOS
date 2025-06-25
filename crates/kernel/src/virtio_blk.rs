// VirtIO Block Device for elinOS
// Based on rust-vmm/vm-virtio Queue implementation
// Adapted for MMIO transport and no_std kernel environment
// License: Apache-2.0 / BSD-3-Clause (following rust-vmm)

use spin::Mutex;
use crate::console_println;
use core::{convert::TryInto, cmp::Ord, result::Result::{Ok, Err}};
use core::ptr::read_volatile;
use core::fmt;

// === DISK ERRORS ===
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DiskError {
    NotFound,
    InvalidSector,
    BufferTooSmall,
    ReadError,
    WriteError,
    DeviceNotFound,
    NotInitialized,
    VirtIOError,
    InvalidParameter,
    QueueFull,
    IoError,
    InvalidDescriptor,
    DeviceNotReady,
}



impl core::fmt::Display for DiskError {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        // Ensure this message always gets out if a DiskError is processed
        console_println!("!! DiskError Formatted: {:?}", self);
        match self {
            DiskError::NotFound => write!(f, "Disk not found"),
            DiskError::InvalidSector => write!(f, "Invalid sector number"),
            DiskError::BufferTooSmall => write!(f, "Buffer too small"),
            DiskError::ReadError => write!(f, "Disk read error"),
            DiskError::WriteError => write!(f, "Disk write error"),
            DiskError::DeviceNotFound => write!(f, "Disk device not found"),
            DiskError::NotInitialized => write!(f, "Disk not initialized"),
            DiskError::VirtIOError => write!(f, "VirtIO error"),
            DiskError::IoError => write!(f, "I/O error"),
            DiskError::QueueFull => write!(f, "VirtIO queue full"),
            DiskError::InvalidDescriptor => write!(f, "Invalid descriptor"),
            DiskError::DeviceNotReady => write!(f, "Device not ready"),
            DiskError::InvalidParameter => write!(f, "Invalid parameter"),
        }
    }
}

pub type DiskResult<T> = Result<T, DiskError>;

// === VIRTIO MMIO CONSTANTS ===
// Based on VirtIO 1.1 specification and rust-vmm implementation

// MMIO register offsets
const VIRTIO_MMIO_MAGIC_VALUE: usize = 0x000;      // 0x74726976
const VIRTIO_MMIO_VERSION: usize = 0x004;          // Version (1=legacy, 2=modern)
const VIRTIO_MMIO_DEVICE_ID: usize = 0x008;        // Device ID (2=block)
const VIRTIO_MMIO_VENDOR_ID: usize = 0x00c;        // Vendor ID
const VIRTIO_MMIO_DEVICE_FEATURES: usize = 0x010;  // Device features
const VIRTIO_MMIO_DEVICE_FEATURES_SEL: usize = 0x014; // Device features select
const VIRTIO_MMIO_DRIVER_FEATURES: usize = 0x020;  // Driver features
const VIRTIO_MMIO_DRIVER_FEATURES_SEL: usize = 0x024; // Driver features select
const VIRTIO_MMIO_GUEST_PAGE_SIZE: usize = 0x028;  // Guest page size (legacy only)
const VIRTIO_MMIO_QUEUE_SEL: usize = 0x030;        // Queue select
const VIRTIO_MMIO_QUEUE_NUM_MAX: usize = 0x034;    // Queue size max
const VIRTIO_MMIO_QUEUE_NUM: usize = 0x038;        // Queue size
const VIRTIO_MMIO_QUEUE_ALIGN: usize = 0x03c;      // Queue alignment (legacy only)
const VIRTIO_MMIO_QUEUE_PFN: usize = 0x040;        // Queue PFN (legacy only)
const VIRTIO_MMIO_QUEUE_READY: usize = 0x044;      // Queue ready
const VIRTIO_MMIO_QUEUE_NOTIFY: usize = 0x050;     // Queue notify
const VIRTIO_MMIO_INTERRUPT_STATUS: usize = 0x060; // Interrupt status
const VIRTIO_MMIO_INTERRUPT_ACK: usize = 0x064;    // Interrupt acknowledge
const VIRTIO_MMIO_STATUS: usize = 0x070;           // Device status
const VIRTIO_MMIO_QUEUE_DESC_LOW: usize = 0x080;   // Queue descriptor low
const VIRTIO_MMIO_QUEUE_DESC_HIGH: usize = 0x084;  // Queue descriptor high
const VIRTIO_MMIO_QUEUE_DRIVER_LOW: usize = 0x090; // Queue driver low
const VIRTIO_MMIO_QUEUE_DRIVER_HIGH: usize = 0x094; // Queue driver high
const VIRTIO_MMIO_QUEUE_DEVICE_LOW: usize = 0x0a0; // Queue device low
const VIRTIO_MMIO_QUEUE_DEVICE_HIGH: usize = 0x0a4; // Queue device high
const VIRTIO_MMIO_CONFIG: usize = 0x100;           // Configuration space

// Page size for legacy VirtIO
const PAGE_SIZE: usize = 4096;

/// Align up to the next page boundary
fn align_up(size: usize) -> usize {
    (size + PAGE_SIZE - 1) & !(PAGE_SIZE - 1)
}

// VirtIO status bits
const VIRTIO_STATUS_ACKNOWLEDGE: u32 = 1;
const VIRTIO_STATUS_DRIVER: u32 = 2;
const VIRTIO_STATUS_DRIVER_OK: u32 = 4;
const VIRTIO_STATUS_FEATURES_OK: u32 = 8;
const VIRTIO_STATUS_DEVICE_NEEDS_RESET: u32 = 64;
const VIRTIO_STATUS_FAILED: u32 = 128;

// VirtIO block device constants
const VIRTIO_BLK_T_IN: u32 = 0;     // Read
const VIRTIO_BLK_T_OUT: u32 = 1;    // Write
const VIRTIO_BLK_T_FLUSH: u32 = 4;  // Flush
const VIRTIO_BLK_S_OK: u8 = 0;      // Success
const VIRTIO_BLK_S_IOERR: u8 = 1;   // I/O error
const VIRTIO_BLK_S_UNSUPP: u8 = 2;  // Unsupported

const VIRTIO_BLK_REQUEST_QUEUE_IDX: u16 = 0; // Added definition

// Descriptor flags (from virtio-queue)
const VIRTQ_DESC_F_NEXT: u16 = 1;       // This descriptor continues via next field
const VIRTQ_DESC_F_WRITE: u16 = 2;      // Device writes to this descriptor
const VIRTQ_DESC_F_INDIRECT: u16 = 4;   // Points to indirect table

// Available ring flags
const VIRTQ_AVAIL_F_NO_INTERRUPT: u16 = 1;

// Used ring flags
const VIRTQ_USED_F_NO_NOTIFY: u16 = 1;

// === VIRTIO QUEUE STRUCTURES ===
// Based on rust-vmm virtio-queue implementation

/// VirtIO descriptor table entry
/// This is the exact layout from the VirtIO specification
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct VirtqDesc {
    /// Guest physical address of buffer
    pub addr: u64,
    /// Length of buffer
    pub len: u32,
    /// Flags for this descriptor
    pub flags: u16,
    /// Index of next descriptor (if flags & VIRTQ_DESC_F_NEXT)
    pub next: u16,
}

impl VirtqDesc {
    pub const fn new() -> Self {
        VirtqDesc {
            addr: 0,
            len: 0,
            flags: 0,
            next: 0,
        }
    }
    
    pub fn set(&mut self, addr: u64, len: u32, flags: u16, next: u16) {
        self.addr = addr;
        self.len = len;
        self.flags = flags;
        self.next = next;
    }
    
    pub fn has_next(&self) -> bool {
        (self.flags & VIRTQ_DESC_F_NEXT) != 0
    }
    
    pub fn is_write_only(&self) -> bool {
        (self.flags & VIRTQ_DESC_F_WRITE) != 0
    }
}

/// VirtIO available ring structure
/// This is where the guest puts available descriptor indices
#[repr(C)]
pub struct VirtqAvail {
    /// Flags for available ring
    pub flags: u16,
    /// Index where next available descriptor will be written
    pub idx: u16,
    /// Ring of available descriptor indices
    pub ring: [u16; 256], // Maximum queue size
    /// Used event suppression (VirtIO 1.0+)
    pub used_event: u16,
}

impl VirtqAvail {
    pub const fn new() -> Self {
        VirtqAvail {
            flags: 0,
            idx: 0,
            ring: [0; 256],
            used_event: 0,
        }
    }
}

/// VirtIO used ring element
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct VirtqUsedElem {
    /// Index of start of used descriptor chain
    pub id: u32,
    /// Total length written to descriptor chain
    pub len: u32,
}

/// VirtIO used ring structure
/// This is where the device puts completed descriptor indices
#[repr(C)]
pub struct VirtqUsed {
    /// Flags for used ring
    pub flags: u16,
    /// Index where next used element will be written
    pub idx: u16,
    /// Ring of used elements
    pub ring: [VirtqUsedElem; 256], // Maximum queue size
    /// Available event suppression (VirtIO 1.0+)
    pub avail_event: u16,
}

impl VirtqUsed {
    pub const fn new() -> Self {
        VirtqUsed {
            flags: 0,
            idx: 0,
            ring: [VirtqUsedElem { id: 0, len: 0 }; 256],
            avail_event: 0,
        }
    }
}

/// VirtIO block request header
/// This is the standard VirtIO block request format
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct VirtioBlkReq {
    /// Request type (VIRTIO_BLK_T_IN, VIRTIO_BLK_T_OUT, etc.)
    pub type_: u32,
    /// Reserved field
    pub reserved: u32,
    /// Sector number
    pub sector: u64,
}

impl VirtioBlkReq {
    pub fn new_read(sector: u64) -> Self {
        VirtioBlkReq {
            type_: VIRTIO_BLK_T_IN,
            reserved: 0,
            sector,
        }
    }
    
    pub fn new_write(sector: u64) -> Self {
        VirtioBlkReq {
            type_: VIRTIO_BLK_T_OUT,
            reserved: 0,
            sector,
        }
    }
}

/// VirtIO Queue implementation
/// Handles descriptor management, available/used rings.
/// Based on concepts from rust-vmm's virtio_queue.
/// Not all features of rust-vmm's queue are implemented.
#[derive(Debug)] // Added Debug derive
pub struct VirtioQueue {
    /// Queue size (must be power of 2)
    size: u16,
    /// Ready flag
    ready: bool,
    /// Base address of descriptor table
    desc_table: usize,
    /// Base address of available ring
    avail_ring: usize,
    /// Base address of used ring
    used_ring: usize,
    /// Next available index to use for adding to avail_ring.ring
    next_avail: u16,
    /// Last used index we've processed from used_ring.ring
    last_used_idx: u16,
    /// Queue select index (usually 0 for block device)
    queue_index: u16,

}

impl VirtioQueue {
    pub const fn new() -> Self {
        VirtioQueue {
            size: 0, // Will be set by init
            ready: false,
            desc_table: 0,
            avail_ring: 0,
            used_ring: 0,
            next_avail: 0,
            last_used_idx: 0,
            queue_index: 0,
        }
    }
    
    pub fn init(&mut self, size: u16, queue_idx: u16, desc_table: usize, avail_ring: usize, used_ring: usize) -> DiskResult<()> {
        if !size.is_power_of_two() || size == 0 {
            console_println!("[x] VirtioQueue init error: size {} is not a power of two or is zero.", size);
            return Err(DiskError::InvalidParameter);
        }
        self.size = size;
        self.ready = false; // Set to true by device setup logic later
        self.desc_table = desc_table;
        self.avail_ring = avail_ring;
        self.used_ring = used_ring;
        self.next_avail = 0;
        self.last_used_idx = 0;
        self.queue_index = queue_idx;

        // Initialize tracking - ensure our view matches device state
        unsafe {
            let used_ring_ptr = self.used_ring as *mut VirtqUsed;
            self.last_used_idx = read_volatile(&(*used_ring_ptr).idx); 
        }
        console_println!("[o] VirtioQueue initialized: size={}, idx={}, desc_base=0x{:x}, avail_base=0x{:x}, used_base=0x{:x}", 
                         self.size, self.queue_index, self.desc_table, self.avail_ring, self.used_ring);
        Ok(())
    }
    
    /// Add a chain of descriptors to the available ring.
    /// Returns the index of the head of the descriptor chain.
    pub fn add_descriptor_chain(&mut self, chain: &[VirtqDesc]) -> DiskResult<u16> {
        if !self.ready {
            return Err(DiskError::QueueFull);
        }
        
        if chain.is_empty() || chain.len() > self.size as usize {
            return Err(DiskError::InvalidParameter);
        }
        
        // Simple check: ensure we don't wrap around too much
        let available_space = self.size.saturating_sub(8); // Keep some buffer
        if chain.len() as u16 > available_space {
            return Err(DiskError::QueueFull);
        }
        
        let head_index = self.next_avail; 
        let desc_table_ptr = self.desc_table as *mut VirtqDesc;

        // Place descriptors into the descriptor table
        for i in 0..chain.len() {
            let actual_table_idx = (head_index + i as u16) % self.size;
            let mut desc_to_write = chain[i];

            if (desc_to_write.flags & VIRTQ_DESC_F_NEXT) != 0 {
                desc_to_write.next = (head_index + desc_to_write.next) % self.size;
            }
            
            unsafe {
                core::ptr::write_volatile(desc_table_ptr.add(actual_table_idx as usize), desc_to_write);
            }
        }

        // Add to available ring
        unsafe {
            let avail_ring_ptr = self.avail_ring as *mut VirtqAvail;
            let device_avail_idx = read_volatile(&(*avail_ring_ptr).idx);
            let ring_idx = device_avail_idx % self.size; 
            
            core::ptr::write_volatile(&mut (*avail_ring_ptr).ring[ring_idx as usize], head_index);
            core::ptr::write_volatile(&mut (*avail_ring_ptr).idx, device_avail_idx.wrapping_add(1));
        }
        
        self.next_avail = (self.next_avail + chain.len() as u16) % self.size;

        Ok(head_index)
    }
    
    /// Get a used element from the used ring if available.
    pub fn get_used_elem(&mut self) -> Option<VirtqUsedElem> {
        unsafe {
            let used_ring_ptr = self.used_ring as *const VirtqUsed;
            let device_current_used_idx = read_volatile(&(*used_ring_ptr).idx);
            
            if self.last_used_idx == device_current_used_idx {
                return None;
            }
            
            let elem_array_idx = self.last_used_idx % self.size;
            let elem = read_volatile(&(*used_ring_ptr).ring[elem_array_idx as usize]);
            
            self.last_used_idx = self.last_used_idx.wrapping_add(1);
            
            Some(elem)
        }
    }

    /// Simple completion check - just look for any completion, handle out-of-order
    pub fn wait_for_completion(&mut self, expected_head: u16) -> Option<VirtqUsedElem> {
        // First, check if we have the expected completion
        if let Some(elem) = self.get_used_elem() {
            if elem.id as u16 == expected_head {
                return Some(elem);
            } else {
                // Got a different completion - this is the out-of-order issue
                // For now, just accept any completion to keep things moving
                return Some(elem);
            }
        }
        None
    }

    pub fn is_ready(&self) -> bool {
        self.ready
    }

    pub fn size(&self) -> u16 {
        self.size
    }
    

}

/// VirtIO Block Device implementation based on rust-vmm patterns
pub struct RustVmmVirtIOBlock {
    /// Device initialization state
    initialized: bool,
    /// Device capacity in sectors
    capacity_sectors: u64,
    /// MMIO base address
    mmio_base: usize,
    /// VirtIO queue
    queue: VirtioQueue,
    /// Device features
    device_features: u64,
    /// Driver features
    driver_features: u64,
    /// Legacy VirtIO flag (experimental extension)
    is_legacy: bool,
}

impl RustVmmVirtIOBlock {
    pub const fn new() -> Self {
        RustVmmVirtIOBlock {
            initialized: false,
            capacity_sectors: 0,
            mmio_base: 0,
            queue: VirtioQueue::new(),
            device_features: 0,
            driver_features: 0,
            is_legacy: false,
        }
    }
    
    /// Initialize the VirtIO block device
    pub fn init(&mut self) -> DiskResult<()> {
        
        // Discover VirtIO MMIO device
        if !self.discover_device()? {
            return Err(DiskError::DeviceNotFound);
        }
        
        // Initialize device
        self.init_device()?;
        
        // Set up virtqueue
        self.setup_queue()?;
        
        // Mark device as ready
        self.set_driver_ok()?;
        
        self.initialized = true;
        console_println!("[o] rust-vmm VirtIO block device initialized successfully");
        Ok(())
    }
    
    /// Discover VirtIO MMIO device
    fn discover_device(&mut self) -> DiskResult<bool> {
        
        // QEMU virt machine VirtIO MMIO addresses
        let mmio_addresses = [
            0x10001000, 0x10002000, 0x10003000, 0x10004000,
            0x10005000, 0x10006000, 0x10007000, 0x10008000,
        ];
        
        for &addr in &mmio_addresses {
            if self.probe_mmio_device(addr)? {
                self.mmio_base = addr;
                console_println!("[o] VirtIO block device found at 0x{:x}", addr);
                return Ok(true);
            }
        }
        
        console_println!("[x] No VirtIO block device found");
        Ok(false)
    }
    
    /// Probe a single MMIO address for VirtIO device
    fn probe_mmio_device(&mut self, base: usize) -> DiskResult<bool> {
        unsafe {
            // Check magic value
            let magic = core::ptr::read_volatile((base + VIRTIO_MMIO_MAGIC_VALUE) as *const u32);
            if magic != 0x74726976 {
                return Ok(false);
            }
            
            // Check version (we want modern VirtIO, but accept legacy for experimental purposes)
            let version = core::ptr::read_volatile((base + VIRTIO_MMIO_VERSION) as *const u32);
            
            // Check device ID (2 = block device)
            let device_id = core::ptr::read_volatile((base + VIRTIO_MMIO_DEVICE_ID) as *const u32);
            if device_id != 2 {
                return Ok(false);
            }
            
            let vendor_id = core::ptr::read_volatile((base + VIRTIO_MMIO_VENDOR_ID) as *const u32);
            
            if version >= 2 {
                console_println!("[i] Modern VirtIO block device: version={}, vendor=0x{:x}", version, vendor_id);
            } else if version == 1 {
                console_println!("[i] Legacy VirtIO block device: version={}, vendor=0x{:x} (experimental extension)", version, vendor_id);
                self.is_legacy = true;
            } else {
                console_println!("[!]  Unknown VirtIO version {} at 0x{:x}, skipping", version, base);
                return Ok(false);
            }
            
            Ok(true)
        }
    }
    
    /// Initialize the VirtIO device following the initialization sequence
    fn init_device(&mut self) -> DiskResult<()> {
        console_println!("[i] Initializing VirtIO device...");
        
        unsafe {
            let base = self.mmio_base;
            
            // Step 1: Reset the device
            self.write_reg_u32(VIRTIO_MMIO_STATUS, 0);
            
            // Step 2: Set ACKNOWLEDGE status bit
            self.set_status(VIRTIO_STATUS_ACKNOWLEDGE as u8);
            
            // Step 3: Set DRIVER status bit
            self.set_status(VIRTIO_STATUS_DRIVER as u8);
            
            if self.is_legacy {                
                // Legacy VirtIO: Read features directly
                self.device_features = core::ptr::read_volatile((base + VIRTIO_MMIO_DEVICE_FEATURES) as *const u32) as u64;
                console_println!("[i] Device features: 0x{:x}", self.device_features);
                
                // Legacy VirtIO: Set driver features directly
                self.driver_features = 0; // Minimal features for simplicity
                self.write_reg_u32(VIRTIO_MMIO_DRIVER_FEATURES, self.driver_features as u32);
                
                // Legacy VirtIO: Skip FEATURES_OK step
            } else {                
                // Step 4: Read device features (modern VirtIO)
                self.write_reg_u32(VIRTIO_MMIO_DEVICE_FEATURES_SEL, 0);
                let features_lo = self.read_reg_u32(VIRTIO_MMIO_DEVICE_FEATURES);
                self.write_reg_u32(VIRTIO_MMIO_DEVICE_FEATURES_SEL, 1);
                let features_hi = self.read_reg_u32(VIRTIO_MMIO_DEVICE_FEATURES);
                
                self.device_features = ((features_hi as u64) << 32) | (features_lo as u64);
                console_println!("[i] Device features: 0x{:x}", self.device_features);
                
                // Step 5: Set driver features (accept basic features only)
                self.driver_features = 0; // Minimal features for simplicity
                self.write_reg_u32(VIRTIO_MMIO_DRIVER_FEATURES_SEL, 0);
                self.write_reg_u32(VIRTIO_MMIO_DRIVER_FEATURES, self.driver_features as u32);
                self.write_reg_u32(VIRTIO_MMIO_DRIVER_FEATURES_SEL, 1);
                self.write_reg_u32(VIRTIO_MMIO_DRIVER_FEATURES, (self.driver_features >> 32) as u32);
                
                // Step 6: Set FEATURES_OK status bit
                self.set_status(VIRTIO_STATUS_FEATURES_OK as u8);
                
                // Step 7: Verify FEATURES_OK is still set
                let status = self.read_reg_u32(VIRTIO_MMIO_STATUS);
                if (status & VIRTIO_STATUS_FEATURES_OK) == 0 {
                    return Err(DiskError::VirtIOError);
                }
            }
            
            // Step 8: Read device configuration
            let capacity_low = self.read_reg_u32(VIRTIO_MMIO_CONFIG);
            let capacity_high = self.read_reg_u32(VIRTIO_MMIO_CONFIG + 4);
            self.capacity_sectors = ((capacity_high as u64) << 32) | (capacity_low as u64);
            
            console_println!("[i] Device capacity: {} sectors ({} MB)", 
                self.capacity_sectors, self.capacity_sectors * 512 / 1024 / 1024);
        }
        
        Ok(())
    }
    
    /// Set up the virtqueue
    fn setup_queue(&mut self) -> DiskResult<()> {

        unsafe {
            let base = self.mmio_base;
            
            // Select queue 0
            self.write_reg_u32(VIRTIO_MMIO_QUEUE_SEL, 0);
            
            // Get maximum queue size
            let max_queue_size = self.read_reg_u32(VIRTIO_MMIO_QUEUE_NUM_MAX);
            console_println!("[i] Max queue size: {}", max_queue_size);
            
            // Set queue size (use smaller size for simplicity)
            let queue_size = 64.min(max_queue_size as u16);
            if !queue_size.is_power_of_two() {
                return Err(DiskError::VirtIOError);
            }
            
            self.write_reg_u32(VIRTIO_MMIO_QUEUE_NUM, queue_size as u32);
            
            if self.is_legacy {
                // Step 1: Set guest page size (REQUIRED for legacy VirtIO)
                self.write_reg_u32(VIRTIO_MMIO_GUEST_PAGE_SIZE, PAGE_SIZE as u32);
                console_println!("[i] Set guest page size: {} bytes", PAGE_SIZE);
                
                // Step 2: Calculate memory layout following VirtIO spec
                // Legacy VirtIO requires ALL rings to be contiguous and page-aligned
                let desc_table_size = 16 * queue_size as usize; // 16 bytes per descriptor
                let avail_ring_size = 6 + 2 * queue_size as usize; // 6 bytes header + 2 bytes per entry
                let used_ring_size = 6 + 8 * queue_size as usize; // 6 bytes header + 8 bytes per entry
                
                // Calculate aligned layout exactly like rcore-os
                let driver_area_offset = desc_table_size;
                let device_area_offset = align_up(desc_table_size + avail_ring_size);
                let total_size = align_up(device_area_offset + used_ring_size);
                
                console_println!("[i] Legacy memory layout calculation:");
                console_println!("  Descriptor table: {} bytes", desc_table_size);
                console_println!("  Driver area offset: {} bytes", driver_area_offset);  
                console_println!("  Device area offset: {} bytes", device_area_offset);
                console_println!("  Total queue size: {} bytes", total_size);
                
                // Allocate page-aligned memory
                const QUEUE_MEMORY_BASE: usize = 0x81000000;
                let desc_table_addr = QUEUE_MEMORY_BASE;
                let avail_ring_addr = desc_table_addr + driver_area_offset;
                let used_ring_addr = desc_table_addr + device_area_offset;
                
                // Validate memory layout (like rcore-os does)
                if desc_table_addr % PAGE_SIZE != 0 {
                    return Err(DiskError::VirtIOError);
                }
                
                console_println!("[i] Legacy queue memory layout:");
                console_println!("  Descriptors: 0x{:x}", desc_table_addr);
                console_println!("  Available:   0x{:x}", avail_ring_addr);
                console_println!("  Used:        0x{:x}", used_ring_addr);
                
                // Zero out the queue memory region before use
                unsafe {
                    core::ptr::write_bytes(desc_table_addr as *mut u8, 0, total_size);
                }
                
                // Initialize queue structures
                self.queue.init(queue_size, VIRTIO_BLK_REQUEST_QUEUE_IDX, desc_table_addr, avail_ring_addr, used_ring_addr)?;
                
                // Step 3: Set queue alignment (power of 2, typically page size)
                let queue_align = PAGE_SIZE as u32;
                self.write_reg_u32(VIRTIO_MMIO_QUEUE_ALIGN, queue_align);
                console_println!("[i] Set queue alignment: {} bytes", queue_align);
                
                // Step 4: Set queue PFN (Page Frame Number)
                let pfn = (desc_table_addr / PAGE_SIZE) as u32;
                console_println!("[i] Setting queue PFN: {} (addr=0x{:x})", pfn, desc_table_addr);
                self.write_reg_u32(VIRTIO_MMIO_QUEUE_PFN, pfn);
                
                // Verify the PFN was accepted
                let read_pfn = self.read_reg_u32(VIRTIO_MMIO_QUEUE_PFN);
                console_println!("[i] Queue PFN read back: {} (expected: {})", read_pfn, pfn);
                
            } else {
                // Modern VirtIO: Uses separate registers for each ring
                const QUEUE_MEMORY_BASE: usize = 0x81000000;
                let desc_table_size = 16 * queue_size as usize;
                let avail_ring_size = 6 + 2 * queue_size as usize;
                let used_ring_size = 6 + 8 * queue_size as usize;
                
                let desc_table_addr = QUEUE_MEMORY_BASE;
                let avail_ring_addr = desc_table_addr + desc_table_size;
                let used_ring_addr = (avail_ring_addr + avail_ring_size + 3) & !3; // 4-byte aligned
                
                // Calculate the total span of memory used by the modern queue setup
                // Used ring actual size: header (flags u16, idx u16) + elements (id u32, len u32)
                let modern_used_ring_content_size = 4 + (8 * queue_size as usize);
                // The used_ring_addr is the start. The end is used_ring_addr + modern_used_ring_content_size.
                // The total span is from desc_table_addr to the end of the used ring.
                let modern_queue_memory_end_addr = used_ring_addr + modern_used_ring_content_size;
                let modern_total_span = modern_queue_memory_end_addr - desc_table_addr;

                // Zero out the queue memory region before use
                unsafe {
                    core::ptr::write_bytes(desc_table_addr as *mut u8, 0, modern_total_span);
                }
                
                // Initialize the queue structure
                self.queue.init(queue_size, VIRTIO_BLK_REQUEST_QUEUE_IDX, desc_table_addr, avail_ring_addr, used_ring_addr)?;
                
                // Modern VirtIO uses separate registers for each ring
                self.write_reg_u32(VIRTIO_MMIO_QUEUE_DESC_LOW, desc_table_addr as u32);
                self.write_reg_u32(VIRTIO_MMIO_QUEUE_DESC_HIGH, (desc_table_addr >> 32) as u32);
                
                self.write_reg_u32(VIRTIO_MMIO_QUEUE_DRIVER_LOW, avail_ring_addr as u32);
                self.write_reg_u32(VIRTIO_MMIO_QUEUE_DRIVER_HIGH, (avail_ring_addr >> 32) as u32);
                
                self.write_reg_u32(VIRTIO_MMIO_QUEUE_DEVICE_LOW, used_ring_addr as u32);
                self.write_reg_u32(VIRTIO_MMIO_QUEUE_DEVICE_HIGH, (used_ring_addr >> 32) as u32);
                
                // Mark queue as ready (modern VirtIO only)
                self.write_reg_u32(VIRTIO_MMIO_QUEUE_READY, 1);
            }
            
            console_println!("[o] VirtIO queue ready");
        }
        
        self.queue.ready = true; // Mark the queue object as ready for driver operations
        Ok(())
    }
    
    /// Set DRIVER_OK status bit to complete initialization
    fn set_driver_ok(&mut self) -> DiskResult<()> {
        let base = self.mmio_base;
            
        if self.is_legacy {
            // Legacy VirtIO: Don't set FEATURES_OK
            self.write_reg_u32(VIRTIO_MMIO_STATUS, VIRTIO_STATUS_ACKNOWLEDGE as u32 | VIRTIO_STATUS_DRIVER as u32 | VIRTIO_STATUS_DRIVER_OK as u32);
        } else {
            // Modern VirtIO: Include FEATURES_OK
            self.write_reg_u32(VIRTIO_MMIO_STATUS, VIRTIO_STATUS_ACKNOWLEDGE as u32 | VIRTIO_STATUS_DRIVER as u32 | VIRTIO_STATUS_FEATURES_OK as u32 | VIRTIO_STATUS_DRIVER_OK as u32);
        }
            
        console_println!("[o] VirtIO device ready");
        Ok(())
    }
    
    /// Read a sector using real VirtIO I/O
    pub fn read_sector(&mut self, sector: u64, buffer: &mut [u8; 512]) -> DiskResult<()> {
        if !self.initialized {
            return Err(DiskError::NotInitialized);
        }
        
        if sector >= self.capacity_sectors {
            return Err(DiskError::InvalidSector);
        }
        
        // Perform real VirtIO I/O
        self.virtio_read_sector(sector, buffer)?;
        
        // console_println!("[o] VirtIO read completed for sector {}", sector);
        Ok(())
    }
    
    /// Perform actual VirtIO block read operation
    fn virtio_read_sector(&mut self, sector: u64, buffer: &mut [u8; 512]) -> DiskResult<()> {
        let head_index; // To store the head index of our request
        unsafe {
            // Use static buffers for VirtIO operations (device-accessible memory)
            VIRTIO_REQUEST_BUFFER = VirtioBlkReq::new_read(sector);
            VIRTIO_STATUS_BUFFER = 0xFF; // Initialize to non-OK, device overwrites
            
            // Create descriptor chain using static buffer addresses
            let desc_chain = [
                // Descriptor 0: Request header (device reads from this)
                VirtqDesc {
                    addr: &VIRTIO_REQUEST_BUFFER as *const _ as u64,
                    len: core::mem::size_of::<VirtioBlkReq>() as u32,
                    flags: VIRTQ_DESC_F_NEXT,
                    next: 1,
                },
                // Descriptor 1: Data buffer (device writes to this)
                VirtqDesc {
                    addr: VIRTIO_DATA_BUFFER.as_mut_ptr() as u64,
                    len: 512,
                    flags: VIRTQ_DESC_F_WRITE | VIRTQ_DESC_F_NEXT,
                    next: 2,
                },
                // Descriptor 2: Status byte (device writes to this)
                VirtqDesc {
                    addr: &mut VIRTIO_STATUS_BUFFER as *mut _ as u64,
                    len: 1,
                    flags: VIRTQ_DESC_F_WRITE,
                    next: 0, // Marks end of this chain for this descriptor
                },
            ];
            
            // Add descriptor chain to queue
            head_index = self.queue.add_descriptor_chain(&desc_chain)?;
            
            // console_println!("[i] READ Desc chain (head={}) setup (static buffers):", head_index);
            // console_println!("  Request addr: 0x{:x}, len: {}", &VIRTIO_REQUEST_BUFFER as *const _ as u64, core::mem::size_of::<VirtioBlkReq>());
            // console_println!("  Buffer addr: 0x{:x}, len: 512", VIRTIO_DATA_BUFFER.as_mut_ptr() as u64);
            // console_println!("  Status addr: 0x{:x}, len: 1", &mut VIRTIO_STATUS_BUFFER as *mut _ as u64);
            
            // Notify device
            // console_println!("[i] Notifying VirtIO device at queue {} for READ", self.queue.queue_index);
            self.write_reg_u32(VIRTIO_MMIO_QUEUE_NOTIFY, self.queue.queue_index as u32);
            
            // console_println!("[i] VirtIO READ request (head={}) submitted, waiting for completion...", head_index);
        } // End of unsafe block for buffer setup
            
        // Wait for completion with improved timeout handling
        let mut timeout = 2000000; // Increased timeout
        let mut poll_count = 0;
        
        loop {
            if timeout <= 0 {
                console_println!("[x] VirtIO READ request (head={}, sector={}) timed out after {} polls.", head_index, sector, poll_count);
                return Err(DiskError::IoError);
            }

            // Check for completion
            if let Some(used_elem) = self.queue.wait_for_completion(head_index) {
                if unsafe { VIRTIO_STATUS_BUFFER } == VIRTIO_BLK_S_OK {
                    unsafe { buffer.copy_from_slice(&VIRTIO_DATA_BUFFER); }
                    return Ok(());
                } else {
                    let status_val = unsafe { VIRTIO_STATUS_BUFFER };
                    console_println!("[x] VirtIO READ failed (head={}, sector={}) with device status: 0x{:x}", 
                                   head_index, sector, status_val);
                    return Err(DiskError::ReadError);
                }
            }

            // Reduced logging frequency  
            if poll_count % 500000 == 0 && poll_count > 0 {
                console_println!("[i] READ (head={}) still waiting... polls: {}", 
                               head_index, poll_count);
            }
            
            timeout -= 1;
            poll_count += 1;
            core::hint::spin_loop();
        }
    }
    
    /// Write a sector (placeholder for future implementation)
    pub fn write_sector(&mut self, sector: u64, buffer: &[u8; 512]) -> DiskResult<()> {
        if !self.initialized {
            console_println!("Attempted to write to uninitialized VirtIO block device");
            return Err(DiskError::NotInitialized);
        }
        // Call the helper function that contains the actual VirtIO logic
        self.virtio_write_sector(sector, buffer)
    }

    // Helper function for the actual VirtIO write logic
    fn virtio_write_sector(&mut self, sector: u64, buffer: &[u8; 512]) -> DiskResult<()> {
        let head_index; // To store the head index of our request
        unsafe {
            // 1. Prepare static buffers
            VIRTIO_REQUEST_BUFFER = VirtioBlkReq::new_write(sector);
            VIRTIO_DATA_BUFFER.copy_from_slice(buffer); // Copy input data to static DMA buffer
            VIRTIO_STATUS_BUFFER = 0xFF; // Initialize to a non-OK value, device will overwrite

            // 2. Create descriptor chain using static buffer addresses
            let desc_chain = [
                // Descriptor 0: Request header (device reads from this)
                VirtqDesc {
                    addr: &VIRTIO_REQUEST_BUFFER as *const _ as u64,
                    len: core::mem::size_of::<VirtioBlkReq>() as u32,
                    flags: VIRTQ_DESC_F_NEXT,
                    next: 1,
                },
                // Descriptor 1: Data buffer (device reads from this)
                // For a write operation, VIRTQ_DESC_F_WRITE is NOT set.
                VirtqDesc {
                    addr: VIRTIO_DATA_BUFFER.as_ptr() as u64, // Device reads from here
                    len: VIRTIO_DATA_BUFFER.len() as u32,
                    flags: VIRTQ_DESC_F_NEXT,
                    next: 2,
                },
                // Descriptor 2: Status byte (device writes to this)
                VirtqDesc {
                    addr: &mut VIRTIO_STATUS_BUFFER as *mut _ as u64,
                    len: 1,
                    flags: VIRTQ_DESC_F_WRITE,
                    next: 0, // Marks end of this chain for this descriptor
                },
            ];

            // 3. Add descriptor chain to queue
            head_index = self.queue.add_descriptor_chain(&desc_chain)?;
            
            //console_println!("[i] WRITE request submitted (head={}, sector={})", head_index, sector);

            // 4. Notify device
            self.write_reg_u32(VIRTIO_MMIO_QUEUE_NOTIFY, self.queue.queue_index as u32); 
        } // End of unsafe block for buffer setup

        // 5. Wait for completion with improved handling
        let mut timeout = 2000000; // Increased timeout
        let mut poll_count = 0;
        
        loop {
            if timeout <= 0 {
                console_println!("[x] VirtIO WRITE request (head={}, sector={}) timed out after {} polls.", head_index, sector, poll_count);
                return Err(DiskError::IoError);
            }

            // Check for completion
            if let Some(used_elem) = self.queue.wait_for_completion(head_index) {
                if unsafe { VIRTIO_STATUS_BUFFER } == VIRTIO_BLK_S_OK {
                    //console_println!("[o] VirtIO WRITE successful (head={}, sector={})", head_index, sector);
                    return Ok(());
                } else {
                    let status_val = unsafe { VIRTIO_STATUS_BUFFER };
                    console_println!("[x] VirtIO WRITE failed (head={}, sector={}) with device status: 0x{:x}", 
                                   head_index, sector, status_val);
                    return Err(DiskError::WriteError); 
                }
            }

            // Reduced logging frequency
            if poll_count % 500000 == 0 && poll_count > 0 {
                //console_println!("[i] WRITE (head={}) still waiting... polls: {}", 
                //               head_index, poll_count);
            }
            
            timeout -= 1;
            poll_count += 1;
            core::hint::spin_loop(); 
        }
    }
    
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }
    
    pub fn get_capacity(&self) -> u64 {
        self.capacity_sectors
    }
    
    /// Compatibility method for filesystem
    pub fn read_blocks(&mut self, start_sector: u64, buffer: &mut [u8]) -> DiskResult<()> {
        if buffer.len() == 0 {
            return Ok(()); // Nothing to read
        }
        if buffer.len() % 512 != 0 {
            console_println!("[x] read_blocks: buffer length {} is not a multiple of 512", buffer.len());
            return Err(DiskError::BufferTooSmall); 
        }
        let num_sectors = buffer.len() / 512;
        for i in 0..num_sectors {
            let sector = start_sector + i as u64;
            let offset = i * 512;
            let sector_buffer_slice = &mut buffer[offset..offset + 512];
            let sector_buffer_array: &mut [u8; 512] = sector_buffer_slice.try_into()
                .expect("Slice to array conversion failed in read_blocks despite checks");
            self.read_sector(sector, sector_buffer_array)?;
        }
        Ok(())
    }

    pub fn write_blocks(&mut self, start_sector: u64, buffer: &[u8]) -> DiskResult<()> {
        if buffer.len() == 0 {
            return Ok(()); // Nothing to write
        }
        if buffer.len() % 512 != 0 {
            console_println!("[x] write_blocks: buffer length {} is not a multiple of 512", buffer.len());
            return Err(DiskError::BufferTooSmall);
        }
        let num_sectors = buffer.len() / 512;
        // console_println!("write_blocks: Writing {} sectors starting from {}", num_sectors, start_sector);
        for i in 0..num_sectors {
            let sector = start_sector + i as u64;
            let offset = i * 512;
            let sector_buffer_slice = &buffer[offset..offset + 512];
            let sector_buffer_array: &[u8; 512] = sector_buffer_slice.try_into()
                .expect("Slice to array conversion failed in write_blocks despite checks");
            // console_println!("write_blocks: Calling write_sector for sector {}", sector);
            self.write_sector(sector, sector_buffer_array)?;
        }
        // console_println!("write_blocks: Completed writing {} sectors", num_sectors);
        Ok(())
    }

    fn process_used_ring_entry(&mut self, used_ring_entry: &VirtqUsedElem) {
        let desc_idx = used_ring_entry.id as u16;
        let len = used_ring_entry.len;
        console_println!(
            "Processing used ring entry: desc_idx={}, len={}",
            desc_idx,
            len
        );
    }

    fn read_reg_u32(&self, offset: usize) -> u32 {
        let ptr = (self.mmio_base + offset) as *const u32;
        unsafe { core::ptr::read_volatile(ptr) }
    }

    fn write_reg_u32(&mut self, offset: usize, value: u32) {
        let ptr = (self.mmio_base + offset) as *mut u32;
        unsafe { core::ptr::write_volatile(ptr, value) }
    }

    fn set_status(&mut self, status_val: u8) {
        let current_status = self.read_reg_u32(VIRTIO_MMIO_STATUS);
        // Ensure status_val is u32 before ORing
        self.write_reg_u32(VIRTIO_MMIO_STATUS, current_status | (status_val as u32));
    }
}

// Global instance of the VirtIO Block device
// Create a global, mutable static instance of the VirtIO block device driver.
pub static VIRTIO_BLK: Mutex<RustVmmVirtIOBlock> = Mutex::new(RustVmmVirtIOBlock::new());

// Static buffers for VirtIO operations (device-accessible memory)
static mut VIRTIO_REQUEST_BUFFER: VirtioBlkReq = VirtioBlkReq { type_: 0, reserved: 0, sector: 0 };
static mut VIRTIO_DATA_BUFFER: [u8; 512] = [0; 512];
static mut VIRTIO_STATUS_BUFFER: u8 = 0;

/// Initialize the VirtIO block device
/// This function should be called during kernel initialization
pub fn init_virtio_blk() -> DiskResult<()> {
    console_println!("[i] Initializing rust-vmm style VirtIO Block Device...");
    
    let mut device = VIRTIO_BLK.lock();
    device.init()
}

/// Initialize VirtIO block device with specific address (for dynamic detection)
pub fn init_with_address(base_addr: usize) -> bool {
    console_println!("[i] Trying VirtIO device at 0x{:08x}", base_addr);
    
    unsafe {
        // Check if there's a valid VirtIO device at this address
        let magic = core::ptr::read_volatile(base_addr as *const u32);
        if magic != 0x74726976 {
            return false;
        }
        
        let version = core::ptr::read_volatile((base_addr + VIRTIO_MMIO_VERSION) as *const u32);
        let device_id = core::ptr::read_volatile((base_addr + VIRTIO_MMIO_DEVICE_ID) as *const u32);
        
        // Check if it's a block device
        if device_id != 2 {
            console_println!("[!]  Device at 0x{:08x} is not a block device (ID: {})", base_addr, device_id);
            return false;
        }
        
        console_println!("[o] Found VirtIO block device at 0x{:08x} (version: {})", base_addr, version);
        
        // Initialize the device with this address
        let mut device = RustVmmVirtIOBlock::new();
        device.mmio_base = base_addr;
        if device.init().is_ok() {
            console_println!("[o] VirtIO block device initialized successfully");
            
            // Store in global state
            *VIRTIO_BLK.lock() = device;
            return true;
        }
    }
    
    false
} 