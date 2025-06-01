use core::fmt::Write;
use spin::Mutex;
use crate::UART;

// VirtIO MMIO device registers
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
const VIRTIO_MMIO_QUEUE_PFN: usize = 0x040;
const VIRTIO_MMIO_QUEUE_NOTIFY: usize = 0x050;
const VIRTIO_MMIO_INTERRUPT_STATUS: usize = 0x060;
const VIRTIO_MMIO_INTERRUPT_ACK: usize = 0x064;
const VIRTIO_MMIO_STATUS: usize = 0x070;

// VirtIO device status
const VIRTIO_STATUS_ACKNOWLEDGE: u32 = 1;
const VIRTIO_STATUS_DRIVER: u32 = 2;
const VIRTIO_STATUS_DRIVER_OK: u32 = 4;
const VIRTIO_STATUS_FEATURES_OK: u32 = 8;

// VirtIO block device type
const VIRTIO_DEVICE_ID_BLOCK: u32 = 2;

// VirtIO block request types
const VIRTIO_BLK_T_IN: u32 = 0;
const VIRTIO_BLK_T_OUT: u32 = 1;

// VirtIO block status
const VIRTIO_BLK_S_OK: u8 = 0;
const VIRTIO_BLK_S_IOERR: u8 = 1;
const VIRTIO_BLK_S_UNSUPP: u8 = 2;

#[repr(C)]
struct VirtIOBlockRequest {
    request_type: u32,
    reserved: u32,
    sector: u64,
}

#[repr(C)]
struct VirtIOBlockDevice {
    base_addr: usize,
    queue_size: u16,
    initialized: bool,
}

impl VirtIOBlockDevice {
    pub const fn new(base_addr: usize) -> Self {
        VirtIOBlockDevice {
            base_addr,
            queue_size: 0,
            initialized: false,
        }
    }

    fn read_reg(&self, offset: usize) -> u32 {
        unsafe {
            let ptr = (self.base_addr + offset) as *const u32;
            ptr.read_volatile()
        }
    }

    fn write_reg(&self, offset: usize, value: u32) {
        unsafe {
            let ptr = (self.base_addr + offset) as *mut u32;
            ptr.write_volatile(value);
        }
    }

    pub fn init(&mut self) -> Result<(), &'static str> {
        // Check magic value
        let magic = self.read_reg(VIRTIO_MMIO_MAGIC_VALUE);
        if magic != 0x74726976 {
            return Err("Invalid VirtIO magic value");
        }

        // Check device ID
        let device_id = self.read_reg(VIRTIO_MMIO_DEVICE_ID);
        if device_id != VIRTIO_DEVICE_ID_BLOCK {
            return Err("Not a block device");
        }

        {
            let mut uart = UART.lock();
            let _ = writeln!(uart, "VirtIO block device found at 0x{:x}", self.base_addr);
        }

        // Reset device
        self.write_reg(VIRTIO_MMIO_STATUS, 0);

        // Acknowledge device
        self.write_reg(VIRTIO_MMIO_STATUS, VIRTIO_STATUS_ACKNOWLEDGE);

        // Set driver status
        self.write_reg(VIRTIO_MMIO_STATUS, VIRTIO_STATUS_ACKNOWLEDGE | VIRTIO_STATUS_DRIVER);

        // Read and negotiate features (for now, we don't need any special features)
        self.write_reg(VIRTIO_MMIO_DRIVER_FEATURES_SEL, 0);
        self.write_reg(VIRTIO_MMIO_DRIVER_FEATURES, 0);

        // Features OK
        self.write_reg(VIRTIO_MMIO_STATUS, 
            VIRTIO_STATUS_ACKNOWLEDGE | VIRTIO_STATUS_DRIVER | VIRTIO_STATUS_FEATURES_OK);

        // Check if features are accepted
        let status = self.read_reg(VIRTIO_MMIO_STATUS);
        if (status & VIRTIO_STATUS_FEATURES_OK) == 0 {
            return Err("Features not accepted");
        }

        // Set up queue
        self.write_reg(VIRTIO_MMIO_QUEUE_SEL, 0);
        let max_queue_size = self.read_reg(VIRTIO_MMIO_QUEUE_NUM_MAX) as u16;
        self.queue_size = if max_queue_size > 128 { 128 } else { max_queue_size };
        self.write_reg(VIRTIO_MMIO_QUEUE_NUM, self.queue_size as u32);

        // For now, we'll use a simple implementation without actual queue setup
        // In a full implementation, you'd allocate DMA-able memory for the queue

        // Driver OK
        self.write_reg(VIRTIO_MMIO_STATUS, 
            VIRTIO_STATUS_ACKNOWLEDGE | VIRTIO_STATUS_DRIVER | 
            VIRTIO_STATUS_FEATURES_OK | VIRTIO_STATUS_DRIVER_OK);

        self.initialized = true;

        {
            let mut uart = UART.lock();
            let _ = writeln!(uart, "VirtIO block device initialized, queue size: {}", self.queue_size);
        }

        Ok(())
    }

    pub fn read_block(&self, _block_num: u64, _buffer: &mut [u8]) -> Result<(), &'static str> {
        if !self.initialized {
            return Err("Device not initialized");
        }

        // This is a simplified implementation
        // In a real implementation, you would:
        // 1. Allocate descriptors in the virtqueue
        // 2. Set up the request structure
        // 3. Submit to the queue
        // 4. Wait for completion
        
        {
            let mut uart = UART.lock();
            let _ = writeln!(uart, "VirtIO block read requested (simplified implementation)");
        }

        // For now, just return success to test the framework
        Ok(())
    }

    pub fn write_block(&self, _block_num: u64, _buffer: &[u8]) -> Result<(), &'static str> {
        if !self.initialized {
            return Err("Device not initialized");
        }

        {
            let mut uart = UART.lock();
            let _ = writeln!(uart, "VirtIO block write requested (simplified implementation)");
        }

        // For now, just return success to test the framework
        Ok(())
    }
}

// Global block device instance
pub static VIRTIO_BLK: Mutex<VirtIOBlockDevice> = Mutex::new(VirtIOBlockDevice::new(0x10008000));

pub fn init_block_device() -> Result<(), &'static str> {
    let mut device = VIRTIO_BLK.lock();
    device.init()
}

pub fn probe_virtio_devices() {
    let mut uart = UART.lock();
    let _ = writeln!(uart, "\nProbing for VirtIO devices...");
    drop(uart);

    // Standard VirtIO MMIO addresses for QEMU virt machine
    let virtio_addrs = [
        0x10001000, 0x10002000, 0x10003000, 0x10004000,
        0x10005000, 0x10006000, 0x10007000, 0x10008000,
    ];

    for &addr in &virtio_addrs {
        unsafe {
            let magic_ptr = addr as *const u32;
            let magic = magic_ptr.read_volatile();
            
            if magic == 0x74726976 {
                let device_id_ptr = (addr + VIRTIO_MMIO_DEVICE_ID) as *const u32;
                let device_id = device_id_ptr.read_volatile();
                
                {
                    let mut uart = UART.lock();
                    let _ = writeln!(uart, "VirtIO device at 0x{:x}, ID: {}", addr, device_id);
                    
                    match device_id {
                        VIRTIO_DEVICE_ID_BLOCK => {
                            let _ = writeln!(uart, "  - Block device found!");
                        },
                        _ => {
                            let _ = writeln!(uart, "  - Unknown device type");
                        }
                    }
                }
                
                if device_id == VIRTIO_DEVICE_ID_BLOCK {
                    // Initialize the block device
                    let mut device = VirtIOBlockDevice::new(addr);
                    if let Err(e) = device.init() {
                        let mut uart = UART.lock();
                        let _ = writeln!(uart, "  - Failed to initialize: {}", e);
                    }
                }
            }
        }
    }
} 