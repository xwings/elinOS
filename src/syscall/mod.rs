// elinOS System Call Module

use crate::UART;
use core::fmt::Write;

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
    Error(&'static str),
}

impl SysCallResult {
    pub fn as_isize(&self) -> isize {
        match self {
            SysCallResult::Success(val) => *val,
            SysCallResult::Error(_) => -1,
        }
    }
    
    pub fn is_error(&self) -> bool {
        matches!(self, SysCallResult::Error(_))
    }
}

// Standardized syscall arguments structure
#[derive(Debug, Clone, Copy)]
pub struct SyscallArgs {
    pub syscall_num: usize,
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
            syscall_num,
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
            syscall_num,
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
        1..=50 => "File I/O Operations",
        51..=70 => "Directory Operations", 
        71..=120 => "Memory Management",
        121..=170 => "Process Management",
        171..=220 => "Device and I/O Management",
        221..=270 => "Network Operations",
        271..=300 => "Time and Timer Operations",
        301..=350 => "System Information",
        900..=999 => "elinOS-Specific Operations",
        _ => "Unknown Category",
    }
}

// Main system call handler with standardized dispatch
pub fn syscall_handler(
    syscall_num: usize,
    arg0: usize,
    arg1: usize,
    arg2: usize,
    arg3: usize,
) -> SysCallResult {
    let args = SyscallArgs::new(syscall_num, arg0, arg1, arg2, arg3);
    
    match syscall_num {
        // === FILE I/O OPERATIONS (1-50) ===
        1..=50 => file::handle_file_syscall(&args),
        
        // === DIRECTORY OPERATIONS (51-70) ===
        51..=70 => directory::handle_directory_syscall(&args),
        
        // === MEMORY MANAGEMENT (71-120) ===
        71..=120 => memory::handle_memory_syscall(&args),
        
        // === PROCESS MANAGEMENT (121-170) ===
        121..=170 => process::handle_process_syscall(&args),
        
        // === DEVICE AND I/O MANAGEMENT (171-220) ===
        171..=220 => device::handle_device_syscall(&args),
        
        // === NETWORK OPERATIONS (221-270) ===
        221..=270 => network::handle_network_syscall(&args),
        
        // === TIME AND TIMER OPERATIONS (271-300) ===
        271..=300 => time::handle_time_syscall(&args),
        
        // === SYSTEM INFORMATION (301-350) ===
        301..=350 => sysinfo::handle_sysinfo_syscall(&args),
        
        // === ELINOS-SPECIFIC (900-999) ===
        900..=999 => elinos::handle_elinos_syscall(&args),
        
        _ => SysCallResult::Error("Unknown system call"),
    }
}

// Utility function for user programs to print using SYS_WRITE
pub fn sys_print(s: &str) -> Result<(), &'static str> {
    let result = syscall_handler(
        file::SYS_WRITE,
        STDOUT_FD as usize,
        s.as_ptr() as usize,
        s.len(),
        0,
    );
    
    match result {
        SysCallResult::Success(_) => Ok(()),
        SysCallResult::Error(e) => Err(e),
    }
}

// Utility function for memory info using SYS_GETMEMINFO  
pub fn sys_memory_info() -> Result<(), &'static str> {
    let result = syscall_handler(memory::SYS_GETMEMINFO, 0, 0, 0, 0);
    match result {
        SysCallResult::Success(_) => Ok(()),
        SysCallResult::Error(e) => Err(e),
    }
}

// Utility function for device info using SYS_GETDEVICES
pub fn sys_device_info() -> Result<(), &'static str> {
    let result = syscall_handler(device::SYS_GETDEVICES, 0, 0, 0, 0);
    match result {
        SysCallResult::Success(_) => Ok(()),
        SysCallResult::Error(e) => Err(e),
    }
}

// Debug function to show syscall categories
pub fn sys_show_categories() -> Result<(), &'static str> {
    sys_print("System Call Categories:\n")?;
    sys_print("  1-50:   File I/O Operations\n")?;
    sys_print("  51-70:  Directory Operations\n")?;
    sys_print("  71-120: Memory Management\n")?;
    sys_print("  121-170: Process Management\n")?;
    sys_print("  171-220: Device and I/O Management\n")?;
    sys_print("  221-270: Network Operations\n")?;
    sys_print("  271-300: Time and Timer Operations\n")?;
    sys_print("  301-350: System Information\n")?;
    sys_print("  900-999: elinOS-Specific Operations\n")?;
    Ok(())
} 