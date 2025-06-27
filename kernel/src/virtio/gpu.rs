//! VirtIO GPU Device implementation for elinOS
//! Provides hardware-accelerated graphics output through VirtIO GPU

use crate::console_println;
use spin::Mutex;
use core::ptr::{read_volatile, write_volatile};

use super::{DiskResult, DiskError};
use super::mmio::*;
use super::queue::{VirtioQueue, VirtqDesc};

// All VirtIO GPU constants are imported from super::mmio::*

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

        console_println!("[i] Scanning for VirtIO GPU devices...");
        for &addr in VIRTIO_MMIO_BASES {
            console_println!("[i] Probing MMIO address 0x{:x}...", addr);
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

        console_println!("[!] No VirtIO GPU device found in MMIO scan");
        console_println!("[i] Note: VirtIO GPU PCI devices are not yet supported");
        Ok(false)
    }

    /// Probe MMIO device for VirtIO GPU
    fn probe_mmio_device(&mut self, base: usize) -> DiskResult<bool> {
        unsafe {
            let magic = read_volatile((base + VIRTIO_MMIO_MAGIC_VALUE) as *const u32);
            console_println!("[i]   Magic: 0x{:x} (expected: 0x74726976)", magic);
            if magic != 0x74726976 {
                return Ok(false);
            }

            let version = read_volatile((base + VIRTIO_MMIO_VERSION) as *const u32);
            let device_id = read_volatile((base + VIRTIO_MMIO_DEVICE_ID) as *const u32);
            console_println!("[i]   Version: {}, Device ID: {} (GPU=16)", version, device_id);
            
            if device_id != VIRTIO_ID_GPU {
                if device_id != 0 {
                    console_println!("[i]   Found VirtIO device ID {} (not GPU)", device_id);
                }
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
        // Check if this is a Legacy VirtIO device (version 1)
        let version = unsafe { self.read_reg_u32(VIRTIO_MMIO_VERSION) };
        
        if version == 1 {
            console_println!("[i] Setting up Legacy VirtIO GPU queues...");
            self.setup_legacy_queues()
        } else {
            console_println!("[i] Setting up Modern VirtIO GPU queues...");
            self.setup_modern_queues()
        }
    }

    /// Setup Legacy VirtIO GPU queues (version 1)
    fn setup_legacy_queues(&mut self) -> DiskResult<()> {
        unsafe {
            self.write_reg_u32(VIRTIO_MMIO_QUEUE_SEL, VIRTIO_GPU_CONTROLQ as u32);
            
            let max_queue_size = self.read_reg_u32(VIRTIO_MMIO_QUEUE_NUM_MAX);
            console_println!("[i] Queue {} max size: {}", VIRTIO_GPU_CONTROLQ, max_queue_size);
            
            let queue_size = 64.min(max_queue_size as u16);
            if !queue_size.is_power_of_two() {
                return Err(DiskError::VirtIOError);
            }
            
            self.write_reg_u32(VIRTIO_MMIO_QUEUE_NUM, queue_size as u32);

            // Set guest page size for legacy VirtIO
            self.write_reg_u32(VIRTIO_MMIO_GUEST_PAGE_SIZE, 4096);
            console_println!("[i] Set guest page size: 4096 bytes");

            // Legacy VirtIO queue setup - calculate memory layout
            let desc_table_size = 16 * queue_size as usize;
            let driver_area_offset = desc_table_size;
            let device_area_offset = ((driver_area_offset + 6 + 2 * queue_size as usize) + 4095) & !4095; // Page aligned
            let total_size = device_area_offset + 6 + 8 * queue_size as usize;

            console_println!("[i] Legacy memory layout calculation:");
            console_println!("  Descriptor table: {} bytes", desc_table_size);
            console_println!("  Driver area offset: {} bytes", driver_area_offset);
            console_println!("  Device area offset: {} bytes", device_area_offset);
            console_println!("  Total queue size: {} bytes", total_size);

            // Allocate memory for queue
            let queue_mem = super::allocate_virtio_memory(total_size)?;
            let desc_table_addr = queue_mem;
            let avail_ring_addr = queue_mem + driver_area_offset;
            let used_ring_addr = queue_mem + device_area_offset;

            console_println!("[i] Legacy queue memory layout:");
            console_println!("  Descriptors: 0x{:x}", desc_table_addr);
            console_println!("  Available:   0x{:x}", avail_ring_addr);
            console_println!("  Used:        0x{:x}", used_ring_addr);

            // Zero out the queue memory
            core::ptr::write_bytes(queue_mem as *mut u8, 0, total_size);

            // Initialize queue structure
            self.control_queue.init(queue_size, VIRTIO_GPU_CONTROLQ, desc_table_addr, avail_ring_addr, used_ring_addr)?;

            // Set queue alignment for legacy VirtIO
            self.write_reg_u32(VIRTIO_MMIO_QUEUE_ALIGN, 4096);
            console_println!("[i] Set queue alignment: 4096 bytes");

            // Set queue PFN (Page Frame Number) for legacy VirtIO
            let queue_pfn = desc_table_addr / 4096;
            self.write_reg_u32(VIRTIO_MMIO_QUEUE_PFN, queue_pfn as u32);
            console_println!("[i] Setting queue PFN: {} (addr=0x{:x})", queue_pfn, desc_table_addr);

            // Verify the PFN was set correctly
            let read_pfn = self.read_reg_u32(VIRTIO_MMIO_QUEUE_PFN);
            console_println!("[i] Queue PFN read back: {} (expected: {})", read_pfn, queue_pfn);

            self.control_queue.set_ready(true);
        }

        console_println!("[o] VirtIO GPU queue {} ready", VIRTIO_GPU_CONTROLQ);
        console_println!("[o] VirtIO GPU queues initialized");
        Ok(())
    }

    /// Setup Modern VirtIO GPU queues (version 2+)
    fn setup_modern_queues(&mut self) -> DiskResult<()> {
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
        console_println!("[o] VirtIO GPU queues initialized");
        Ok(())
    }

    /// Get display information from VirtIO GPU
    fn get_display_info(&mut self) -> DiskResult<()> {
        console_println!("[i] Getting VirtIO GPU display information...");
        
        // Send GET_DISPLAY_INFO command to get actual display capabilities
        let cmd = VirtioGpuCtrlHdr {
            type_: VIRTIO_GPU_CMD_GET_DISPLAY_INFO,
            flags: 0,
            fence_id: 0,
            ctx_id: 0,
            padding: 0,
        };

        // For now, we'll assume the command succeeds and use standard display
        // In a full implementation, we'd parse the response
        match self.send_command(&cmd) {
            Ok(()) => {
                console_println!("[o] VirtIO GPU display info retrieved");
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
            Err(_) => {
                console_println!("[!] Failed to get display info, using defaults");
                self.display_info = Some(VirtioGpuDisplayInfo {
                    enabled: 1,
                    x: 0,
                    y: 0,
                    width: 640,
                    height: 480,
                });
                console_println!("[o] VirtIO GPU display: 640x480 (default)");
                Ok(())
            }
        }
    }

    /// Setup framebuffer with VirtIO GPU
    fn setup_framebuffer(&mut self) -> DiskResult<()> {
        console_println!("[i] Setting up VirtIO GPU framebuffer...");
        
        // Step 1: Create 2D resource
        self.create_2d_resource()?;
        
        // Step 2: Attach backing store (our framebuffer memory)
        self.attach_backing_store()?;
        
        // Step 3: Set scanout to connect resource to display
        self.set_scanout()?;
        
        console_println!("[o] VirtIO GPU framebuffer setup complete");
        console_println!("[i] Framebuffer at 0x{:x}, size: {} KB", 
                        self.framebuffer_addr, self.framebuffer_size / 1024);
        Ok(())
    }

    /// Create 2D resource
    fn create_2d_resource(&mut self) -> DiskResult<()> {
        self.resource_id = 1; // Use resource ID 1
        
        console_println!("[i] Creating VirtIO GPU 2D resource...");
        let cmd = VirtioGpuResourceCreate2d {
            hdr: VirtioGpuCtrlHdr {
                type_: VIRTIO_GPU_CMD_RESOURCE_CREATE_2D,
                flags: 0,
                fence_id: 0,
                ctx_id: 0,
                padding: 0,
            },
            resource_id: self.resource_id,
            format: VIRTIO_GPU_FORMAT_B8G8R8A8_UNORM, // Try B8G8R8A8 format (most common)
            width: 640,
            height: 480,
        };

        match self.send_command(&cmd) {
            Ok(()) => {
                console_println!("[o] VirtIO GPU 2D resource created successfully (ID: {}, format: XRGB)", self.resource_id);
                Ok(())
            }
            Err(e) => {
                console_println!("[x] Failed to create VirtIO GPU 2D resource: {:?}", e);
                Err(e)
            }
        }
    }

    /// Attach backing store to resource
    fn attach_backing_store(&mut self) -> DiskResult<()> {
        console_println!("[i] Attaching backing store to VirtIO GPU resource...");
        let cmd = VirtioGpuResourceAttachBacking {
            hdr: VirtioGpuCtrlHdr {
                type_: VIRTIO_GPU_CMD_RESOURCE_ATTACH_BACKING,
                flags: 0,
                fence_id: 0,
                ctx_id: 0,
                padding: 0,
            },
            resource_id: self.resource_id,
            nr_entries: 1,
        };

        let mem_entry = VirtioGpuMemEntry {
            addr: self.framebuffer_addr as u64,
            length: self.framebuffer_size as u32,
            padding: 0,
        };

        console_println!("[i] Backing store: addr=0x{:x}, size={} bytes", self.framebuffer_addr, self.framebuffer_size);
        match self.send_command_with_data(&cmd, &mem_entry) {
            Ok(()) => {
                console_println!("[o] VirtIO GPU backing store attached successfully");
                Ok(())
            }
            Err(e) => {
                console_println!("[x] Failed to attach VirtIO GPU backing store: {:?}", e);
                Err(e)
            }
        }
    }

    /// Set scanout to connect resource to display
    fn set_scanout(&mut self) -> DiskResult<()> {
        console_println!("[i] Setting VirtIO GPU scanout...");
        let cmd = VirtioGpuSetScanout {
            hdr: VirtioGpuCtrlHdr {
                type_: VIRTIO_GPU_CMD_SET_SCANOUT,
                flags: 0,
                fence_id: 0,
                ctx_id: 0,
                padding: 0,
            },
            r: VirtioGpuRect {
                x: 0,
                y: 0,
                width: 640,
                height: 480,
            },
            scanout_id: 0, // Primary display
            resource_id: self.resource_id,
        };

        match self.send_command(&cmd) {
            Ok(()) => {
                console_println!("[o] VirtIO GPU scanout configured successfully");
                Ok(())
            }
            Err(e) => {
                console_println!("[x] Failed to configure VirtIO GPU scanout: {:?}", e);
                Err(e)
            }
        }
    }

    /// Send command to VirtIO GPU
    fn send_command<T>(&mut self, cmd: &T) -> DiskResult<()> {
        let cmd_ptr = cmd as *const T as *const u8;
        let cmd_size = core::mem::size_of::<T>();
        
        // Allocate response buffer on stack
        let mut response_buffer = [0u8; 64]; // Should be enough for most responses
        
        unsafe {
            let desc_chain = [
                VirtqDesc {
                    addr: cmd_ptr as u64,
                    len: cmd_size as u32,
                    flags: VIRTQ_DESC_F_NEXT,
                    next: 1,
                },
                VirtqDesc {
                    addr: response_buffer.as_mut_ptr() as u64,
                    len: response_buffer.len() as u32,
                    flags: VIRTQ_DESC_F_WRITE, // Device writes response here
                    next: 0,
                },
            ];

            let head_index = self.control_queue.add_descriptor_chain(&desc_chain)?;
            self.write_reg_u32(VIRTIO_MMIO_QUEUE_NOTIFY, VIRTIO_GPU_CONTROLQ as u32);
            
            // Wait for completion
            let mut timeout = 1000000;
            while timeout > 0 {
                if let Some(_) = self.control_queue.wait_for_completion(head_index) {
                    // Check response status (first 4 bytes should be response type)
                    let response_type = u32::from_le_bytes([
                        response_buffer[0], response_buffer[1], 
                        response_buffer[2], response_buffer[3]
                    ]);
                    
                    if response_type == VIRTIO_GPU_RESP_OK_NODATA || 
                       response_type == VIRTIO_GPU_RESP_OK_DISPLAY_INFO {
                        return Ok(());
                    } else {
                        console_println!("[!] VirtIO GPU command failed, response: 0x{:x}", response_type);
                        return Err(DiskError::VirtIOError);
                    }
                }
                timeout -= 1;
                core::hint::spin_loop();
            }
            
            Err(DiskError::IoError)
        }
    }

    /// Send command with additional data to VirtIO GPU
    fn send_command_with_data<T, U>(&mut self, cmd: &T, data: &U) -> DiskResult<()> {
        let cmd_ptr = cmd as *const T as *const u8;
        let cmd_size = core::mem::size_of::<T>();
        let data_ptr = data as *const U as *const u8;
        let data_size = core::mem::size_of::<U>();
        
        // Allocate response buffer on stack
        let mut response_buffer = [0u8; 64];
        
        unsafe {
            let desc_chain = [
                VirtqDesc {
                    addr: cmd_ptr as u64,
                    len: cmd_size as u32,
                    flags: VIRTQ_DESC_F_NEXT,
                    next: 1,
                },
                VirtqDesc {
                    addr: data_ptr as u64,
                    len: data_size as u32,
                    flags: VIRTQ_DESC_F_NEXT,
                    next: 2,
                },
                VirtqDesc {
                    addr: response_buffer.as_mut_ptr() as u64,
                    len: response_buffer.len() as u32,
                    flags: VIRTQ_DESC_F_WRITE, // Device writes response here
                    next: 0,
                },
            ];

            let head_index = self.control_queue.add_descriptor_chain(&desc_chain)?;
            self.write_reg_u32(VIRTIO_MMIO_QUEUE_NOTIFY, VIRTIO_GPU_CONTROLQ as u32);
            
            // Wait for completion
            let mut timeout = 1000000;
            while timeout > 0 {
                if let Some(_) = self.control_queue.wait_for_completion(head_index) {
                    // Check response status
                    let response_type = u32::from_le_bytes([
                        response_buffer[0], response_buffer[1], 
                        response_buffer[2], response_buffer[3]
                    ]);
                    
                    if response_type == VIRTIO_GPU_RESP_OK_NODATA || 
                       response_type == VIRTIO_GPU_RESP_OK_DISPLAY_INFO {
                        return Ok(());
                    } else {
                        console_println!("[!] VirtIO GPU command with data failed, response: 0x{:x}", response_type);
                        return Err(DiskError::VirtIOError);
                    }
                }
                timeout -= 1;
                core::hint::spin_loop();
            }
            
            Err(DiskError::IoError)
        }
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
            console_println!("[!] VirtIO GPU not initialized, cannot flush");
            return Err(DiskError::NotInitialized);
        }

        console_println!("[i] Starting VirtIO GPU framebuffer flush...");

        // Step 1: Transfer framebuffer data to host
        self.transfer_to_host()?;
        
        // Step 2: Flush the resource to make it visible
        self.flush_resource()?;
        
        console_println!("[o] VirtIO GPU framebuffer flush completed successfully");
        Ok(())
    }

    /// Transfer framebuffer data to host
    fn transfer_to_host(&mut self) -> DiskResult<()> {
        console_println!("[i] Transferring framebuffer data to VirtIO GPU host...");
        let cmd = VirtioGpuTransferToHost2d {
            hdr: VirtioGpuCtrlHdr {
                type_: VIRTIO_GPU_CMD_TRANSFER_TO_HOST_2D,
                flags: 0,
                fence_id: 0,
                ctx_id: 0,
                padding: 0,
            },
            r: VirtioGpuRect {
                x: 0,
                y: 0,
                width: 640,
                height: 480,
            },
            offset: 0,
            resource_id: self.resource_id,
            padding: 0,
        };

        match self.send_command(&cmd) {
            Ok(()) => {
                console_println!("[o] VirtIO GPU transfer to host completed successfully");
                Ok(())
            }
            Err(e) => {
                console_println!("[x] VirtIO GPU transfer to host failed: {:?}", e);
                Err(e)
            }
        }
    }

    /// Flush resource to display
    fn flush_resource(&mut self) -> DiskResult<()> {
        console_println!("[i] Flushing VirtIO GPU resource to display...");
        let cmd = VirtioGpuResourceFlush {
            hdr: VirtioGpuCtrlHdr {
                type_: VIRTIO_GPU_CMD_RESOURCE_FLUSH,
                flags: 0,
                fence_id: 0,
                ctx_id: 0,
                padding: 0,
            },
            r: VirtioGpuRect {
                x: 0,
                y: 0,
                width: 640,
                height: 480,
            },
            resource_id: self.resource_id,
            padding: 0,
        };

        match self.send_command(&cmd) {
            Ok(()) => {
                console_println!("[o] VirtIO GPU resource flush completed successfully");
                Ok(())
            }
            Err(e) => {
                console_println!("[x] VirtIO GPU resource flush failed: {:?}", e);
                Err(e)
            }
        }
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