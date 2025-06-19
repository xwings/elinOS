// elinOS System Call Module

use crate::UART;
use core::fmt::Write;

// Common error codes (Linux-compatible)
pub const EPERM: isize = 1;      // Operation not permitted
pub const ENOENT: isize = 2;     // No such file or directory
pub const ESRCH: isize = 3;      // No such process
pub const EINTR: isize = 4;      // Interrupted system call
pub const EIO: isize = 5;        // I/O error
pub const ENXIO: isize = 6;      // No such device or address
pub const E2BIG: isize = 7;      // Argument list too long
pub const ENOEXEC: isize = 8;    // Exec format error
pub const EBADF: isize = 9;      // Bad file number
pub const ECHILD: isize = 10;    // No child processes
pub const EAGAIN: isize = 11;    // Try again
pub const ENOMEM: isize = 12;    // Out of memory
pub const EACCES: isize = 13;    // Permission denied
pub const EFAULT: isize = 14;    // Bad address
pub const ENOTBLK: isize = 15;   // Block device required
pub const EBUSY: isize = 16;     // Device or resource busy
pub const EEXIST: isize = 17;    // File exists
pub const EXDEV: isize = 18;     // Cross-device link
pub const ENODEV: isize = 19;    // No such device
pub const ENOTDIR: isize = 20;   // Not a directory
pub const EISDIR: isize = 21;    // Is a directory
pub const EINVAL: isize = 22;    // Invalid argument
pub const ENFILE: isize = 23;    // File table overflow
pub const EMFILE: isize = 24;    // Too many open files
pub const ENOTTY: isize = 25;    // Not a typewriter
pub const ETXTBSY: isize = 26;   // Text file busy
pub const EFBIG: isize = 27;     // File too large
pub const ENOSPC: isize = 28;    // No space left on device
pub const ESPIPE: isize = 29;    // Illegal seek
pub const EROFS: isize = 30;     // Read-only file system
pub const EMLINK: isize = 31;    // Too many links
pub const EPIPE: isize = 32;     // Broken pipe
pub const EDOM: isize = 33;      // Math argument out of domain of func
pub const ERANGE: isize = 34;    // Math result not representable
pub const ENOSYS: isize = 38;    // Function not implemented

// Import all syscall category modules
pub mod file;
pub mod directory;
pub mod memory;
pub mod process;
pub mod device;
pub mod network;
pub mod time;
pub mod sysinfo;
pub mod elinos;

// Re-export all syscall constants for easy access
pub use file::*;
pub use directory::*;
pub use memory::*;
pub use process::*;
pub use device::*;
pub use network::*;
pub use time::*;
pub use sysinfo::*;
pub use elinos::*;

// System call results
#[derive(Debug)]
pub enum SysCallResult {
    Success(isize),
    Error(isize),
}

impl SysCallResult {
    pub fn as_isize(&self) -> isize {
        match self {
            SysCallResult::Success(val) => *val,
            SysCallResult::Error(code) => -*code,
        }
    }
    
    pub fn is_error(&self) -> bool {
        matches!(self, SysCallResult::Error(_))
    }
}

// Standardized syscall arguments structure
#[derive(Debug, Clone, Copy)]
pub struct SyscallArgs {
    pub syscall_number: usize,
    pub arg0: usize,
    pub arg1: usize,
    pub arg2: usize,
    pub arg3: usize,
    pub arg4: usize,
    pub arg5: usize,
}

impl SyscallArgs {
    pub fn new(syscall_num: usize, arg0: usize, arg1: usize, arg2: usize, arg3: usize) -> Self {
        Self {
            syscall_number: syscall_num,
            arg0,
            arg1,
            arg2,
            arg3,
            arg4: 0,
            arg5: 0,
        }
    }

    pub fn with_all_args(
        syscall_num: usize,
        arg0: usize,
        arg1: usize,
        arg2: usize,
        arg3: usize,
        arg4: usize,
        arg5: usize,
    ) -> Self {
        Self {
            syscall_number: syscall_num,
            arg0,
            arg1,
            arg2,
            arg3,
            arg4,
            arg5,
        }
    }

    // Convenience methods for type casting
    pub fn arg0_as_i32(&self) -> i32 { self.arg0 as i32 }
    pub fn arg1_as_i32(&self) -> i32 { self.arg1 as i32 }
    pub fn arg2_as_i32(&self) -> i32 { self.arg2 as i32 }
    pub fn arg3_as_i32(&self) -> i32 { self.arg3 as i32 }
    
    pub fn arg0_as_ptr<T>(&self) -> *const T { self.arg0 as *const T }
    pub fn arg1_as_ptr<T>(&self) -> *const T { self.arg1 as *const T }
    pub fn arg2_as_ptr<T>(&self) -> *const T { self.arg2 as *const T }
    
    pub fn arg0_as_mut_ptr<T>(&self) -> *mut T { self.arg0 as *mut T }
    pub fn arg1_as_mut_ptr<T>(&self) -> *mut T { self.arg1 as *mut T }
    pub fn arg2_as_mut_ptr<T>(&self) -> *mut T { self.arg2 as *mut T }
}

// Standardized syscall handler trait
pub trait SyscallHandler {
    fn handle_syscall(&self, args: &SyscallArgs) -> SysCallResult;
    fn get_category_name(&self) -> &'static str;
    fn get_syscall_range(&self) -> (usize, usize);
}

// File descriptor constants
pub const STDOUT_FD: i32 = 1;
pub const STDERR_FD: i32 = 2;

// System call categorization for debugging and documentation
pub fn get_syscall_category(syscall_num: usize) -> &'static str {
    match syscall_num {
        // File I/O operations (Linux numbers)
        35 | 45..=47 | 56..=64 | 78..=83 => "File I/O Operations",
        
        // Directory operations (Linux numbers)  
        34 | 49..=55 => "Directory Operations",
        
        // Device and I/O management (Linux numbers)
        23..=33 | 59 => "Device and I/O Management",
        
        // Process management (Linux numbers) - non-overlapping ranges
        93..=100 | 129..=178 | 220..=221 => "Process Management",
        
        // Time operations (Linux numbers) - non-overlapping ranges  
        101..=115 => "Time and Timer Operations",
        
        // System information (Linux numbers) - non-overlapping ranges
        160..=168 | 169..=171 | 179 => "System Information",
        
        // Network operations (Linux numbers)
        198..=213 => "Network Operations", 
        
        // Memory management (Linux numbers)
        214..=239 => "Memory Management",
        
        // elinOS-specific operations
        900..=999 => "elinOS-Specific Operations",
        
        _ => "Unknown Category",
    }
}

/// Unified system call handler - dispatches all syscalls to appropriate modules
pub fn handle_syscall(args: SyscallArgs) -> SysCallResult {
    let syscall_num = args.syscall_number;
    
    match syscall_num {
        // === DEVICE AND I/O MANAGEMENT (Linux numbers) ===
        23..=33 |      // dup, dup3, fcntl, ioctl, etc.
        59 |           // pipe2
        950            // elinOS: getdevices
        => device::handle_device_syscall(&args),
        
        // === DIRECTORY OPERATIONS (Linux numbers) ===
        34 |           // mkdirat
        49..=55        // chdir, fchdir, chroot, fchmod, fchmodat, fchownat, fchown
        => directory::handle_directory_syscall(&args),
        
        // === FILE I/O OPERATIONS (Linux numbers) ===
        35 |           // unlinkat
        45..=47 |      // truncate, ftruncate, fallocate  
        56..=64 |      // openat, close, read, write, readv, writev, etc.
        78..=83        // readlinkat, newfstatat, fstat, sync, fsync, fdatasync
        => file::handle_file_syscall(&args),
        
        // === PROCESS MANAGEMENT (Linux numbers - first range) ===
        93..=100       // exit, exit_group, waitid, futex, etc.
        => process::handle_process_syscall(syscall_num, &args),
        
        // === TIME AND TIMER OPERATIONS (Linux numbers) ===
        101..=115      // nanosleep, getitimer, setitimer, timer_*, clock_*
        => time::handle_time_syscall(&args),
        
        // === PROCESS MANAGEMENT (Linux numbers - second range) ===
        129..=178      // kill, getpid, getppid, etc.
        => process::handle_process_syscall(syscall_num, &args),
        
        // === NETWORK OPERATIONS (Linux numbers) ===
        198..=213      // socket, socketpair, bind, listen, accept, connect, etc.
        => network::handle_network_syscall(&args),
        
        // === MEMORY MANAGEMENT (Linux numbers) ===
        214..=239 |    // brk, munmap, mremap, mmap, mprotect, msync, mlock, etc.
        960            // elinOS: getmeminfo
        => memory::handle_memory_syscall(&args),
        
        // === PROCESS MANAGEMENT (Linux numbers - third range) ===
        220..=221      // clone, execve
        => process::handle_process_syscall(syscall_num, &args),
        
        // === SYSTEM INFORMATION (Linux numbers) ===
        970..=979      // elinOS: getsysinfo, getversion, etc.
        => sysinfo::handle_sysinfo_syscall(&args),
        
        // === elinOS-SPECIFIC OPERATIONS ===
        900..=949 |    // elinOS: load_elf, exec_elf, elf_info, etc.
        980..=999      // elinOS: misc operations
        => elinos::handle_elinos_syscall(&args),
        
        // === UNKNOWN SYSCALLS ===
        _ => {
            crate::console_println!("❓ Unknown syscall: {} (category: {})", 
                syscall_num, get_syscall_category(syscall_num));
            SysCallResult::Error(-1)
        }
    }
}

/// Legacy syscall handler for backward compatibility
pub fn syscall_handler(
    syscall_num: usize,
    arg0: usize,
    arg1: usize,
    arg2: usize,
    arg3: usize,
) -> SysCallResult {
    let args = SyscallArgs::new(syscall_num, arg0, arg1, arg2, arg3);
    handle_syscall(args)
}

// Utility function for user programs to print using SYS_WRITE
pub fn sys_print(s: &str) -> Result<(), &'static str> {
    let result = handle_syscall(SyscallArgs::new(SYS_WRITE, 1, s.as_ptr() as usize, s.len(), 0));
    match result {
        SysCallResult::Success(_) => Ok(()),
        SysCallResult::Error(_) => Err("Syscall failed"),
    }
}

// Utility function for memory info using SYS_GETMEMINFO  
pub fn sys_memory_info() -> Result<(), &'static str> {
    let result = handle_syscall(SyscallArgs::new(memory::SYS_GETMEMINFO, 0, 0, 0, 0));
    match result {
        SysCallResult::Success(_) => Ok(()),
        SysCallResult::Error(_) => Err("Syscall failed"),
    }
}

// Utility function for device info using SYS_GETDEVICES
pub fn sys_device_info() -> Result<(), &'static str> {
    let result = handle_syscall(SyscallArgs::new(device::SYS_GETDEVICES, 0, 0, 0, 0));
    match result {
        SysCallResult::Success(_) => Ok(()),
        SysCallResult::Error(_) => Err("Syscall failed"),
    }
}

// Debug function to show syscall categories
pub fn sys_show_categories() -> Result<(), &'static str> {
    sys_print("System Call Categories (Linux Compatible Numbers):\n")?;
    sys_print("  File I/O Operations:\n")?;
    sys_print("    35: unlinkat, 45-47: truncate/ftruncate/fallocate\n")?;
    sys_print("    56-64: openat/close/read/write/readv/writev/sendfile/etc\n")?;
    sys_print("    78-83: readlinkat/newfstatat/fstat/sync/fsync/fdatasync\n")?;
    sys_print("  Directory Operations:\n")?;
    sys_print("    34: mkdirat, 49-55: chdir/fchdir/chroot/fchmod/etc\n")?;
    sys_print("  Memory Management:\n")?;
    sys_print("    214-239: brk/munmap/mremap/mmap/mprotect/mlock/etc\n")?;
    sys_print("    960: getmeminfo (elinOS-specific)\n")?;
    sys_print("  Process Management:\n")?;
    sys_print("    93-100: exit/waitid/futex/getpid/getppid/kill/etc\n")?;
    sys_print("    129-178: kill/getpid/getppid/etc\n")?;
    sys_print("    220-221: clone/execve\n")?;
    sys_print("  Device and I/O Management:\n")?;
    sys_print("    23-33: dup/dup3/fcntl/ioctl/flock/mknodat/etc\n")?;
    sys_print("    59: pipe2, 950: getdevices (elinOS-specific)\n")?;
    sys_print("  Network Operations:\n")?;
    sys_print("    198-213: socket/bind/listen/accept/connect/etc\n")?;
    sys_print("  Time and Timer Operations:\n")?;
    sys_print("    101-115: nanosleep/getitimer/timer_*/clock_*\n")?;
    sys_print("  System Information:\n")?;
    sys_print("    160-168: uname/sethostname/getrlimit/setrlimit/etc\n")?;
    sys_print("    169-171: gettimeofday/settimeofday/adjtimex\n")?;
    sys_print("    179: sysinfo\n")?;
    sys_print("  elinOS-Specific Operations:\n")?;
    sys_print("    900-999: debug/version/shutdown/load_elf/exec_elf/etc\n")?;
    Ok(())
}

// Utility functions for printing numbers and formatting
pub fn sys_print_num(num: u64) -> Result<(), &'static str> {
    // Convert number to string
    let mut buffer = [0u8; 20];
    let mut temp = num;
    let mut pos = 0;
    
    if temp == 0 {
        buffer[0] = b'0';
        pos = 1;
    } else {
        while temp > 0 {
            buffer[19 - pos] = b'0' + (temp % 10) as u8;
            temp /= 10;
            pos += 1;
        }
    }
    
    let num_str = core::str::from_utf8(&buffer[20 - pos..]).unwrap_or("?");
    sys_print(num_str)
}

pub fn sys_print_hex(num: u32, digits: usize) -> Result<(), &'static str> {
    // Convert number to hex string with specified number of digits
    let mut buffer = [0u8; 8];
    let mut temp = num;
    
    for i in 0..digits.min(8) {
        let digit = (temp % 16) as u8;
        buffer[digits - 1 - i] = if digit < 10 { b'0' + digit } else { b'a' + digit - 10 };
        temp /= 16;
    }
    
    let hex_str = core::str::from_utf8(&buffer[..digits]).unwrap_or("?");
    sys_print(hex_str)
}

pub fn sys_print_char(c: char) -> Result<(), &'static str> {
    let mut buffer = [0u8; 4];
    let s = c.encode_utf8(&mut buffer);
    sys_print(s)
} 