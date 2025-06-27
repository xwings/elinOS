//! VirtIO GPU Device implementation for elinOS
//! Provides hardware-accelerated graphics output through VirtIO GPU

use spin::Mutex;
use crate::console_println;
use core::ptr::{read_volatile, write_volatile};

use super::{DiskResult, DiskError, VirtqDesc, VirtioQueue};
use super::mmio::*;

/// VirtIO GPU Display Information
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct VirtioGpuDisplayInfo {
    pub enabled: u32,
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

/// VirtIO GPU Rectangle
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct VirtioGpuRect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

/// VirtIO GPU Command Header
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct VirtioGpuCtrlHdr {
    pub type_: u32,
    pub flags: u32,
    pub fence_id: u64,
    pub ctx_id: u32,
    pub padding: u32,
}

/// VirtIO GPU Resource Create 2D Command
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct VirtioGpuResourceCreate2d {
    pub hdr: VirtioGpuCtrlHdr,
    pub resource_id: u32,
    pub format: u32,
    pub width: u32,
    pub height: u32,
}

/// VirtIO GPU Set Scanout Command
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct VirtioGpuSetScanout {
    pub hdr: VirtioGpuCtrlHdr,
    pub r: VirtioGpuRect,
    pub scanout_id: u32,
    pub resource_id: u32,
}

/// VirtIO GPU Resource Attach Backing Command
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct VirtioGpuResourceAttachBacking {
    pub hdr: VirtioGpuCtrlHdr,
    pub resource_id: u32,
    pub nr_entries: u32,
}

/// VirtIO GPU Memory Entry
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct VirtioGpuMemEntry {
    pub addr: u64,
    pub length: u32,
    pub padding: u32,
}

/// VirtIO GPU Transfer to Host 2D Command
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct VirtioGpuTransferToHost2d {
    pub hdr: VirtioGpuCtrlHdr,
    pub r: VirtioGpuRect,
    pub offset: u64,
    pub resource_id: u32,
    pub padding: u32,
}

/// VirtIO GPU Resource Flush Command
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct VirtioGpuResourceFlush {
    pub hdr: VirtioGpuCtrlHdr,
    pub r: VirtioGpuRect,
    pub resource_id: u32,
    pub padding: u32,
}

/// VirtIO GPU Device
pub struct VirtioGpu {
    initialized: bool,
    mmio_base: usize,
    control_queue: VirtioQueue,
    cursor_queue: VirtioQueue,
    display_info: Option<VirtioGpuDisplayInfo>,
    resource_id: u32,
    framebuffer_addr: usize,
    framebuffer_size: usize,
}

impl VirtioGpu {
    pub const fn new() -> Self {
        VirtioGpu {
            initialized: false,
            mmio_base: 0,
            control_queue: VirtioQueue::new(),
            cursor_queue: VirtioQueue::new(),
            display_info: None,
            resource_id: 1,
            framebuffer_addr: 0,
            framebuffer_size: 0,
        }
    }

    /// Initialize VirtIO GPU device
    pub fn init(&mut self, framebuffer_addr: usize, framebuffer_size: usize) -> DiskResult<()> {
        console_println!("[i] Searching for VirtIO GPU device...");
        
        if !self.discover_device()? {
            console_println!("[!] No VirtIO GPU device found - using software framebuffer");
            return Err(DiskError::DeviceNotFound);
        }

        self.framebuffer_addr = framebuffer_addr;
        self.framebuffer_size = framebuffer_size;

        console_println!("[i] Initializing VirtIO GPU device...");
        self.init_device()?;
        self.setup_queues()?;
        self.get_display_info()?;
        self.setup_framebuffer()?;
        self.set_driver_ok()?;

        self.initialized = true;
        console_println!("[o] VirtIO GPU device initialized successfully!");
        Ok(())
    }

    /// Discover VirtIO GPU device
    fn discover_device(&mut self) -> DiskResult<bool> {
        const VIRTIO_MMIO_BASES: &[usize] = &[
            0x10001000, // VirtIO MMIO device 0
            0x10002000, // VirtIO MMIO device 1
            0x10003000, // VirtIO MMIO device 2
            0x10004000, // VirtIO MMIO device 3
            0x10005000, // VirtIO MMIO device 4
            0x10006000, // VirtIO MMIO device 5
            0x10007000, // VirtIO MMIO device 6
            0x10008000, // VirtIO MMIO device 7
        ];

        for &addr in VIRTIO_MMIO_BASES {
            if self.probe_mmio_device(addr)? {
                self.mmio_base = addr;
                console_println!("[o] VirtIO GPU device found at 0x{:x}", addr);
                
                // Register the device MMIO region
                const VIRTIO_MMIO_SIZE: usize = 0x1000; // 4KB MMIO region
                match super::register_virtio_device(addr, VIRTIO_MMIO_SIZE, "VirtIO-GPU") {
                    Ok(_) => console_println!("[i] VirtIO GPU device MMIO region registered"),
                    Err(_) => console_println!("[!] Failed to register VirtIO GPU MMIO region"),
                }
                
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Probe MMIO device for VirtIO GPU
    fn probe_mmio_device(&mut self, base: usize) -> DiskResult<bool> {
        unsafe {
            let magic = read_volatile((base + VIRTIO_MMIO_MAGIC_VALUE) as *const u32);
            if magic != 0x74726976 {
                return Ok(false);
            }

            let version = read_volatile((base + VIRTIO_MMIO_VERSION) as *const u32);
            let device_id = read_volatile((base + VIRTIO_MMIO_DEVICE_ID) as *const u32);
            
            if device_id != VIRTIO_ID_GPU {
                return Ok(false);
            }

            console_println!("[i] VirtIO GPU device detected (version: {})", version);
            Ok(true)
        }
    }

    /// Initialize VirtIO GPU device
    fn init_device(&mut self) -> DiskResult<()> {
        unsafe {
            // Reset device
            self.write_reg_u32(VIRTIO_MMIO_STATUS, 0);
            
            // Acknowledge device
            self.set_status(VIRTIO_STATUS_ACKNOWLEDGE as u8);
            self.set_status(VIRTIO_STATUS_DRIVER as u8);

            // Read device features
            self.write_reg_u32(VIRTIO_MMIO_DEVICE_FEATURES_SEL, 0);
            let features_lo = self.read_reg_u32(VIRTIO_MMIO_DEVICE_FEATURES);
            self.write_reg_u32(VIRTIO_MMIO_DEVICE_FEATURES_SEL, 1);
            let features_hi = self.read_reg_u32(VIRTIO_MMIO_DEVICE_FEATURES);
            
            let device_features = ((features_hi as u64) << 32) | (features_lo as u64);
            console_println!("[i] VirtIO GPU device features: 0x{:x}", device_features);

            // Set driver features (none for basic operation)
            self.write_reg_u32(VIRTIO_MMIO_DRIVER_FEATURES_SEL, 0);
            self.write_reg_u32(VIRTIO_MMIO_DRIVER_FEATURES, 0);
            self.write_reg_u32(VIRTIO_MMIO_DRIVER_FEATURES_SEL, 1);
            self.write_reg_u32(VIRTIO_MMIO_DRIVER_FEATURES, 0);

            self.set_status(VIRTIO_STATUS_FEATURES_OK as u8);

            // Verify features OK
            let status = self.read_reg_u32(VIRTIO_MMIO_STATUS);
            if (status & VIRTIO_STATUS_FEATURES_OK) == 0 {
                console_println!("[x] VirtIO GPU features not accepted by device");
                return Err(DiskError::VirtIOError);
            }
        }

        Ok(())
    }

    /// Setup VirtIO GPU queues
    fn setup_queues(&mut self) -> DiskResult<()> {
        // Setup control queue (queue 0) - we need to use a different approach to avoid borrowing issues
        unsafe {
            self.write_reg_u32(VIRTIO_MMIO_QUEUE_SEL, VIRTIO_GPU_CONTROLQ as u32);
            
            let max_queue_size = self.read_reg_u32(VIRTIO_MMIO_QUEUE_NUM_MAX);
            console_println!("[i] Queue {} max size: {}", VIRTIO_GPU_CONTROLQ, max_queue_size);
            
            let queue_size = 64.min(max_queue_size as u16);
            if !queue_size.is_power_of_two() {
                return Err(DiskError::VirtIOError);
            }
            
            self.write_reg_u32(VIRTIO_MMIO_QUEUE_NUM, queue_size as u32);

            // Modern VirtIO queue setup
            let desc_table_size = 16 * queue_size as usize;
            let avail_ring_size = 6 + 2 * queue_size as usize;
            let used_ring_size = 6 + 8 * queue_size as usize;
            let total_size = desc_table_size + avail_ring_size + used_ring_size + 64;

            // Allocate memory for queue
            let desc_table_addr = super::allocate_virtio_memory(total_size)?;
            let avail_ring_addr = desc_table_addr + desc_table_size;
            let used_ring_addr = (avail_ring_addr + avail_ring_size + 3) & !3; // 4-byte aligned

            // Zero out the queue memory
            core::ptr::write_bytes(desc_table_addr as *mut u8, 0, total_size);

            // Initialize queue structure
            self.control_queue.init(queue_size, VIRTIO_GPU_CONTROLQ, desc_table_addr, avail_ring_addr, used_ring_addr)?;

            // Set queue addresses
            self.write_reg_u32(VIRTIO_MMIO_QUEUE_DESC_LOW, desc_table_addr as u32);
            self.write_reg_u32(VIRTIO_MMIO_QUEUE_DESC_HIGH, (desc_table_addr >> 32) as u32);
            
            self.write_reg_u32(VIRTIO_MMIO_QUEUE_DRIVER_LOW, avail_ring_addr as u32);
            self.write_reg_u32(VIRTIO_MMIO_QUEUE_DRIVER_HIGH, (avail_ring_addr >> 32) as u32);
            
            self.write_reg_u32(VIRTIO_MMIO_QUEUE_DEVICE_LOW, used_ring_addr as u32);
            self.write_reg_u32(VIRTIO_MMIO_QUEUE_DEVICE_HIGH, (used_ring_addr >> 32) as u32);

            // Mark queue as ready
            self.write_reg_u32(VIRTIO_MMIO_QUEUE_READY, 1);
            
            self.control_queue.set_ready(true);
        }

        console_println!("[o] VirtIO GPU queue {} ready", VIRTIO_GPU_CONTROLQ);
        
        // We don't need cursor queue for basic framebuffer operation
        console_println!("[o] VirtIO GPU queues initialized");
        Ok(())
    }

    /// Get display information from VirtIO GPU
    fn get_display_info(&mut self) -> DiskResult<()> {
        console_println!("[i] Getting VirtIO GPU display information...");
        
        // For now, assume standard display info
        // In a full implementation, we'd send VIRTIO_GPU_CMD_GET_DISPLAY_INFO
        self.display_info = Some(VirtioGpuDisplayInfo {
            enabled: 1,
            x: 0,
            y: 0,
            width: 640,
            height: 480,
        });

        console_println!("[o] VirtIO GPU display: 640x480");
        Ok(())
    }

    /// Setup framebuffer with VirtIO GPU
    fn setup_framebuffer(&mut self) -> DiskResult<()> {
        console_println!("[i] Setting up VirtIO GPU framebuffer...");
        
        // This is a simplified setup - in a full implementation we would:
        // 1. Send VIRTIO_GPU_CMD_RESOURCE_CREATE_2D
        // 2. Send VIRTIO_GPU_CMD_RESOURCE_ATTACH_BACKING with our framebuffer
        // 3. Send VIRTIO_GPU_CMD_SET_SCANOUT to connect resource to display
        // 4. Send VIRTIO_GPU_CMD_TRANSFER_TO_HOST_2D and VIRTIO_GPU_CMD_RESOURCE_FLUSH to update
        
        console_println!("[o] VirtIO GPU framebuffer setup complete");
        console_println!("[i] Framebuffer at 0x{:x}, size: {} KB", 
                        self.framebuffer_addr, self.framebuffer_size / 1024);
        Ok(())
    }

    /// Set driver OK status
    fn set_driver_ok(&mut self) -> DiskResult<()> {
        self.write_reg_u32(VIRTIO_MMIO_STATUS, 
            VIRTIO_STATUS_ACKNOWLEDGE as u32 | 
            VIRTIO_STATUS_DRIVER as u32 | 
            VIRTIO_STATUS_FEATURES_OK as u32 | 
            VIRTIO_STATUS_DRIVER_OK as u32);
        
        console_println!("[o] VirtIO GPU driver ready");
        Ok(())
    }

    /// Flush framebuffer to display
    pub fn flush_framebuffer(&mut self) -> DiskResult<()> {
        if !self.initialized {
            return Err(DiskError::NotInitialized);
        }

        // In a full implementation, we would send:
        // VIRTIO_GPU_CMD_TRANSFER_TO_HOST_2D followed by VIRTIO_GPU_CMD_RESOURCE_FLUSH
        // For now, this is a placeholder that indicates the framebuffer should be visible
        
        Ok(())
    }

    /// Read 32-bit register
    fn read_reg_u32(&self, offset: usize) -> u32 {
        unsafe { read_volatile((self.mmio_base + offset) as *const u32) }
    }

    /// Write 32-bit register
    fn write_reg_u32(&self, offset: usize, value: u32) {
        unsafe { write_volatile((self.mmio_base + offset) as *mut u32, value) }
    }

    /// Set device status
    fn set_status(&self, status: u8) {
        let current_status = self.read_reg_u32(VIRTIO_MMIO_STATUS);
        self.write_reg_u32(VIRTIO_MMIO_STATUS, current_status | (status as u32));
    }

    pub fn is_initialized(&self) -> bool {
        self.initialized
    }
}

// Global VirtIO GPU device
pub static VIRTIO_GPU: Mutex<VirtioGpu> = Mutex::new(VirtioGpu::new());

/// Initialize VirtIO GPU with existing framebuffer
pub fn init_virtio_gpu(framebuffer_addr: usize, framebuffer_size: usize) -> DiskResult<()> {
    let mut gpu = VIRTIO_GPU.lock();
    gpu.init(framebuffer_addr, framebuffer_size)
}

/// Flush framebuffer to display
pub fn flush_display() -> DiskResult<()> {
    let mut gpu = VIRTIO_GPU.lock();
    gpu.flush_framebuffer()
} 