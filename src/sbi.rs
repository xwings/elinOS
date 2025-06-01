use core::arch::asm;

// SBI function IDs
const SBI_GET_MEMORY_REGIONS: usize = 0x100;

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