//! VirtIO Block Device implementation

use spin::Mutex;
use elinos_common::console_println;
use core::{convert::TryInto, result::Result::{Ok, Err}};
use core::ptr::read_volatile;

use super::super::{DiskResult, DiskError, VirtqDesc, VirtqUsedElem, VirtioQueue};
use super::super::mmio::*;
use super::{VIRTIO_BLK_T_IN, VIRTIO_BLK_T_OUT, VIRTIO_BLK_S_OK, VIRTIO_BLK_REQUEST_QUEUE_IDX};

/// VirtIO buffer management - integrated with memory mapping system
use crate::memory::mapping;

/// VirtIO buffer structure for proper memory management
struct VirtioBuffers {
    base_addr: usize,
    request_offset: usize,
    data_offset: usize,
    status_offset: usize,
}

impl VirtioBuffers {
    fn new(base_addr: usize) -> Self {
        VirtioBuffers {
            base_addr,
            request_offset: 0,
            data_offset: 16,      // After 16-byte request
            status_offset: 528,   // After 16-byte request + 512-byte data
        }
    }
    
    fn get_request_buffer(&self) -> *mut VirtioBlkReq {
        (self.base_addr + self.request_offset) as *mut VirtioBlkReq
    }
    
    fn get_data_buffer(&self) -> *mut [u8; 512] {
        (self.base_addr + self.data_offset) as *mut [u8; 512]
    }
    
    fn get_status_buffer(&self) -> *mut u8 {
        (self.base_addr + self.status_offset) as *mut u8
    }
}

/// VirtIO buffer addresses (will be set during initialization)
static mut VIRTIO_BUFFERS: Option<VirtioBuffers> = None;

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
                
                // Register the device MMIO region using our memory mapping API
                const VIRTIO_MMIO_SIZE: usize = 0x1000; // 4KB MMIO region
                match super::super::register_virtio_device(addr, VIRTIO_MMIO_SIZE, "VirtIO-Block") {
                    Ok(_) => {},
                    Err(_) => console_println!("[!] Failed to register VirtIO MMIO region"),
                }
                
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
                // Modern VirtIO block device
            } else if version == 1 {
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
            
        }
        
        Ok(())
    }

    fn setup_queue(&mut self) -> DiskResult<()> {
        unsafe {
            self.write_reg_u32(VIRTIO_MMIO_QUEUE_SEL, 0);
            
            let max_queue_size = self.read_reg_u32(VIRTIO_MMIO_QUEUE_NUM_MAX);
            
            let queue_size = 64.min(max_queue_size as u16);
            if !queue_size.is_power_of_two() {
                return Err(DiskError::VirtIOError);
            }
            
            self.write_reg_u32(VIRTIO_MMIO_QUEUE_NUM, queue_size as u32);
            
            if self.is_legacy {
                // Step 1: Set guest page size (REQUIRED for legacy VirtIO)
                self.write_reg_u32(VIRTIO_MMIO_GUEST_PAGE_SIZE, PAGE_SIZE as u32);
                
                // Step 2: Calculate memory layout following VirtIO spec
                // Legacy VirtIO requires ALL rings to be contiguous and page-aligned
                let desc_table_size = 16 * queue_size as usize; // 16 bytes per descriptor
                let avail_ring_size = 6 + 2 * queue_size as usize; // 6 bytes header + 2 bytes per entry
                let used_ring_size = 6 + 8 * queue_size as usize; // 6 bytes header + 8 bytes per entry
                
                // Calculate aligned layout exactly like rcore-os
                let driver_area_offset = desc_table_size;
                let device_area_offset = align_up(desc_table_size + avail_ring_size);
                let buffer_area_offset = align_up(device_area_offset + used_ring_size);
                let buffer_area_size = 1024; // Space for request + data + status buffers
                let total_size = align_up(buffer_area_offset + buffer_area_size);
                
                
                // Allocate page-aligned memory using VirtIO memory manager
                let desc_table_addr = super::super::allocate_virtio_memory(total_size)?;
                let avail_ring_addr = desc_table_addr + driver_area_offset;
                let used_ring_addr = desc_table_addr + device_area_offset;
                let buffer_area_addr = desc_table_addr + buffer_area_offset;
                
                // Validate memory layout (like rcore-os does)
                if desc_table_addr % PAGE_SIZE != 0 {
                    return Err(DiskError::VirtIOError);
                }
                
                
                // Zero out the queue memory region before use
                unsafe {
                    core::ptr::write_bytes(desc_table_addr as *mut u8, 0, total_size);
                }
                
                // Initialize queue structures
                self.queue.init(queue_size, VIRTIO_BLK_REQUEST_QUEUE_IDX, desc_table_addr, avail_ring_addr, used_ring_addr)?;
                
                // Set up buffer area for VirtIO operations
                unsafe {
                    VIRTIO_BUFFERS = Some(VirtioBuffers::new(buffer_area_addr));
                }
                
                // Step 3: Set queue alignment (power of 2, typically page size)
                let queue_align = PAGE_SIZE as u32;
                self.write_reg_u32(VIRTIO_MMIO_QUEUE_ALIGN, queue_align);
                
                // Step 4: Set queue PFN (Page Frame Number)
                let pfn = (desc_table_addr / PAGE_SIZE) as u32;
                self.write_reg_u32(VIRTIO_MMIO_QUEUE_PFN, pfn);
                
                // Verify the PFN was accepted
                let read_pfn = self.read_reg_u32(VIRTIO_MMIO_QUEUE_PFN);
                
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

                // Allocate buffer area for VirtIO operations (same as legacy)
                const BUFFER_AREA_SIZE: usize = 4096; // Request + Data + Status buffers
                let buffer_area_addr = super::super::allocate_virtio_memory(BUFFER_AREA_SIZE)?;

                // Zero out the queue memory region before use
                unsafe {
                    core::ptr::write_bytes(desc_table_addr as *mut u8, 0, total_size);
                }
                
                // Initialize the queue structure
                self.queue.init(queue_size, VIRTIO_BLK_REQUEST_QUEUE_IDX, desc_table_addr, avail_ring_addr, used_ring_addr)?;
                
                // Set up buffer area for VirtIO operations
                unsafe {
                    VIRTIO_BUFFERS = Some(VirtioBuffers::new(buffer_area_addr));
                }
                
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
            // Initialize request in virtual buffer
            let request_ptr = get_request_buffer();
            *request_ptr = VirtioBlkReq::new_read(sector);
            
            // Initialize status in virtual buffer
            let status_ptr = get_status_buffer();
            *status_ptr = 0xFF;
            
            let desc_chain = [
                VirtqDesc {
                    addr: request_ptr as u64,
                    len: core::mem::size_of::<VirtioBlkReq>() as u32,
                    flags: VIRTQ_DESC_F_NEXT,
                    next: 1,
                },
                VirtqDesc {
                    addr: get_data_buffer() as u64,
                    len: 512,
                    flags: VIRTQ_DESC_F_WRITE | VIRTQ_DESC_F_NEXT,
                    next: 2,
                },
                VirtqDesc {
                    addr: status_ptr as u64,
                    len: 1,
                    flags: VIRTQ_DESC_F_WRITE,
                    next: 0,
                },
            ];
            
            // Debug: Show descriptor addresses
            // console_println!("[DEBUG] VirtIO descriptors (virtual buffers):");
            // console_println!("  Request: 0x{:x} (len={})", desc_chain[0].addr, desc_chain[0].len);
            // console_println!("  Data:    0x{:x} (len={})", desc_chain[1].addr, desc_chain[1].len);
            // console_println!("  Status:  0x{:x} (len={})", desc_chain[2].addr, desc_chain[2].len);
            
            head_index = self.queue.add_descriptor_chain(&desc_chain)?;
            self.write_reg_u32(VIRTIO_MMIO_QUEUE_NOTIFY, self.queue.queue_index as u32);
        }
            
        let mut timeout = 2000000;
        
        loop {
            if timeout <= 0 {
                return Err(DiskError::IoError);
            }

            if let Some(_) = self.queue.wait_for_completion(head_index) {
                unsafe {
                    if *get_status_buffer() == VIRTIO_BLK_S_OK {
                        let data_buffer = &*get_data_buffer();
                        buffer.copy_from_slice(data_buffer);
                        return Ok(());
                    } else {
                        return Err(DiskError::ReadError);
                    }
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
            // Initialize request in virtual buffer
            let request_ptr = get_request_buffer();
            *request_ptr = VirtioBlkReq::new_write(sector);
            
            // Copy data to virtual buffer
            let data_buffer = &mut *get_data_buffer();
            data_buffer.copy_from_slice(buffer);
            
            // Initialize status in virtual buffer
            let status_ptr = get_status_buffer();
            *status_ptr = 0xFF;

            let desc_chain = [
                VirtqDesc {
                    addr: request_ptr as u64,
                    len: core::mem::size_of::<VirtioBlkReq>() as u32,
                    flags: VIRTQ_DESC_F_NEXT,
                    next: 1,
                },
                VirtqDesc {
                    addr: data_buffer.as_ptr() as u64,
                    len: 512,
                    flags: VIRTQ_DESC_F_NEXT,
                    next: 2,
                },
                VirtqDesc {
                    addr: status_ptr as u64,
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
                unsafe {
                    if *get_status_buffer() == VIRTIO_BLK_S_OK {
                        return Ok(());
                    } else {
                        return Err(DiskError::WriteError); 
                    }
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

// Helper functions that use proper buffer management
unsafe fn get_request_buffer() -> *mut VirtioBlkReq {
    VIRTIO_BUFFERS.as_ref().unwrap().get_request_buffer()
}

unsafe fn get_data_buffer() -> *mut [u8; 512] {
    VIRTIO_BUFFERS.as_ref().unwrap().get_data_buffer()
}

unsafe fn get_status_buffer() -> *mut u8 {
    VIRTIO_BUFFERS.as_ref().unwrap().get_status_buffer()
}

// Global instance
pub static VIRTIO_BLK: Mutex<RustVmmVirtIOBlock> = Mutex::new(RustVmmVirtIOBlock::new());

/// Initialize the VirtIO block device
pub fn init_virtio_blk() -> DiskResult<()> {
    let mut device = VIRTIO_BLK.lock();
    device.init()
}

/// Initialize VirtIO block device with specific address
pub fn init_with_address(base_addr: usize) -> bool {
    
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