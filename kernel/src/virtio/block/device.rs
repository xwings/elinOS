//! VirtIO Block Device implementation

use spin::Mutex;
use crate::console_println;
use core::{convert::TryInto, result::Result::{Ok, Err}};
use core::ptr::read_volatile;

use super::super::{DiskResult, DiskError, VirtqDesc, VirtqUsedElem, VirtioQueue};
use super::super::mmio::*;
use super::{VIRTIO_BLK_T_IN, VIRTIO_BLK_T_OUT, VIRTIO_BLK_S_OK, VIRTIO_BLK_REQUEST_QUEUE_IDX};

/// VirtIO block request header
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct VirtioBlkReq {
    pub type_: u32,
    pub reserved: u32,
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

/// VirtIO Block Device implementation
pub struct RustVmmVirtIOBlock {
    initialized: bool,
    capacity_sectors: u64,
    mmio_base: usize,
    queue: VirtioQueue,
    device_features: u64,
    driver_features: u64,
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

    pub fn init(&mut self) -> DiskResult<()> {
        if !self.discover_device()? {
            return Err(DiskError::DeviceNotFound);
        }
        
        self.init_device()?;
        self.setup_queue()?;
        self.set_driver_ok()?;
        
        self.initialized = true;
        Ok(())
    }

    fn discover_device(&mut self) -> DiskResult<bool> {
        let mmio_addresses = [
            0x10001000, 0x10002000, 0x10003000, 0x10004000,
            0x10005000, 0x10006000, 0x10007000, 0x10008000,
        ];
        
        for &addr in &mmio_addresses {
            if self.probe_mmio_device(addr)? {
                self.mmio_base = addr;
                return Ok(true);
            }
        }
        
        console_println!("[x] No VirtIO block device found");
        Ok(false)
    }

    fn probe_mmio_device(&mut self, base: usize) -> DiskResult<bool> {
        unsafe {
            let magic = core::ptr::read_volatile((base + VIRTIO_MMIO_MAGIC_VALUE) as *const u32);
            if magic != 0x74726976 {
                return Ok(false);
            }
            
            let version = core::ptr::read_volatile((base + VIRTIO_MMIO_VERSION) as *const u32);
            let device_id = core::ptr::read_volatile((base + VIRTIO_MMIO_DEVICE_ID) as *const u32);
            if device_id != 2 {
                return Ok(false);
            }
            
            if version >= 2 {
                console_println!("[i] Modern VirtIO block device found");
            } else if version == 1 {
                console_println!("[i] Legacy VirtIO block device found");
                self.is_legacy = true;
            } else {
                return Ok(false);
            }
            
            Ok(true)
        }
    }

    fn init_device(&mut self) -> DiskResult<()> {
        unsafe {
            self.write_reg_u32(VIRTIO_MMIO_STATUS, 0);
            self.set_status(VIRTIO_STATUS_ACKNOWLEDGE as u8);
            self.set_status(VIRTIO_STATUS_DRIVER as u8);
            
            if self.is_legacy {
                self.device_features = core::ptr::read_volatile((self.mmio_base + VIRTIO_MMIO_DEVICE_FEATURES) as *const u32) as u64;
                self.driver_features = 0;
                self.write_reg_u32(VIRTIO_MMIO_DRIVER_FEATURES, self.driver_features as u32);
            } else {
                self.write_reg_u32(VIRTIO_MMIO_DEVICE_FEATURES_SEL, 0);
                let features_lo = self.read_reg_u32(VIRTIO_MMIO_DEVICE_FEATURES);
                self.write_reg_u32(VIRTIO_MMIO_DEVICE_FEATURES_SEL, 1);
                let features_hi = self.read_reg_u32(VIRTIO_MMIO_DEVICE_FEATURES);
                
                self.device_features = ((features_hi as u64) << 32) | (features_lo as u64);
                self.driver_features = 0;
                
                self.write_reg_u32(VIRTIO_MMIO_DRIVER_FEATURES_SEL, 0);
                self.write_reg_u32(VIRTIO_MMIO_DRIVER_FEATURES, self.driver_features as u32);
                self.write_reg_u32(VIRTIO_MMIO_DRIVER_FEATURES_SEL, 1);
                self.write_reg_u32(VIRTIO_MMIO_DRIVER_FEATURES, (self.driver_features >> 32) as u32);
                
                self.set_status(VIRTIO_STATUS_FEATURES_OK as u8);
                
                let status = self.read_reg_u32(VIRTIO_MMIO_STATUS);
                if (status & VIRTIO_STATUS_FEATURES_OK) == 0 {
                    return Err(DiskError::VirtIOError);
                }
            }
            
            let capacity_low = self.read_reg_u32(VIRTIO_MMIO_CONFIG);
            let capacity_high = self.read_reg_u32(VIRTIO_MMIO_CONFIG + 4);
            self.capacity_sectors = ((capacity_high as u64) << 32) | (capacity_low as u64);
            
            console_println!("[i] Device capacity: {} sectors", self.capacity_sectors);
        }
        
        Ok(())
    }

    fn setup_queue(&mut self) -> DiskResult<()> {
        unsafe {
            self.write_reg_u32(VIRTIO_MMIO_QUEUE_SEL, 0);
            
            let max_queue_size = self.read_reg_u32(VIRTIO_MMIO_QUEUE_NUM_MAX);
            console_println!("[i] Max queue size: {}", max_queue_size);
            
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
                
                // Allocate page-aligned memory using VirtIO memory manager
                let desc_table_addr = super::super::allocate_virtio_memory(total_size)?;
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
                let desc_table_size = 16 * queue_size as usize;
                let avail_ring_size = 6 + 2 * queue_size as usize;
                let used_ring_size = 6 + 8 * queue_size as usize;
                
                // Calculate the total span of memory used by the modern queue setup
                let modern_used_ring_content_size = 4 + (8 * queue_size as usize);
                let total_size = desc_table_size + avail_ring_size + modern_used_ring_content_size + 64; // Add padding
                
                // Allocate memory using VirtIO memory manager
                let desc_table_addr = super::super::allocate_virtio_memory(total_size)?;
                let avail_ring_addr = desc_table_addr + desc_table_size;
                let used_ring_addr = (avail_ring_addr + avail_ring_size + 3) & !3; // 4-byte aligned

                // Zero out the queue memory region before use
                unsafe {
                    core::ptr::write_bytes(desc_table_addr as *mut u8, 0, total_size);
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
        
        // CRITICAL: Set queue ready AFTER all hardware setup is complete
        self.queue.ready = true;
        Ok(())
    }

    fn set_driver_ok(&mut self) -> DiskResult<()> {
        if self.is_legacy {
            self.write_reg_u32(VIRTIO_MMIO_STATUS, VIRTIO_STATUS_ACKNOWLEDGE as u32 | VIRTIO_STATUS_DRIVER as u32 | VIRTIO_STATUS_DRIVER_OK as u32);
        } else {
            self.write_reg_u32(VIRTIO_MMIO_STATUS, VIRTIO_STATUS_ACKNOWLEDGE as u32 | VIRTIO_STATUS_DRIVER as u32 | VIRTIO_STATUS_FEATURES_OK as u32 | VIRTIO_STATUS_DRIVER_OK as u32);
        }
        
        console_println!("[o] VirtIO device ready");
        Ok(())
    }

    pub fn read_sector(&mut self, sector: u64, buffer: &mut [u8; 512]) -> DiskResult<()> {
        if !self.initialized {
            return Err(DiskError::NotInitialized);
        }
        
        if sector >= self.capacity_sectors {
            return Err(DiskError::InvalidSector);
        }
        
        self.virtio_read_sector(sector, buffer)
    }

    fn virtio_read_sector(&mut self, sector: u64, buffer: &mut [u8; 512]) -> DiskResult<()> {
        let head_index;
        unsafe {
            VIRTIO_REQUEST_BUFFER = VirtioBlkReq::new_read(sector);
            VIRTIO_STATUS_BUFFER = 0xFF;
            
            let desc_chain = [
                VirtqDesc {
                    addr: &VIRTIO_REQUEST_BUFFER as *const _ as u64,
                    len: core::mem::size_of::<VirtioBlkReq>() as u32,
                    flags: VIRTQ_DESC_F_NEXT,
                    next: 1,
                },
                VirtqDesc {
                    addr: VIRTIO_DATA_BUFFER.as_mut_ptr() as u64,
                    len: 512,
                    flags: VIRTQ_DESC_F_WRITE | VIRTQ_DESC_F_NEXT,
                    next: 2,
                },
                VirtqDesc {
                    addr: &mut VIRTIO_STATUS_BUFFER as *mut _ as u64,
                    len: 1,
                    flags: VIRTQ_DESC_F_WRITE,
                    next: 0,
                },
            ];
            
            head_index = self.queue.add_descriptor_chain(&desc_chain)?;
            self.write_reg_u32(VIRTIO_MMIO_QUEUE_NOTIFY, self.queue.queue_index as u32);
        }
            
        let mut timeout = 2000000;
        
        loop {
            if timeout <= 0 {
                return Err(DiskError::IoError);
            }

            if let Some(_) = self.queue.wait_for_completion(head_index) {
                if unsafe { VIRTIO_STATUS_BUFFER } == VIRTIO_BLK_S_OK {
                    unsafe { buffer.copy_from_slice(&VIRTIO_DATA_BUFFER); }
                    return Ok(());
                } else {
                    return Err(DiskError::ReadError);
                }
            }
            
            timeout -= 1;
            core::hint::spin_loop();
        }
    }

    pub fn write_sector(&mut self, sector: u64, buffer: &[u8; 512]) -> DiskResult<()> {
        if !self.initialized {
            return Err(DiskError::NotInitialized);
        }
        self.virtio_write_sector(sector, buffer)
    }

    fn virtio_write_sector(&mut self, sector: u64, buffer: &[u8; 512]) -> DiskResult<()> {
        let head_index;
        unsafe {
            VIRTIO_REQUEST_BUFFER = VirtioBlkReq::new_write(sector);
            VIRTIO_DATA_BUFFER.copy_from_slice(buffer);
            VIRTIO_STATUS_BUFFER = 0xFF;

            let desc_chain = [
                VirtqDesc {
                    addr: &VIRTIO_REQUEST_BUFFER as *const _ as u64,
                    len: core::mem::size_of::<VirtioBlkReq>() as u32,
                    flags: VIRTQ_DESC_F_NEXT,
                    next: 1,
                },
                VirtqDesc {
                    addr: VIRTIO_DATA_BUFFER.as_ptr() as u64,
                    len: VIRTIO_DATA_BUFFER.len() as u32,
                    flags: VIRTQ_DESC_F_NEXT,
                    next: 2,
                },
                VirtqDesc {
                    addr: &mut VIRTIO_STATUS_BUFFER as *mut _ as u64,
                    len: 1,
                    flags: VIRTQ_DESC_F_WRITE,
                    next: 0,
                },
            ];

            head_index = self.queue.add_descriptor_chain(&desc_chain)?;
            self.write_reg_u32(VIRTIO_MMIO_QUEUE_NOTIFY, self.queue.queue_index as u32); 
        }

        let mut timeout = 2000000;
        
        loop {
            if timeout <= 0 {
                return Err(DiskError::IoError);
            }

            if let Some(_) = self.queue.wait_for_completion(head_index) {
                if unsafe { VIRTIO_STATUS_BUFFER } == VIRTIO_BLK_S_OK {
                    return Ok(());
                } else {
                    return Err(DiskError::WriteError); 
                }
            }
            
            timeout -= 1;
            core::hint::spin_loop(); 
        }
    }
    
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }
    
    pub fn get_capacity(&self) -> u64 {
        self.capacity_sectors
    }
    
    pub fn read_blocks(&mut self, start_sector: u64, buffer: &mut [u8]) -> DiskResult<()> {
        if buffer.len() == 0 {
            return Ok(());
        }
        if buffer.len() % 512 != 0 {
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
            return Ok(());
        }
        if buffer.len() % 512 != 0 {
            return Err(DiskError::BufferTooSmall);
        }
        let num_sectors = buffer.len() / 512;
        for i in 0..num_sectors {
            let sector = start_sector + i as u64;
            let offset = i * 512;
            let sector_buffer_slice = &buffer[offset..offset + 512];
            let sector_buffer_array: &[u8; 512] = sector_buffer_slice.try_into()
                .expect("Slice to array conversion failed in write_blocks despite checks");
            self.write_sector(sector, sector_buffer_array)?;
        }
        Ok(())
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
        self.write_reg_u32(VIRTIO_MMIO_STATUS, current_status | (status_val as u32));
    }
}

// Static buffers for VirtIO operations
static mut VIRTIO_REQUEST_BUFFER: VirtioBlkReq = VirtioBlkReq { type_: 0, reserved: 0, sector: 0 };
static mut VIRTIO_DATA_BUFFER: [u8; 512] = [0; 512];
static mut VIRTIO_STATUS_BUFFER: u8 = 0;

// Global instance
pub static VIRTIO_BLK: Mutex<RustVmmVirtIOBlock> = Mutex::new(RustVmmVirtIOBlock::new());

/// Initialize the VirtIO block device
pub fn init_virtio_blk() -> DiskResult<()> {
    console_println!("[i] Initializing rust-vmm style VirtIO Block Device...");
    
    let mut device = VIRTIO_BLK.lock();
    device.init()
}

/// Initialize VirtIO block device with specific address
pub fn init_with_address(base_addr: usize) -> bool {
    console_println!("[i] Trying VirtIO device at 0x{:08x}", base_addr);
    
    unsafe {
        let magic = core::ptr::read_volatile(base_addr as *const u32);
        if magic != 0x74726976 {
            return false;
        }
        
        let device_id = core::ptr::read_volatile((base_addr + VIRTIO_MMIO_DEVICE_ID) as *const u32);
        
        if device_id != 2 {
            return false;
        }
        
        let mut device = RustVmmVirtIOBlock::new();
        device.mmio_base = base_addr;
        if device.init().is_ok() {
            *VIRTIO_BLK.lock() = device;
            return true;
        }
    }
    
    false
} 