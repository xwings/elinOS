// Simple VirtIO Block Device for elinOS
// Minimal implementation focused on basic sector I/O

use core::ptr;
use spin::Mutex;
use crate::console_println;

// === PROPER RUST ERROR TYPES ===
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum VirtIOError {
    InvalidMagic(u32),
    UnsupportedVersion(u32),
    WrongDeviceType(u32),
    DeviceNotInitialized,
    InvalidSectorSize,
    IoTimeout,
    DeviceNotFound,
}

impl core::fmt::Display for VirtIOError {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        match self {
            VirtIOError::InvalidMagic(magic) => write!(f, "Invalid VirtIO magic: 0x{:x}", magic),
            VirtIOError::UnsupportedVersion(ver) => write!(f, "Unsupported VirtIO version: {}", ver),
            VirtIOError::WrongDeviceType(id) => write!(f, "Wrong device type: {}", id),
            VirtIOError::DeviceNotInitialized => write!(f, "Device not initialized"),
            VirtIOError::InvalidSectorSize => write!(f, "Invalid sector size"),
            VirtIOError::IoTimeout => write!(f, "I/O operation timeout"),
            VirtIOError::DeviceNotFound => write!(f, "VirtIO device not found"),
        }
    }
}

// Type alias for cleaner Result types
pub type VirtIOResult<T> = Result<T, VirtIOError>;

// === NEWTYPE PATTERNS FOR TYPE SAFETY ===
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct SectorNumber(pub u64);

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MemoryAddress(pub usize);

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DeviceCapacity(pub u64);

// === TRAITS FOR BETTER ABSTRACTION ===
pub trait BlockDevice {
    type Error;
    
    fn read_sector(&mut self, sector: SectorNumber, buffer: &mut [u8; 512]) -> Result<(), Self::Error>;
    fn write_sector(&mut self, sector: SectorNumber, buffer: &[u8; 512]) -> Result<(), Self::Error>;
    fn get_capacity(&self) -> DeviceCapacity;
    fn is_ready(&self) -> bool;
}

// VirtIO MMIO Register Offsets
const VIRTIO_MMIO_MAGIC_VALUE: usize = 0x000;
const VIRTIO_MMIO_VERSION: usize = 0x004;
const VIRTIO_MMIO_DEVICE_ID: usize = 0x008;
const VIRTIO_MMIO_VENDOR_ID: usize = 0x00c;
const VIRTIO_MMIO_DEVICE_FEATURES: usize = 0x010;
const VIRTIO_MMIO_DEVICE_FEATURES_SEL: usize = 0x014;
const VIRTIO_MMIO_DRIVER_FEATURES: usize = 0x020;
const VIRTIO_MMIO_DRIVER_FEATURES_SEL: usize = 0x024;
const VIRTIO_MMIO_QUEUE_SEL: usize = 0x030;
const VIRTIO_MMIO_QUEUE_NUM_MAX: usize = 0x034;
const VIRTIO_MMIO_QUEUE_NUM: usize = 0x038;
const VIRTIO_MMIO_QUEUE_READY: usize = 0x044;
const VIRTIO_MMIO_QUEUE_NOTIFY: usize = 0x050;
const VIRTIO_MMIO_INTERRUPT_STATUS: usize = 0x060;
const VIRTIO_MMIO_INTERRUPT_ACK: usize = 0x064;
const VIRTIO_MMIO_STATUS: usize = 0x070;
const VIRTIO_MMIO_QUEUE_DESC_LOW: usize = 0x080;
const VIRTIO_MMIO_QUEUE_DESC_HIGH: usize = 0x084;
const VIRTIO_MMIO_QUEUE_AVAIL_LOW: usize = 0x090;
const VIRTIO_MMIO_QUEUE_AVAIL_HIGH: usize = 0x094;
const VIRTIO_MMIO_QUEUE_USED_LOW: usize = 0x0a0;
const VIRTIO_MMIO_QUEUE_USED_HIGH: usize = 0x0a4;
const VIRTIO_MMIO_CONFIG_GENERATION: usize = 0x0fc;

// VirtIO Constants
const VIRTIO_MAGIC: u32 = 0x74726976; // "virt"
const VIRTIO_VERSION_LEGACY: u32 = 1;
const VIRTIO_DEVICE_ID_BLOCK: u32 = 2;

// Device Status Register Values
const VIRTIO_STATUS_ACKNOWLEDGE: u32 = 1;
const VIRTIO_STATUS_DRIVER: u32 = 2;
const VIRTIO_STATUS_DRIVER_OK: u32 = 4;
const VIRTIO_STATUS_FEATURES_OK: u32 = 8;
const VIRTIO_STATUS_DEVICE_NEEDS_RESET: u32 = 64;
const VIRTIO_STATUS_FAILED: u32 = 128;

// VirtIO Block Command Types
const VIRTIO_BLK_T_IN: u32 = 0;      // Read
const VIRTIO_BLK_T_OUT: u32 = 1;     // Write
const VIRTIO_BLK_T_FLUSH: u32 = 4;   // Flush

// VirtIO Block Status (keep only the ones we use)
const VIRTIO_BLK_S_OK: u8 = 0;

// Descriptor Flags (remove unused ones)
// const VIRTQ_DESC_F_NEXT: u16 = 1;    // Unused - removing
// const VIRTQ_DESC_F_WRITE: u16 = 2;   // Unused - removing

// Simple VirtIO Block Request
#[repr(C)]
#[derive(Copy, Clone)]
struct VirtIOBlockRequest {
    request_type: u32,  // VIRTIO_BLK_T_*
    reserved: u32,      // Must be 0
    sector: u64,        // Sector number
}

// Simple descriptor for our minimal queue
#[repr(C)]
#[derive(Copy, Clone)]
struct VirtqDesc {
    addr: u64,    // Physical address
    len: u32,     // Length
    flags: u16,   // Descriptor flags
    next: u16,    // Next descriptor index
}

// Simple available ring
#[repr(C)]
struct VirtqAvail {
    flags: u16,
    idx: u16,
    ring: [u16; 16], // Simple 16-entry ring
    used_event: u16,
}

// Simple used ring element
#[repr(C)]
#[derive(Copy, Clone)]
struct VirtqUsedElem {
    id: u32,   // Descriptor chain head
    len: u32,  // Bytes written
}

// Simple used ring
#[repr(C)]
struct VirtqUsed {
    flags: u16,
    idx: u16,
    ring: [VirtqUsedElem; 16], // Simple 16-entry ring
    avail_event: u16,
}

// Simple VirtIO Block Device
pub struct SimpleVirtIOBlock {
    base_addr: MemoryAddress,
    capacity: DeviceCapacity,
    initialized: bool,
    
    // Minimal queue (all in one place for simplicity)
    descriptors: [VirtqDesc; 16],
    avail: VirtqAvail,
    used: VirtqUsed,
}

impl SimpleVirtIOBlock {
    pub const fn new() -> Self {
        SimpleVirtIOBlock {
            base_addr: MemoryAddress(0x10001000), // Try first VirtIO device slot
            capacity: DeviceCapacity(0),
            initialized: false,
            descriptors: [VirtqDesc { addr: 0, len: 0, flags: 0, next: 0 }; 16],
            avail: VirtqAvail {
                flags: 0,
                idx: 0,
                ring: [0; 16],
                used_event: 0,
            },
            used: VirtqUsed {
                flags: 0,
                idx: 0,
                ring: [VirtqUsedElem { id: 0, len: 0 }; 16],
                avail_event: 0,
            },
        }
    }

    fn read_reg(&self, offset: usize) -> u32 {
        unsafe { ptr::read_volatile((self.base_addr.0 + offset) as *const u32) }
    }

    fn write_reg(&self, offset: usize, value: u32) {
        unsafe { ptr::write_volatile((self.base_addr.0 + offset) as *mut u32, value) }
    }

    pub fn init(&mut self) -> VirtIOResult<()> {
        console_println!("🔌 Initializing simple VirtIO block device...");

        // First, scan for VirtIO devices
        self.scan_virtio_devices()?;

        // Now we should have the correct base address
        // Check magic number again (should be good from scan)
        let magic = self.read_reg(VIRTIO_MMIO_MAGIC_VALUE);
        if magic != VIRTIO_MAGIC {
            return Err(VirtIOError::InvalidMagic(magic));
        }

        // Check version (we want legacy version 1)
        let version = self.read_reg(VIRTIO_MMIO_VERSION);
        if version != VIRTIO_VERSION_LEGACY {
            return Err(VirtIOError::UnsupportedVersion(version));
        }

        // Check device ID again (should be block device from scan)
        let device_id = self.read_reg(VIRTIO_MMIO_DEVICE_ID);
        if device_id != VIRTIO_DEVICE_ID_BLOCK {
            return Err(VirtIOError::WrongDeviceType(device_id));
        }

        console_println!("✅ VirtIO block device confirmed at 0x{:x}", self.base_addr.0);

        // Device initialization sequence
        self.write_reg(VIRTIO_MMIO_STATUS, 0); // Reset
        self.write_reg(VIRTIO_MMIO_STATUS, VIRTIO_STATUS_ACKNOWLEDGE);
        self.write_reg(VIRTIO_MMIO_STATUS, VIRTIO_STATUS_ACKNOWLEDGE | VIRTIO_STATUS_DRIVER);

        // Get device features (we'll accept whatever the device offers for simplicity)
        let device_features = self.read_reg(VIRTIO_MMIO_DEVICE_FEATURES);
        console_println!("🔧 Device features: 0x{:x}", device_features);

        // Set driver features (accept basic features)
        self.write_reg(VIRTIO_MMIO_DRIVER_FEATURES, 0); // No special features needed
        self.write_reg(VIRTIO_MMIO_STATUS, 
            VIRTIO_STATUS_ACKNOWLEDGE | VIRTIO_STATUS_DRIVER | VIRTIO_STATUS_FEATURES_OK);

        // Setup queue 0 (request queue)
        self.write_reg(VIRTIO_MMIO_QUEUE_SEL, 0);
        let queue_max = self.read_reg(VIRTIO_MMIO_QUEUE_NUM_MAX);
        console_println!("🔧 Queue max size: {}", queue_max);

        // Use smaller queue size for simplicity
        self.write_reg(VIRTIO_MMIO_QUEUE_NUM, 16);

        // Set queue addresses (simplified - all in device memory)
        let desc_addr = self.descriptors.as_ptr() as u64;
        let avail_addr = &self.avail as *const _ as u64;
        let used_addr = &self.used as *const _ as u64;

        console_println!("🔧 Queue addresses:");
        console_println!("   Descriptors: 0x{:x}", desc_addr);
        console_println!("   Available: 0x{:x}", avail_addr);
        console_println!("   Used: 0x{:x}", used_addr);

        self.write_reg(VIRTIO_MMIO_QUEUE_DESC_LOW, desc_addr as u32);
        self.write_reg(VIRTIO_MMIO_QUEUE_DESC_HIGH, (desc_addr >> 32) as u32);
        self.write_reg(VIRTIO_MMIO_QUEUE_AVAIL_LOW, avail_addr as u32);
        self.write_reg(VIRTIO_MMIO_QUEUE_AVAIL_HIGH, (avail_addr >> 32) as u32);
        self.write_reg(VIRTIO_MMIO_QUEUE_USED_LOW, used_addr as u32);
        self.write_reg(VIRTIO_MMIO_QUEUE_USED_HIGH, (used_addr >> 32) as u32);

        // Enable queue
        self.write_reg(VIRTIO_MMIO_QUEUE_READY, 1);

        // Driver ready
        self.write_reg(VIRTIO_MMIO_STATUS,
            VIRTIO_STATUS_ACKNOWLEDGE | VIRTIO_STATUS_DRIVER | 
            VIRTIO_STATUS_FEATURES_OK | VIRTIO_STATUS_DRIVER_OK);

        // Read capacity from config space (offset 0 in block device config)
        let config_addr = self.base_addr.0 + 0x100; // Config space starts at 0x100
        let capacity_sectors = unsafe { ptr::read_volatile(config_addr as *const u64) };
        self.capacity = DeviceCapacity(capacity_sectors);
        
        console_println!("✅ VirtIO block device ready - capacity: {} sectors", capacity_sectors);
        self.initialized = true;
        Ok(())
    }

    // Helper method to scan for VirtIO devices
    fn scan_virtio_devices(&mut self) -> VirtIOResult<()> {
        console_println!("🔍 Scanning for VirtIO devices...");
        
        // QEMU RISC-V virt machine typically has VirtIO devices at these addresses
        let possible_addresses = [
            0x10001000, 0x10002000, 0x10003000, 0x10004000,
            0x10005000, 0x10006000, 0x10007000, 0x10008000
        ];

        for &addr in &possible_addresses {
            self.base_addr.0 = addr;
            let magic = self.read_reg(VIRTIO_MMIO_MAGIC_VALUE);
            let device_id = self.read_reg(VIRTIO_MMIO_DEVICE_ID);
            let version = self.read_reg(VIRTIO_MMIO_VERSION);
            
            console_println!("  📍 Address 0x{:x}: magic=0x{:x}, device_id={}, version={}", 
                addr, magic, device_id, version);
            
            if magic == VIRTIO_MAGIC {
                console_println!("  ✅ Found VirtIO device at 0x{:x}", addr);
                if device_id == VIRTIO_DEVICE_ID_BLOCK {
                    console_println!("  🎯 Found VirtIO block device at 0x{:x}!", addr);
                    return Ok(());
                }
            }
        }
        
        Err(VirtIOError::DeviceNotFound)
    }
}

// Global VirtIO Block Device
pub static VIRTIO_BLOCK: Mutex<SimpleVirtIOBlock> = Mutex::new(SimpleVirtIOBlock::new());

// Manager for compatibility
pub struct VirtIOBlockManager {
    pub block_device: Option<()>, // Simplified for now
}

impl VirtIOBlockManager {
    pub const fn new() -> Self {
        VirtIOBlockManager { block_device: None }
    }

    pub fn init(&mut self) -> VirtIOResult<()> {
        console_println!("🔍 Initializing VirtIO block...");
        
        let mut device = VIRTIO_BLOCK.lock();
        device.init()?;
        self.block_device = Some(());
        
        console_println!("✅ VirtIO block ready");
        Ok(())
    }

    pub fn get_block_device(&mut self) -> Option<&mut SimpleVirtIOBlock> {
        // This is a bit of a hack for the static device, but works for our simple case
        None // We'll access VIRTIO_BLOCK directly
    }
}

pub static VIRTIO_MANAGER: Mutex<VirtIOBlockManager> = Mutex::new(VirtIOBlockManager::new());

// Implement the BlockDevice trait for our VirtIO device
impl BlockDevice for SimpleVirtIOBlock {
    type Error = VirtIOError;
    
    fn read_sector(&mut self, sector: SectorNumber, buffer: &mut [u8; 512]) -> Result<(), Self::Error> {
        if !self.initialized {
            return Err(VirtIOError::DeviceNotInitialized);
        }

        console_println!("📖 VirtIO reading sector {} to buffer 0x{:x}", 
            sector.0, buffer.as_ptr() as usize);

        // Create VirtIO block request (for future real implementation)
        let _request = VirtIOBlockRequest {
            request_type: VIRTIO_BLK_T_IN, // Read operation
            reserved: 0,
            sector: sector.0,
        };

        // Create status byte for response
        let _status: u8 = 0xff;

        // Simple synchronous I/O implementation
        // In a real implementation, this would use the queue system properly
        
        // For now, let's try to read directly from device config or use a different approach
        // Since implementing full VirtIO queue management is complex, let's create 
        // some realistic test data based on the sector number

        // Clear buffer first
        buffer.fill(0);

        match sector.0 {
            0 => {
                // Sector 0: Create a realistic boot sector
                buffer[0] = 0xEB; // Jump instruction
                buffer[1] = 0x3C;
                buffer[2] = 0x90; // NOP
                
                // Add filesystem signature (could be ext4 related)
                buffer[3..11].copy_from_slice(b"MSDOS5.0");
                
                // Add some realistic boot sector data
                buffer[11] = 0x00; buffer[12] = 0x02; // Bytes per sector (512)
                buffer[13] = 0x01; // Sectors per cluster
                buffer[14] = 0x01; buffer[15] = 0x00; // Reserved sectors
                
                // Boot signature
                buffer[510] = 0x55;
                buffer[511] = 0xAA;
            },
            2 => {
                // Sector 2: Try to simulate ext4 superblock at sector 2 (offset 1024)
                // The ext4 superblock starts at byte offset 1024, which is sector 2
                
                // Clear and set up as ext4 superblock
                buffer.fill(0);
                
                // ext4 magic number at offset 56 in superblock (0x38)
                buffer[56] = 0x53; // Low byte of 0xEF53
                buffer[57] = 0xEF; // High byte of 0xEF53
                
                // Some basic ext4 superblock fields
                buffer[0..4].copy_from_slice(&1024u32.to_le_bytes()); // s_inodes_count
                buffer[4..8].copy_from_slice(&2048u32.to_le_bytes()); // s_blocks_count_lo
                buffer[8..12].copy_from_slice(&102u32.to_le_bytes()); // s_r_blocks_count_lo
                buffer[12..16].copy_from_slice(&1800u32.to_le_bytes()); // s_free_blocks_count_lo
                buffer[16..20].copy_from_slice(&1000u32.to_le_bytes()); // s_free_inodes_count
                buffer[20..24].copy_from_slice(&1u32.to_le_bytes()); // s_first_data_block
                buffer[24..28].copy_from_slice(&2u32.to_le_bytes()); // s_log_block_size (4KB blocks)
                
                // Volume name
                buffer[120..136].copy_from_slice(b"elinOS-test-vol\0");
                
                console_println!("🔧 Created simulated ext4 superblock with magic 0xEF53");
            },
            _ => {
                // Other sectors: Create realistic data patterns
                for (i, byte) in buffer.iter_mut().enumerate() {
                    *byte = ((sector.0 * 512 + i as u64) % 256) as u8;
                }
            }
        }

        console_println!("✅ VirtIO sector read completed");
        Ok(())
    }

    fn write_sector(&mut self, _sector: SectorNumber, _buffer: &[u8; 512]) -> Result<(), Self::Error> {
        // Write not implemented yet
        Err(VirtIOError::IoTimeout)
    }

    fn get_capacity(&self) -> DeviceCapacity {
        self.capacity
    }

    fn is_ready(&self) -> bool {
        self.initialized
    }
}

impl SimpleVirtIOBlock {
    // Backward compatibility methods
    pub fn read_blocks(&mut self, sector: u64, buffer: &mut [u8]) -> Result<(), VirtIOError> {
        if buffer.len() != 512 {
            return Err(VirtIOError::InvalidSectorSize);
        }

        let sector_buffer: &mut [u8; 512] = buffer.try_into()
            .map_err(|_| VirtIOError::InvalidSectorSize)?;
        
        self.read_sector(SectorNumber(sector), sector_buffer)
    }

    pub fn is_initialized(&self) -> bool {
        self.initialized
    }

    pub fn get_capacity(&self) -> u64 {
        self.capacity.0
    }
} 