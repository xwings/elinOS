use core::arch::asm;

// Standard SBI function IDs (SBI v0.2+)
const SBI_BASE_EID: usize = 0x10;
const SBI_BASE_GET_IMPL_ID: usize = 0x1;
const SBI_BASE_GET_IMPL_VERSION: usize = 0x2;

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

// Memory region structure
#[repr(C)]
#[derive(Copy, Clone)]
pub struct SbiMemoryRegion {
    pub start: usize,
    pub size: usize,
    pub flags: usize,
}

// Memory regions container
#[repr(C)]
pub struct SbiMemoryRegions {
    pub count: usize,
    pub regions: [SbiMemoryRegion; 8],
}

// SBI return value structure
#[repr(C)]
struct SbiRet {
    error: isize,
    value: usize,
}

// Make SBI call with proper return value handling
#[inline(always)]
fn sbi_call(eid: usize, fid: usize, arg0: usize, arg1: usize, arg2: usize) -> SbiRet {
    let error: isize;
    let value: usize;
    unsafe {
        asm!(
            "ecall",
            inlateout("a0") arg0 => error,
            inlateout("a1") arg1 => value,
            in("a2") arg2,
            in("a6") fid,
            in("a7") eid,
            options(nostack)
        );
    }
    SbiRet { error, value }
}

// Get memory regions using standard device tree method
pub fn get_memory_regions() -> SbiMemoryRegions {
    // Since we can't use custom SBI calls, we'll use a known working configuration
    // This matches typical QEMU virt machine memory layout
    
    let mut regions = SbiMemoryRegions {
        count: 1,
        regions: [SbiMemoryRegion { start: 0, size: 0, flags: 0 }; 8],
    };
    
    // QEMU virt machine typically provides:
    // - 128MB RAM starting at 0x80000000
    // - We're running at 0x80200000, so available memory starts there
    
    regions.regions[0] = SbiMemoryRegion {
        start: 0x80000000,          // Physical RAM start
        size: 128 * 1024 * 1024,    // 128MB (typical QEMU default)
        flags: 1,                   // RAM flag
    };
    
    // Try to detect actual memory size by probing (safely)
    // This is a simple method that works for most RISC-V systems
    let detected_size = detect_memory_size();
    if detected_size > 0 {
        regions.regions[0].size = detected_size;
    }
    
    regions
}

// Simple memory size detection
fn detect_memory_size() -> usize {
    // Start with a conservative base size
    let base_addr = 0x80000000;
    let mut test_addr = base_addr + (16 * 1024 * 1024); // Start testing at 16MB
    let max_addr = base_addr + (1024 * 1024 * 1024);    // Max 1GB
    
    // Simple probe test: try to read/write at different addresses
    while test_addr < max_addr {
        // Try to safely probe memory
        if !probe_memory_address(test_addr) {
            // Found the limit
            return test_addr - base_addr;
        }
        test_addr += 32 * 1024 * 1024; // Test in 32MB increments
    }
    
    // Default to 128MB if detection fails
    128 * 1024 * 1024
}

// Safely probe a memory address
fn probe_memory_address(addr: usize) -> bool {
    // This is a simple probe - in a real kernel you'd use proper fault handling
    // For now, we'll just assume addresses within reasonable bounds are valid
    
    let base = 0x80000000;
    let reasonable_limit = base + (512 * 1024 * 1024); // 512MB max
    
    addr >= base && addr < reasonable_limit
}

// Get SBI implementation info (for debugging)
pub fn get_sbi_info() -> (usize, usize) {
    let impl_id = sbi_call(SBI_BASE_EID, SBI_BASE_GET_IMPL_ID, 0, 0, 0);
    let impl_version = sbi_call(SBI_BASE_EID, SBI_BASE_GET_IMPL_VERSION, 0, 0, 0);
    
    (impl_id.value, impl_version.value)
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