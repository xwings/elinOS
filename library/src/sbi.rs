// SBI (Supervisor Binary Interface) calls for RISC-V
// This provides the interface between the kernel and the SBI firmware

use core::arch::asm;
use crate::console_println;

// SBI function IDs
const SBI_CONSOLE_PUTCHAR: usize = 0x1;
const SBI_CONSOLE_GETCHAR: usize = 0x2;
const SBI_SHUTDOWN: usize = 0x8;

// SBI extensions
const SBI_EXT_BASE: usize = 0x10;
const SBI_EXT_TIMER: usize = 0x54494D45;
const SBI_EXT_IPI: usize = 0x735049;
// Removed unused SBI extensions: RFENCE, HSM
const SBI_EXT_SRST: usize = 0x53525354;

// SBI reset types
const SBI_SRST_RESET_TYPE_SHUTDOWN: u32 = 0;
const SBI_SRST_RESET_TYPE_COLD_REBOOT: u32 = 1;
// Removed unused reset type: WARM_REBOOT

// SBI reset reasons
const SBI_SRST_RESET_REASON_NONE: u32 = 0;

// SBI return values
#[derive(Debug, Clone, Copy)]
pub struct SbiRet {
    pub error: isize,
    pub value: isize,
}

// Memory region information
#[derive(Debug, Clone, Copy)]
pub struct SbiMemoryRegion {
    pub start: usize,
    pub size: usize,
    pub flags: usize,  // 1 = RAM, 0 = MMIO
}

pub struct SbiMemoryInfo {
    pub regions: [SbiMemoryRegion; 8],
    pub count: usize,
}

// Generic SBI call
fn sbi_call(eid: usize, fid: usize, arg0: usize, arg1: usize, arg2: usize) -> SbiRet {
    let (error, value);
    unsafe {
        asm!(
            "ecall",
            inlateout("a0") arg0 => error,
            inlateout("a1") arg1 => value,
            in("a2") arg2,
            in("a6") fid,
            in("a7") eid,
        );
    }
    SbiRet { error, value }
}

// Console output
pub fn console_putchar(ch: usize) {
    sbi_call(SBI_EXT_BASE, SBI_CONSOLE_PUTCHAR, ch, 0, 0);
}

// Console input (if available)
pub fn console_getchar() -> Option<usize> {
    let ret = sbi_call(SBI_EXT_BASE, SBI_CONSOLE_GETCHAR, 0, 0, 0);
    if ret.error == 0 {
        Some(ret.value as usize)
    } else {
        None
    }
}

// System shutdown
pub fn system_shutdown() -> ! {
    // Use console print fallback since console_println might not be available here
    console_println!("[i] Initiating system shutdown via SBI...\n");
    
    // Try newer SBI system reset extension first
    let ret = sbi_call(SBI_EXT_SRST, 0, SBI_SRST_RESET_TYPE_SHUTDOWN as usize, SBI_SRST_RESET_REASON_NONE as usize, 0);
    
    // If that fails, try legacy shutdown
    if ret.error != 0 {
        sbi_call(SBI_EXT_BASE, SBI_SHUTDOWN, 0, 0, 0);
    }
    
    // If SBI shutdown fails, halt manually
    console_println!("SBI shutdown failed, halting manually\n");
    loop {
        unsafe {
            asm!("wfi");
        }
    }
}

// System reset/reboot
pub fn system_reset() -> ! {
    // Use console print fallback since console_println might not be available here
    console_println!("[i] Initiating system reboot via SBI...\n");
    
    // Try SBI system reset extension
    let ret = sbi_call(SBI_EXT_SRST, 0, SBI_SRST_RESET_TYPE_COLD_REBOOT as usize, SBI_SRST_RESET_REASON_NONE as usize, 0);
    
    console_println!("[x] SBI reset failed (error: {}), halting\n", ret.error);
    loop {
        unsafe {
            asm!("wfi");
        }
    }
}

// Get memory information
pub fn get_memory_info() -> (usize, usize) {
    // Try to get memory info from device tree first
    if let Some((base, size)) = get_memory_from_device_tree() {
        return (base, size);
    }
    
    // Fallback to QEMU virt machine standard memory layout
    // RAM: 0x80000000 - varies (usually 128MB)
    let base = 0x80000000;
    let size = 128 * 1024 * 1024; // 128MB default
    
    (base, size)
}

// Try to get memory information from device tree
fn get_memory_from_device_tree() -> Option<(usize, usize)> {
    // OpenSBI typically passes device tree blob address in register a1
    // This is a placeholder - in real implementation, we'd need to get the DTB address
    // from the boot protocol (usually passed by OpenSBI)
    
    // For now, return None to use fallback
    // TODO: Get actual DTB address from boot protocol
    None
}

// Get memory regions (for compatibility with memory detection)
pub fn get_memory_regions() -> SbiMemoryInfo {
    // Try to get memory regions from device tree first
    if let Some(info) = get_memory_regions_from_device_tree() {
        return info;
    }
    
    // Fallback to hardcoded QEMU regions
    let mut info = SbiMemoryInfo {
        regions: [SbiMemoryRegion { start: 0, size: 0, flags: 0 }; 8],
        count: 0,
    };
    
    // Add main RAM region
    info.regions[0] = SbiMemoryRegion {
        start: 0x80000000,
        size: 128 * 1024 * 1024, // 128MB
        flags: 1, // RAM
    };
    info.count = 1;
    
    // Add MMIO regions
    info.regions[1] = SbiMemoryRegion {
        start: 0x10000000,
        size: 0x1000, // UART
        flags: 0, // MMIO
    };
    info.count = 2;
    
    info.regions[2] = SbiMemoryRegion {
        start: 0x02000000,
        size: 0x10000, // CLINT
        flags: 0, // MMIO
    };
    info.count = 3;
    
    info.regions[3] = SbiMemoryRegion {
        start: 0x0c000000,
        size: 0x400000, // PLIC
        flags: 0, // MMIO
    };
    info.count = 4;
    
    info
}

// Try to get memory regions from device tree
fn get_memory_regions_from_device_tree() -> Option<SbiMemoryInfo> {
    // This would parse the device tree to extract memory regions
    // For now, return None to use fallback
    // TODO: Implement device tree parsing for memory regions
    None
}

// Set timer
pub fn set_timer(stime: u64) {
    sbi_call(SBI_EXT_TIMER, 0, stime as usize, (stime >> 32) as usize, 0);
}

// Send IPI
pub fn send_ipi(hart_mask: usize) {
    sbi_call(SBI_EXT_IPI, 0, hart_mask, 0, 0);
}

// Get SBI implementation ID
pub fn get_sbi_impl_id() -> usize {
    let ret = sbi_call(SBI_EXT_BASE, 1, 0, 0, 0);
    ret.value as usize
}

// Get SBI implementation version
pub fn get_sbi_impl_version() -> usize {
    let ret = sbi_call(SBI_EXT_BASE, 2, 0, 0, 0);
    ret.value as usize
}

// Check if extension is available
pub fn probe_extension(extension_id: usize) -> bool {
    let ret = sbi_call(SBI_EXT_BASE, 3, extension_id, 0, 0);
    ret.value != 0
} 