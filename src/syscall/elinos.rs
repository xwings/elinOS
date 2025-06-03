// elinOS-Specific System Calls (900-999)
// Handles elinOS-specific operations like debug, version, stats, etc.

use crate::{sbi, console_println};
use super::{SysCallResult, SyscallArgs};

// === ELINOS-SPECIFIC SYSTEM CALL CONSTANTS (900-999) ===
pub const SYS_ELINOS_DEBUG: usize = 900;
pub const SYS_ELINOS_STATS: usize = 901;
pub const SYS_ELINOS_VERSION: usize = 902;
pub const SYS_ELINOS_SHUTDOWN: usize = 903;
pub const SYS_ELINOS_REBOOT: usize = 904;
pub const SYS_LOAD_ELF: usize = 905;
pub const SYS_EXEC_ELF: usize = 906;
pub const SYS_ELF_INFO: usize = 907;
// Reserved for elinOS-specific: 905-999

// elinOS-specific syscall handler
pub fn handle_elinos_syscall(args: &SyscallArgs) -> SysCallResult {
    match args.syscall_num {
        SYS_ELINOS_VERSION => sys_elinos_version(),
        SYS_ELINOS_DEBUG => sys_elinos_debug(args.arg0_as_ptr::<u8>()),
        SYS_ELINOS_SHUTDOWN => sys_elinos_shutdown(args.arg0_as_i32()),
        SYS_ELINOS_REBOOT => sys_elinos_reboot(),
        SYS_LOAD_ELF => super::process::sys_load_elf(args.arg0_as_ptr::<u8>(), args.arg1),
        SYS_EXEC_ELF => super::process::sys_exec_elf(args.arg0_as_ptr::<u8>(), args.arg1),
        SYS_ELF_INFO => super::process::sys_elf_info(args.arg0_as_ptr::<u8>(), args.arg1),
        _ => SysCallResult::Error("Unknown elinOS system call"),
    }
}

// === SYSTEM CALL IMPLEMENTATIONS ===

fn sys_elinos_version() -> SysCallResult {
    console_println!("elinOS Version 0.1.0");
    console_println!("RISC-V Educational Operating System");
    console_println!("Built with Rust for learning and research");
    SysCallResult::Success(0)
}

fn sys_elinos_debug(msg_ptr: *const u8) -> SysCallResult {
    if msg_ptr.is_null() {
        return SysCallResult::Error("Invalid debug message pointer");
    }
    
    // Convert raw pointer to string (unsafe but necessary for syscall interface)
    let debug_msg = unsafe {
        let mut len = 0;
        let mut ptr = msg_ptr;
        while *ptr != 0 && len < 1024 { // Max 1KB debug message
            ptr = ptr.add(1);
            len += 1;
        }
        core::str::from_utf8_unchecked(core::slice::from_raw_parts(msg_ptr, len))
    };
    
    console_println!("DEBUG: {}", debug_msg);
    SysCallResult::Success(0)
}

fn sys_elinos_shutdown(status: i32) -> SysCallResult {
    console_println!("System shutdown requested with status: {}", status);
    
    // Use SBI to shutdown the system
    sbi::system_shutdown();
    
    // This should not be reached
    SysCallResult::Success(0)
}

fn sys_elinos_reboot() -> SysCallResult {
    console_println!("System reboot requested");
    
    // Use SBI to reboot the system  
    sbi::system_reset();
    
    // This should not be reached
    SysCallResult::Success(0)
} 