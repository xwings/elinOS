// Memory Management System Calls - Linux Compatible Numbers
// Following Linux ARM64/RISC-V syscall numbers for compatibility

use crate::{memory, console_println};
use super::{SysCallResult, SyscallArgs};

// === LINUX COMPATIBLE MEMORY MANAGEMENT SYSTEM CALL CONSTANTS ===
pub const SYS_BRK: usize = 214;           // Linux: brk
pub const SYS_MUNMAP: usize = 215;        // Linux: munmap
pub const SYS_MREMAP: usize = 216;        // Linux: mremap
pub const SYS_ADD_KEY: usize = 217;       // Linux: add_key
pub const SYS_REQUEST_KEY: usize = 218;   // Linux: request_key
pub const SYS_KEYCTL: usize = 219;        // Linux: keyctl
pub const SYS_CLONE: usize = 220;         // Linux: clone (defined in process.rs)
pub const SYS_EXECVE: usize = 221;        // Linux: execve (defined in process.rs)
pub const SYS_MMAP: usize = 222;          // Linux: mmap
pub const SYS_FADVISE64: usize = 223;     // Linux: fadvise64
pub const SYS_SWAPON: usize = 224;        // Linux: swapon
pub const SYS_SWAPOFF: usize = 225;       // Linux: swapoff
pub const SYS_MPROTECT: usize = 226;      // Linux: mprotect
pub const SYS_MSYNC: usize = 227;         // Linux: msync
pub const SYS_MLOCK: usize = 228;         // Linux: mlock
pub const SYS_MUNLOCK: usize = 229;       // Linux: munlock
pub const SYS_MLOCKALL: usize = 230;      // Linux: mlockall
pub const SYS_MUNLOCKALL: usize = 231;    // Linux: munlockall
pub const SYS_MINCORE: usize = 232;       // Linux: mincore
pub const SYS_MADVISE: usize = 233;       // Linux: madvise
pub const SYS_REMAP_FILE_PAGES: usize = 234; // Linux: remap_file_pages
pub const SYS_MBIND: usize = 235;         // Linux: mbind
pub const SYS_GET_MEMPOLICY: usize = 236; // Linux: get_mempolicy
pub const SYS_SET_MEMPOLICY: usize = 237; // Linux: set_mempolicy
pub const SYS_MIGRATE_PAGES: usize = 238; // Linux: migrate_pages
pub const SYS_MOVE_PAGES: usize = 239;    // Linux: move_pages

// Legacy syscall aliases for backwards compatibility
pub const SYS_SBRK: usize = SYS_BRK;      // Map sbrk to brk

// elinOS-specific memory syscalls (keeping high numbers to avoid conflicts)
pub const SYS_GETMEMINFO: usize = 960;   // elinOS: get memory info
pub const SYS_ALLOC_TEST: usize = 961;   // elinOS: test allocator
pub const SYS_BUDDY_STATS: usize = 962;  // elinOS: buddy allocator stats

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

// Current program break (for brk implementation)
static mut PROGRAM_BREAK: usize = 0;

// Linux compatible memory management syscall handler
pub fn handle_memory_syscall(args: &SyscallArgs) -> SysCallResult {
    match args.syscall_number {
        SYS_MMAP => sys_mmap(args.arg0, args.arg1, args.arg2, args.arg3, args.arg4, args.arg5),
        SYS_MUNMAP => sys_munmap(args.arg0, args.arg1),
        SYS_MPROTECT => sys_mprotect(args.arg0, args.arg1, args.arg2),
        SYS_MADVISE => sys_madvise(args.arg0, args.arg1, args.arg2),
        SYS_MLOCK => sys_mlock(args.arg0, args.arg1),
        SYS_MUNLOCK => sys_munlock(args.arg0, args.arg1),
        SYS_MLOCKALL => sys_mlockall(args.arg0),
        SYS_MUNLOCKALL => sys_munlockall(),
        SYS_BRK => sys_brk(args.arg0),
        SYS_MREMAP => sys_mremap(args.arg0, args.arg1, args.arg2, args.arg3, args.arg4),
        SYS_MSYNC => sys_msync(args.arg0, args.arg1, args.arg2),
        SYS_MINCORE => sys_mincore(args.arg0, args.arg1, args.arg2),
        SYS_GETMEMINFO => sys_getmeminfo(),
        SYS_ALLOC_TEST => sys_alloc_test(args.arg0),
        SYS_BUDDY_STATS => sys_buddy_stats(),
        _ => SysCallResult::Error(crate::syscall::ENOSYS),
    }
}

// === SYSTEM CALL IMPLEMENTATIONS ===

fn sys_mmap(addr: usize, length: usize, prot: usize, flags: usize, _fd: usize, _offset: usize) -> SysCallResult {
    console_println!("mmap called: addr=0x{:x}, len={}, prot={}, flags={}", addr, length, prot, flags);
    
    // For anonymous mappings, use our buddy allocator
    if flags & MAP_ANONYMOUS != 0 {
        if let Ok(allocated_addr) = memory::allocate_memory(length, 8) {
            let addr = allocated_addr.as_ptr() as usize;
            console_println!("mmap allocated: 0x{:x}", addr);
            return SysCallResult::Success(addr as isize);
        } else {
            return SysCallResult::Error(crate::syscall::ENOMEM);
        }
    }
    
            SysCallResult::Error(crate::syscall::ENOSYS)
}

fn sys_munmap(addr: usize, length: usize) -> SysCallResult {
    console_println!("munmap called: addr=0x{:x}, len={}", addr, length);
    
    // Use our deallocator
    if let Some(ptr) = core::ptr::NonNull::new(addr as *mut u8) {
        memory::deallocate_memory(ptr, length);
    }
    
    SysCallResult::Success(0)
}

fn sys_mprotect(_addr: usize, _length: usize, _prot: usize) -> SysCallResult {
    // TODO: Implement memory protection changes
    SysCallResult::Success(0) // Pretend success for now
}

fn sys_madvise(_addr: usize, _length: usize, _advice: usize) -> SysCallResult {
    // TODO: Implement memory advice
    SysCallResult::Success(0) // Pretend success for now
}

fn sys_mlock(_addr: usize, _length: usize) -> SysCallResult {
    // TODO: Implement memory locking
    SysCallResult::Success(0) // Pretend success for now
}

fn sys_munlock(_addr: usize, _length: usize) -> SysCallResult {
    // TODO: Implement memory unlocking
    SysCallResult::Success(0) // Pretend success for now
}

fn sys_mlockall(_flags: usize) -> SysCallResult {
    // TODO: Implement lock all memory
    SysCallResult::Success(0) // Pretend success for now
}

fn sys_munlockall() -> SysCallResult {
    // TODO: Implement unlock all memory
    SysCallResult::Success(0) // Pretend success for now
}

fn sys_brk(addr: usize) -> SysCallResult {
    console_println!("brk called: addr=0x{:x}", addr);
    
    unsafe {
        if addr == 0 {
            // Query current break
            if PROGRAM_BREAK == 0 {
                // Initialize program break - allocate initial heap
                if let Ok(initial_heap) = memory::allocate_memory(64 * 1024, 8) { // 64KB initial heap
                    PROGRAM_BREAK = initial_heap.as_ptr() as usize;
                }
            }
            SysCallResult::Success(PROGRAM_BREAK as isize)
        } else {
            // Set new break
            // For simplicity, we'll just allocate more memory if needed
            if addr > PROGRAM_BREAK {
                let needed = addr - PROGRAM_BREAK;
                if memory::allocate_memory(needed, 8).is_ok() {
                    PROGRAM_BREAK = addr;
                    SysCallResult::Success(addr as isize)
                } else {
                    SysCallResult::Error(crate::syscall::ENOMEM)
                }
            } else {
                // Shrinking heap - for now just update the break
                PROGRAM_BREAK = addr;
                SysCallResult::Success(addr as isize)
            }
        }
    }
}

fn sys_mremap(_old_addr: usize, _old_size: usize, _new_size: usize, _flags: usize, _new_addr: usize) -> SysCallResult {
    // TODO: Implement memory remapping
    SysCallResult::Error(crate::syscall::ENOSYS)
}

fn sys_msync(_addr: usize, _length: usize, _flags: usize) -> SysCallResult {
    // TODO: Implement memory synchronization
    SysCallResult::Success(0) // Pretend success for now
}

fn sys_mincore(_addr: usize, _length: usize, _vec: usize) -> SysCallResult {
    // TODO: Implement memory residency check
    SysCallResult::Error(crate::syscall::ENOSYS)
}

fn sys_getmeminfo() -> SysCallResult {
    console_println!("=== Memory Management Information ===");
    
    // Show simplified memory manager stats
    let stats = memory::get_memory_stats();
    console_println!("Simplified Memory Manager:");
    console_println!("  Total Memory: {} MB", stats.detected_ram_size / (1024 * 1024));
    console_println!("  Allocated: {} bytes", stats.allocated_bytes);
    console_println!("  Allocations: {}", stats.allocation_count);
    console_println!("  Using heap-only allocation");
    
    // Show memory regions
    memory::display_memory_layout();
    
    unsafe {
        console_println!("Program Break: 0x{:x}", PROGRAM_BREAK);
    }
    
    SysCallResult::Success(0)
}

fn sys_alloc_test(size: usize) -> SysCallResult {
    console_println!("=== Allocation Test: {} bytes ===", size);
    
    // Test allocation
    let start_time = 0; // TODO: Add timing
    
    if let Ok(addr) = memory::allocate_memory(size, 8) {
        let addr = addr.as_ptr() as usize;
        console_println!("[o] Allocated {} bytes at 0x{:x}", size, addr);
        
        // Test writing to the memory
        unsafe {
            let ptr = addr as *mut u8;
            *ptr = 0xAA; // Write test pattern
            let read_val = *ptr;
            if read_val == 0xAA {
                console_println!("[o] Memory write/read test passed");
            } else {
                console_println!("[x] Memory write/read test failed: wrote 0xAA, read 0x{:x}", read_val);
            }
        }
        
        // Show updated stats
        let stats = memory::get_memory_stats();
        console_println!("[i] Updated stats: {} allocations, {} bytes allocated", 
                        stats.allocation_count, stats.allocated_bytes);
        
        SysCallResult::Success(addr as isize)
    } else {
        console_println!("[x] Allocation failed");
        SysCallResult::Error(crate::syscall::ENOMEM)
    }
}

fn sys_buddy_stats() -> SysCallResult {
    console_println!("=== Memory Allocator Statistics ===");
    
    let stats = memory::get_memory_stats();
    console_println!("Memory Allocator: Heap-only (simplified)");
    console_println!("Total Allocations: {}", stats.allocation_count);
    console_println!("Allocated Memory: {} bytes ({} KB)", stats.allocated_bytes, stats.allocated_bytes / 1024);
    console_println!("Note: Using simplified heap allocator only");
    
    SysCallResult::Success(0)
} 