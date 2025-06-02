// Memory Management System Calls (71-120)
// Handles memory operations like mmap, munmap, memory info, etc.

use crate::memory;
use crate::UART;
use core::fmt::Write;
use super::{SysCallResult, SyscallArgs};

// === MEMORY MANAGEMENT SYSTEM CALL CONSTANTS (71-120) ===
pub const SYS_MMAP: usize = 71;
pub const SYS_MUNMAP: usize = 72;
pub const SYS_MPROTECT: usize = 73;
pub const SYS_MADVISE: usize = 74;
pub const SYS_MLOCK: usize = 75;
pub const SYS_MUNLOCK: usize = 76;
pub const SYS_BRK: usize = 77;
pub const SYS_SBRK: usize = 78;
pub const SYS_GETMEMINFO: usize = 100;  // ElinOS-specific memory info
// Reserved for future memory management: 79-99, 101-120

// Memory protection flags
pub const PROT_READ: usize = 1;
pub const PROT_WRITE: usize = 2;
pub const PROT_EXEC: usize = 4;
pub const PROT_NONE: usize = 0;

// Memory mapping flags
pub const MAP_SHARED: usize = 1;
pub const MAP_PRIVATE: usize = 2;
pub const MAP_ANONYMOUS: usize = 32;
pub const MAP_FIXED: usize = 16;

// Standardized memory management syscall handler
pub fn handle_memory_syscall(args: &SyscallArgs) -> SysCallResult {
    match args.syscall_num {
        SYS_MMAP => sys_mmap(args.arg0, args.arg1, args.arg2),
        SYS_MUNMAP => sys_munmap(args.arg0, args.arg1),
        SYS_MPROTECT => sys_mprotect(args.arg0, args.arg1, args.arg2),
        SYS_MADVISE => sys_madvise(args.arg0, args.arg1, args.arg2),
        SYS_MLOCK => sys_mlock(args.arg0, args.arg1),
        SYS_MUNLOCK => sys_munlock(args.arg0, args.arg1),
        SYS_BRK => sys_brk(args.arg0),
        SYS_SBRK => sys_sbrk(args.arg0 as isize),
        SYS_GETMEMINFO => sys_getmeminfo(),
        _ => SysCallResult::Error("Unknown memory management system call"),
    }
}

// === SYSTEM CALL IMPLEMENTATIONS ===

fn sys_mmap(_addr: usize, _length: usize, _prot: usize) -> SysCallResult {
    // TODO: Implement memory mapping
    SysCallResult::Error("mmap not implemented")
}

fn sys_munmap(_addr: usize, _length: usize) -> SysCallResult {
    // TODO: Implement memory unmapping
    SysCallResult::Error("munmap not implemented")
}

fn sys_mprotect(_addr: usize, _len: usize, _prot: usize) -> SysCallResult {
    // TODO: Implement memory protection change
    SysCallResult::Error("mprotect not implemented")
}

fn sys_madvise(_addr: usize, _length: usize, _advice: usize) -> SysCallResult {
    // TODO: Implement memory advice
    SysCallResult::Error("madvise not implemented")
}

fn sys_mlock(_addr: usize, _len: usize) -> SysCallResult {
    // TODO: Implement memory locking
    SysCallResult::Error("mlock not implemented")
}

fn sys_munlock(_addr: usize, _len: usize) -> SysCallResult {
    // TODO: Implement memory unlocking
    SysCallResult::Error("munlock not implemented")
}

fn sys_brk(_addr: usize) -> SysCallResult {
    // TODO: Implement program break change
    SysCallResult::Error("brk not implemented")
}

fn sys_sbrk(_increment: isize) -> SysCallResult {
    // TODO: Implement program break increment
    SysCallResult::Error("sbrk not implemented")
}

fn sys_getmeminfo() -> SysCallResult {
    let mem_mgr = memory::MEMORY_MANAGER.lock();
    let mut uart = UART.lock();
    
    let _ = writeln!(uart, "Memory regions:");
    for (i, region) in mem_mgr.get_memory_info().iter().enumerate() {
        let _ = writeln!(uart, "  Region {}: 0x{:x} - 0x{:x} ({} MB) {}",
            i,
            region.start,
            region.start + region.size,
            region.size / (1024 * 1024),
            if region.is_ram { "RAM" } else { "MMIO" }
        );
    }
    
    SysCallResult::Success(0)
} 