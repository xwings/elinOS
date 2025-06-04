// VirtIO Block Device for elinOS
// Based on rust-vmm/vm-virtio Queue implementation
// Adapted for MMIO transport and no_std kernel environment
// License: Apache-2.0 / BSD-3-Clause (following rust-vmm)

use spin::Mutex;
use crate::console_println;
use core::{convert::TryInto, cmp::Ord, result::Result::{Ok, Err}};

// === DISK ERRORS ===
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DiskError {
    InvalidSector,
    BufferTooSmall,
    ReadError,
    WriteError,
    DeviceNotFound,
    NotInitialized,
    VirtIOError,
    IoError,
    QueueFull,
    InvalidDescriptor,
}

impl core::fmt::Display for DiskError {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        match self {
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

/// VirtIO Queue implementation based on rust-vmm
/// Simplified for experimental kernel but following the same patterns
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
    /// Next available index to use
    next_avail: u16,
    /// Last used index we've seen
    last_used_idx: u16,
    /// Queue select index
    queue_index: u16,
}

impl VirtioQueue {
    pub const fn new() -> Self {
        VirtioQueue {
            size: 0,
            ready: false,
            desc_table: 0,
            avail_ring: 0,
            used_ring: 0,
            next_avail: 0,
            last_used_idx: 0,
            queue_index: 0,
        }
    }
    
    /// Initialize the queue with given parameters
    pub fn init(&mut self, size: u16, desc_table: usize, avail_ring: usize, used_ring: usize) -> DiskResult<()> {
        if !size.is_power_of_two() || size == 0 || size > 256 {
            return Err(DiskError::VirtIOError);
        }
        
        self.size = size;
        self.desc_table = desc_table;
        self.avail_ring = avail_ring;
        self.used_ring = used_ring;
        self.next_avail = 0;
        self.last_used_idx = 0;
        self.ready = true;
        
        // Initialize memory structures
        unsafe {
            // Clear descriptor table
            let desc_ptr = self.desc_table as *mut VirtqDesc;
            for i in 0..self.size {
                *desc_ptr.offset(i as isize) = VirtqDesc::new();
            }
            
            // Initialize available ring
            let avail_ptr = self.avail_ring as *mut VirtqAvail;
            *avail_ptr = VirtqAvail::new();
            
            // Initialize used ring
            let used_ptr = self.used_ring as *mut VirtqUsed;
            *used_ptr = VirtqUsed::new();
        }
        
        Ok(())
    }
    
    /// Add a descriptor chain to the available ring
    /// Returns the head descriptor index
    pub fn add_descriptor_chain(&mut self, chain: &[VirtqDesc]) -> DiskResult<u16> {
        if !self.ready || chain.is_empty() || chain.len() > self.size as usize {
            return Err(DiskError::QueueFull);
        }
        
        // Check if we have enough descriptors
        let available_count = self.size - self.get_queue_used_count();
        if chain.len() > available_count as usize {
            return Err(DiskError::QueueFull);
        }
        
        let head_index = self.next_avail;
        
        unsafe {
            let desc_table = self.desc_table as *mut VirtqDesc;
            let avail_ring = self.avail_ring as *mut VirtqAvail;
            
            // Set up descriptor chain
            for (i, desc) in chain.iter().enumerate() {
                let desc_index = (head_index + i as u16) % self.size;
                let desc_ptr = desc_table.offset(desc_index as isize);
                *desc_ptr = *desc;
                
                // Set next pointer (except for last descriptor)
                if i < chain.len() - 1 {
                    (*desc_ptr).flags |= VIRTQ_DESC_F_NEXT;
                    (*desc_ptr).next = (head_index + i as u16 + 1) % self.size;
                }
            }
            
            // Add to available ring
            let avail_idx = (*avail_ring).idx;
            (*avail_ring).ring[avail_idx as usize % self.size as usize] = head_index;
            
            console_println!("📋 Queue state: avail_idx={}, head_index={}, chain_len={}", 
                avail_idx, head_index, chain.len());
            
            // Memory barrier
            core::sync::atomic::fence(core::sync::atomic::Ordering::SeqCst);
            
            // Update available index
            (*avail_ring).idx = avail_idx.wrapping_add(1);
            
            console_println!("📋 Updated avail_idx to: {}", (*avail_ring).idx);
        }
        
        self.next_avail = (self.next_avail + chain.len() as u16) % self.size;
        Ok(head_index)
    }
    
    /// Check for completed requests in the used ring
    pub fn get_used_elem(&mut self) -> Option<VirtqUsedElem> {
        unsafe {
            let used_ring = self.used_ring as *mut VirtqUsed;
            let used_idx = (*used_ring).idx;
            
            if self.last_used_idx == used_idx {
                return None; // No new used elements
            }
            
            let elem = (*used_ring).ring[self.last_used_idx as usize % self.size as usize];
            self.last_used_idx = self.last_used_idx.wrapping_add(1);
            
            Some(elem)
        }
    }
    
    /// Get the number of used descriptors in the queue
    fn get_queue_used_count(&self) -> u16 {
        unsafe {
            let avail_ring = self.avail_ring as *const VirtqAvail;
            let used_ring = self.used_ring as *const VirtqUsed;
            
            let avail_idx = (*avail_ring).idx;
            let used_idx = (*used_ring).idx;
            
            avail_idx.wrapping_sub(used_idx)
        }
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
        console_println!("🚀 Initializing rust-vmm style VirtIO Block Device...");
        
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
        console_println!("✅ rust-vmm VirtIO block device initialized successfully");
        Ok(())
    }
    
    /// Discover VirtIO MMIO device
    fn discover_device(&mut self) -> DiskResult<bool> {
        console_println!("🔍 Scanning for VirtIO MMIO devices...");
        
        // QEMU virt machine VirtIO MMIO addresses
        let mmio_addresses = [
            0x10001000, 0x10002000, 0x10003000, 0x10004000,
            0x10005000, 0x10006000, 0x10007000, 0x10008000,
        ];
        
        for &addr in &mmio_addresses {
            if self.probe_mmio_device(addr)? {
                self.mmio_base = addr;
                console_println!("✅ VirtIO block device found at 0x{:x}", addr);
                return Ok(true);
            }
        }
        
        console_println!("❌ No VirtIO block device found");
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
                console_println!("📱 Modern VirtIO block device: version={}, vendor=0x{:x}", version, vendor_id);
            } else if version == 1 {
                console_println!("📱 Legacy VirtIO block device: version={}, vendor=0x{:x} (experimental extension)", version, vendor_id);
                self.is_legacy = true;
            } else {
                console_println!("⚠️  Unknown VirtIO version {} at 0x{:x}, skipping", version, base);
                return Ok(false);
            }
            
            Ok(true)
        }
    }
    
    /// Initialize the VirtIO device following the initialization sequence
    fn init_device(&mut self) -> DiskResult<()> {
        console_println!("🔧 Initializing VirtIO device...");
        
        unsafe {
            let base = self.mmio_base;
            
            // Step 1: Reset the device
            core::ptr::write_volatile((base + VIRTIO_MMIO_STATUS) as *mut u32, 0);
            
            // Step 2: Set ACKNOWLEDGE status bit
            core::ptr::write_volatile((base + VIRTIO_MMIO_STATUS) as *mut u32, VIRTIO_STATUS_ACKNOWLEDGE);
            
            // Step 3: Set DRIVER status bit
            core::ptr::write_volatile((base + VIRTIO_MMIO_STATUS) as *mut u32, 
                VIRTIO_STATUS_ACKNOWLEDGE | VIRTIO_STATUS_DRIVER);
            
            if self.is_legacy {
                console_println!("🔧 Initializing Legacy VirtIO (experimental extension)");
                
                // Legacy VirtIO: Read features directly
                self.device_features = core::ptr::read_volatile((base + VIRTIO_MMIO_DEVICE_FEATURES) as *const u32) as u64;
                console_println!("🔍 Device features: 0x{:x}", self.device_features);
                
                // Legacy VirtIO: Set driver features directly
                self.driver_features = 0; // Minimal features for simplicity
                core::ptr::write_volatile((base + VIRTIO_MMIO_DRIVER_FEATURES) as *mut u32, 
                    self.driver_features as u32);
                
                // Legacy VirtIO: Skip FEATURES_OK step
            } else {
                console_println!("🔧 Initializing Modern VirtIO");
                
                // Step 4: Read device features (modern VirtIO)
                core::ptr::write_volatile((base + VIRTIO_MMIO_DEVICE_FEATURES_SEL) as *mut u32, 0);
                let features_low = core::ptr::read_volatile((base + VIRTIO_MMIO_DEVICE_FEATURES) as *const u32);
                core::ptr::write_volatile((base + VIRTIO_MMIO_DEVICE_FEATURES_SEL) as *mut u32, 1);
                let features_high = core::ptr::read_volatile((base + VIRTIO_MMIO_DEVICE_FEATURES) as *const u32);
                
                self.device_features = ((features_high as u64) << 32) | (features_low as u64);
                console_println!("🔍 Device features: 0x{:x}", self.device_features);
                
                // Step 5: Set driver features (accept basic features only)
                self.driver_features = 0; // Minimal features for simplicity
                core::ptr::write_volatile((base + VIRTIO_MMIO_DRIVER_FEATURES_SEL) as *mut u32, 0);
                core::ptr::write_volatile((base + VIRTIO_MMIO_DRIVER_FEATURES) as *mut u32, 
                    self.driver_features as u32);
                core::ptr::write_volatile((base + VIRTIO_MMIO_DRIVER_FEATURES_SEL) as *mut u32, 1);
                core::ptr::write_volatile((base + VIRTIO_MMIO_DRIVER_FEATURES) as *mut u32, 
                    (self.driver_features >> 32) as u32);
                
                // Step 6: Set FEATURES_OK status bit
                core::ptr::write_volatile((base + VIRTIO_MMIO_STATUS) as *mut u32, 
                    VIRTIO_STATUS_ACKNOWLEDGE | VIRTIO_STATUS_DRIVER | VIRTIO_STATUS_FEATURES_OK);
                
                // Step 7: Verify FEATURES_OK is still set
                let status = core::ptr::read_volatile((base + VIRTIO_MMIO_STATUS) as *const u32);
                if (status & VIRTIO_STATUS_FEATURES_OK) == 0 {
                    return Err(DiskError::VirtIOError);
                }
            }
            
            // Step 8: Read device configuration
            let capacity_low = core::ptr::read_volatile((base + VIRTIO_MMIO_CONFIG) as *const u32);
            let capacity_high = core::ptr::read_volatile((base + VIRTIO_MMIO_CONFIG + 4) as *const u32);
            self.capacity_sectors = ((capacity_high as u64) << 32) | (capacity_low as u64);
            
            console_println!("💽 Device capacity: {} sectors ({} MB)", 
                self.capacity_sectors, self.capacity_sectors * 512 / 1024 / 1024);
        }
        
        Ok(())
    }
    
    /// Set up the virtqueue
    fn setup_queue(&mut self) -> DiskResult<()> {
        console_println!("🔄 Setting up VirtIO queue...");
        
        unsafe {
            let base = self.mmio_base;
            
            // Select queue 0
            core::ptr::write_volatile((base + VIRTIO_MMIO_QUEUE_SEL) as *mut u32, 0);
            
            // Get maximum queue size
            let max_queue_size = core::ptr::read_volatile((base + VIRTIO_MMIO_QUEUE_NUM_MAX) as *const u32);
            console_println!("📊 Max queue size: {}", max_queue_size);
            
            // Set queue size (use smaller size for simplicity)
            let queue_size = 64.min(max_queue_size as u16);
            if !queue_size.is_power_of_two() {
                return Err(DiskError::VirtIOError);
            }
            
            core::ptr::write_volatile((base + VIRTIO_MMIO_QUEUE_NUM) as *mut u32, queue_size as u32);
            
            if self.is_legacy {
                console_println!("🔧 Legacy VirtIO queue setup (following rcore-os implementation)");
                
                // Step 1: Set guest page size (REQUIRED for legacy VirtIO)
                core::ptr::write_volatile((base + VIRTIO_MMIO_GUEST_PAGE_SIZE) as *mut u32, PAGE_SIZE as u32);
                console_println!("📏 Set guest page size: {} bytes", PAGE_SIZE);
                
                // Step 2: Calculate memory layout following VirtIO spec
                // Legacy VirtIO requires ALL rings to be contiguous and page-aligned
                let desc_table_size = 16 * queue_size as usize; // 16 bytes per descriptor
                let avail_ring_size = 6 + 2 * queue_size as usize; // 6 bytes header + 2 bytes per entry
                let used_ring_size = 6 + 8 * queue_size as usize; // 6 bytes header + 8 bytes per entry
                
                // Calculate aligned layout exactly like rcore-os
                let driver_area_offset = desc_table_size;
                let device_area_offset = align_up(desc_table_size + avail_ring_size);
                let total_size = align_up(device_area_offset + used_ring_size);
                
                console_println!("📐 Legacy memory layout calculation:");
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
                
                console_println!("📍 Legacy queue memory layout (rcore-os style):");
                console_println!("  Descriptors: 0x{:x}", desc_table_addr);
                console_println!("  Available:   0x{:x}", avail_ring_addr);
                console_println!("  Used:        0x{:x}", used_ring_addr);
                
                // Initialize queue structures
                self.queue.init(queue_size, desc_table_addr, avail_ring_addr, used_ring_addr)?;
                
                // Step 3: Set queue alignment (power of 2, typically page size)
                let queue_align = PAGE_SIZE as u32;
                core::ptr::write_volatile((base + VIRTIO_MMIO_QUEUE_ALIGN) as *mut u32, queue_align);
                console_println!("📏 Set queue alignment: {} bytes", queue_align);
                
                // Step 4: Set queue PFN (Page Frame Number)
                let pfn = (desc_table_addr / PAGE_SIZE) as u32;
                console_println!("📄 Setting queue PFN: {} (addr=0x{:x})", pfn, desc_table_addr);
                core::ptr::write_volatile((base + VIRTIO_MMIO_QUEUE_PFN) as *mut u32, pfn);
                
                // Verify the PFN was accepted
                let read_pfn = core::ptr::read_volatile((base + VIRTIO_MMIO_QUEUE_PFN) as *const u32);
                console_println!("📄 Queue PFN read back: {} (expected: {})", read_pfn, pfn);
                
            } else {
                console_println!("🔧 Modern VirtIO queue setup");
                // Modern VirtIO: Uses separate registers for each ring
                const QUEUE_MEMORY_BASE: usize = 0x81000000;
                let desc_table_size = 16 * queue_size as usize;
                let avail_ring_size = 6 + 2 * queue_size as usize;
                let used_ring_size = 6 + 8 * queue_size as usize;
                
                let desc_table_addr = QUEUE_MEMORY_BASE;
                let avail_ring_addr = desc_table_addr + desc_table_size;
                let used_ring_addr = (avail_ring_addr + avail_ring_size + 3) & !3; // 4-byte aligned
                
                // Initialize queue
                self.queue.init(queue_size, desc_table_addr, avail_ring_addr, used_ring_addr)?;
                
                // Modern VirtIO uses separate registers for each ring
                core::ptr::write_volatile((base + VIRTIO_MMIO_QUEUE_DESC_LOW) as *mut u32, desc_table_addr as u32);
                core::ptr::write_volatile((base + VIRTIO_MMIO_QUEUE_DESC_HIGH) as *mut u32, (desc_table_addr >> 32) as u32);
                
                core::ptr::write_volatile((base + VIRTIO_MMIO_QUEUE_DRIVER_LOW) as *mut u32, avail_ring_addr as u32);
                core::ptr::write_volatile((base + VIRTIO_MMIO_QUEUE_DRIVER_HIGH) as *mut u32, (avail_ring_addr >> 32) as u32);
                
                core::ptr::write_volatile((base + VIRTIO_MMIO_QUEUE_DEVICE_LOW) as *mut u32, used_ring_addr as u32);
                core::ptr::write_volatile((base + VIRTIO_MMIO_QUEUE_DEVICE_HIGH) as *mut u32, (used_ring_addr >> 32) as u32);
                
                // Mark queue as ready (modern VirtIO only)
                core::ptr::write_volatile((base + VIRTIO_MMIO_QUEUE_READY) as *mut u32, 1);
            }
            
            console_println!("✅ VirtIO queue ready");
        }
        
        Ok(())
    }
    
    /// Set DRIVER_OK status bit to complete initialization
    fn set_driver_ok(&mut self) -> DiskResult<()> {
        unsafe {
            let base = self.mmio_base;
            
            if self.is_legacy {
                // Legacy VirtIO: Don't set FEATURES_OK
                core::ptr::write_volatile((base + VIRTIO_MMIO_STATUS) as *mut u32, 
                    VIRTIO_STATUS_ACKNOWLEDGE | VIRTIO_STATUS_DRIVER | VIRTIO_STATUS_DRIVER_OK);
            } else {
                // Modern VirtIO: Include FEATURES_OK
                core::ptr::write_volatile((base + VIRTIO_MMIO_STATUS) as *mut u32, 
                    VIRTIO_STATUS_ACKNOWLEDGE | VIRTIO_STATUS_DRIVER | VIRTIO_STATUS_FEATURES_OK | VIRTIO_STATUS_DRIVER_OK);
            }
            
            console_println!("✅ VirtIO device ready");
        }
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
        
        console_println!("📖 VirtIO read sector {} (rust-vmm style)", sector);
        
        // Perform real VirtIO I/O
        self.virtio_read_sector(sector, buffer)?;
        
        console_println!("✅ VirtIO read completed for sector {}", sector);
        Ok(())
    }
    
    /// Perform actual VirtIO block read operation
    fn virtio_read_sector(&mut self, sector: u64, buffer: &mut [u8; 512]) -> DiskResult<()> {
        console_println!("🔧 Executing VirtIO block read operation");
        
        unsafe {
            // Use static buffers for VirtIO operations (device-accessible memory)
            VIRTIO_REQUEST_BUFFER = VirtioBlkReq::new_read(sector);
            VIRTIO_STATUS_BUFFER = 0;
            
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
                    next: 0,
                },
            ];
            
            // Add descriptor chain to queue
            let _head_index = self.queue.add_descriptor_chain(&desc_chain)?;
            
            console_println!("📝 Descriptor chain setup (static buffers):");
            console_println!("  Request addr: 0x{:x}, len: {}", &VIRTIO_REQUEST_BUFFER as *const _ as u64, core::mem::size_of::<VirtioBlkReq>());
            console_println!("  Buffer addr: 0x{:x}, len: 512", VIRTIO_DATA_BUFFER.as_mut_ptr() as u64);
            console_println!("  Status addr: 0x{:x}, len: 1", &mut VIRTIO_STATUS_BUFFER as *mut _ as u64);
            
            // Notify device
            console_println!("🔔 Notifying VirtIO device at queue 0");
            core::ptr::write_volatile((self.mmio_base + VIRTIO_MMIO_QUEUE_NOTIFY) as *mut u32, 0);
            
            console_println!("📤 VirtIO request submitted, waiting for completion...");
            
            // Check initial device state
            let interrupt_status = core::ptr::read_volatile((self.mmio_base + VIRTIO_MMIO_INTERRUPT_STATUS) as *const u32);
            console_println!("🔍 Initial interrupt status: 0x{:x}", interrupt_status);
            
            // Wait for completion with timeout
            let mut timeout = 1000000;
            let mut poll_count = 0;
            while timeout > 0 {
                // Check interrupt status periodically
                if poll_count % 100000 == 0 {
                    let interrupt_status = core::ptr::read_volatile((self.mmio_base + VIRTIO_MMIO_INTERRUPT_STATUS) as *const u32);
                    console_println!("🔍 Poll {}: interrupt status: 0x{:x}", poll_count / 100000, interrupt_status);
                    
                    // Check queue state
                    let used_ring = self.queue.used_ring as *const VirtqUsed;
                    let used_idx = (*used_ring).idx;
                    console_println!("🔍 Queue used_idx: {}, last_used_idx: {}", used_idx, self.queue.last_used_idx);
                }
                
                if let Some(used_elem) = self.queue.get_used_elem() {
                    console_println!("📥 VirtIO request completed, used_elem: id={}, len={}, status={}", 
                        used_elem.id, used_elem.len, VIRTIO_STATUS_BUFFER);
                    
                    if VIRTIO_STATUS_BUFFER == VIRTIO_BLK_S_OK {
                        console_println!("✅ VirtIO read successful!");
                        // Copy data from static buffer to user buffer
                        buffer.copy_from_slice(&VIRTIO_DATA_BUFFER);
                        return Ok(());
                    } else {
                        console_println!("❌ VirtIO read failed with status: {}", VIRTIO_STATUS_BUFFER);
                        return Err(DiskError::ReadError);
                    }
                }
                timeout -= 1;
                poll_count += 1;
                core::hint::spin_loop();
            }
            
            console_println!("⏰ VirtIO request timed out");
            Err(DiskError::IoError)
        }
    }
    
    /// Write a sector (placeholder for future implementation)
    pub fn write_sector(&mut self, _sector: u64, _buffer: &[u8; 512]) -> DiskResult<()> {
        Err(DiskError::WriteError)
    }
    
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }
    
    pub fn get_capacity(&self) -> u64 {
        self.capacity_sectors
    }
    
    /// Compatibility method for filesystem
    pub fn read_blocks(&mut self, sector: u64, buffer: &mut [u8; 512]) -> DiskResult<()> {
        self.read_sector(sector, buffer)
    }
}

// Global VirtIO block device instance
pub static VIRTIO_BLK: Mutex<RustVmmVirtIOBlock> = Mutex::new(RustVmmVirtIOBlock::new());

// Static buffers for VirtIO operations (device-accessible memory)
static mut VIRTIO_REQUEST_BUFFER: VirtioBlkReq = VirtioBlkReq { type_: 0, reserved: 0, sector: 0 };
static mut VIRTIO_DATA_BUFFER: [u8; 512] = [0; 512];
static mut VIRTIO_STATUS_BUFFER: u8 = 0;

/// Initialize the VirtIO block device
/// This function should be called during kernel initialization
pub fn init_virtio_blk() -> DiskResult<()> {
    let mut device = VIRTIO_BLK.lock();
    device.init()
}

/// Initialize VirtIO block device with specific address (for dynamic detection)
pub fn init_with_address(base_addr: usize) -> bool {
    console_println!("🔍 Trying VirtIO device at 0x{:08x}", base_addr);
    
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
            console_println!("⚠️  Device at 0x{:08x} is not a block device (ID: {})", base_addr, device_id);
            return false;
        }
        
        console_println!("✅ Found VirtIO block device at 0x{:08x} (version: {})", base_addr, version);
        
        // Initialize the device with this address
        let mut device = RustVmmVirtIOBlock::new();
        device.mmio_base = base_addr;
        if device.init().is_ok() {
            console_println!("✅ VirtIO block device initialized successfully");
            
            // Store in global state
            *VIRTIO_BLK.lock() = device;
            return true;
        }
    }
    
    false
} 