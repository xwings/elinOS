use core::arch::asm;

// SBI function IDs
const SBI_GET_MEMORY_REGIONS: usize = 0x100;

// SBI System Reset Extension (EID 0x53525354 "SRST")
const SBI_SYSTEM_RESET_EID: usize = 0x53525354;

// Reset function ID
const SBI_SYSTEM_RESET_FID: usize = 0x0;

// Reset types
const SBI_RESET_TYPE_SHUTDOWN: usize = 0x0;
const SBI_RESET_TYPE_COLD_REBOOT: usize = 0x1;
const SBI_RESET_TYPE_WARM_REBOOT: usize = 0x2;

// Reset reasons  
const SBI_RESET_REASON_NO_REASON: usize = 0x0;
const SBI_RESET_REASON_SYSTEM_FAILURE: usize = 0x1;

// SBI return structure
#[repr(C)]
#[derive(Copy, Clone)]
pub struct SbiMemoryRegion {
    pub start: usize,
    pub size: usize,
    pub flags: usize,
}

// SBI return structure
#[repr(C)]
pub struct SbiMemoryRegions {
    pub count: usize,
    pub regions: [SbiMemoryRegion; 8],
}

// Make SBI call
#[inline(always)]
fn sbi_call(which: usize, arg0: usize, arg1: usize, arg2: usize) -> usize {
    let mut ret;
    unsafe {
        asm!(
            "ecall",
            inlateout("a0") arg0 => ret,
            in("a1") arg1,
            in("a2") arg2,
            in("a7") which,
            options(nostack)
        );
    }
    ret
}

// Get memory regions from OpenSBI
pub fn get_memory_regions() -> SbiMemoryRegions {
    let mut regions = SbiMemoryRegions {
        count: 0,
        regions: [SbiMemoryRegion { start: 0, size: 0, flags: 0 }; 8],
    };
    
    // Get number of regions
    let count = sbi_call(SBI_GET_MEMORY_REGIONS, 0, 0, 0);
    if count > 0 {
        regions.count = if count < 8 { count } else { 8 };
        
        // Get each region's information
        for i in 0..regions.count {
            let start = sbi_call(SBI_GET_MEMORY_REGIONS, 1, i, 0);
            let size = sbi_call(SBI_GET_MEMORY_REGIONS, 2, i, 0);
            let flags = sbi_call(SBI_GET_MEMORY_REGIONS, 3, i, 0);
            
            regions.regions[i] = SbiMemoryRegion {
                start,
                size,
                flags,
            };
        }
    }
    
    regions
}

// Shutdown the system using SBI
pub fn shutdown() -> ! {
    unsafe {
        asm!(
            "ecall",
            in("a0") SBI_RESET_TYPE_SHUTDOWN,
            in("a1") SBI_RESET_REASON_NO_REASON,
            in("a6") SBI_SYSTEM_RESET_FID,
            in("a7") SBI_SYSTEM_RESET_EID,
            options(noreturn)
        );
    }
}

// Reboot the system using SBI
pub fn reboot() -> ! {
    unsafe {
        asm!(
            "ecall",
            in("a0") SBI_RESET_TYPE_COLD_REBOOT,
            in("a1") SBI_RESET_REASON_NO_REASON,
            in("a6") SBI_SYSTEM_RESET_FID,
            in("a7") SBI_SYSTEM_RESET_EID,
            options(noreturn)
        );
    }
} 