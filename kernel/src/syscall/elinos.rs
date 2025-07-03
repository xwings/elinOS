// elinOS-Specific System Calls (900-999)
// Handles elinOS-specific operations like debug, version, stats, etc.

use elinos_common::{sbi, console_println};
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
    match args.syscall_number {
        SYS_ELINOS_VERSION => sys_elinos_version(),
        SYS_ELINOS_DEBUG => sys_elinos_debug(args.arg0_as_ptr::<u8>()),
        SYS_ELINOS_SHUTDOWN => sys_elinos_shutdown(),
        SYS_ELINOS_REBOOT => sys_elinos_reboot(),
        SYS_LOAD_ELF => super::process::sys_load_elf(args.arg0_as_ptr::<u8>(), args.arg1),
        SYS_EXEC_ELF => super::process::sys_exec_elf(args.arg0_as_ptr::<u8>(), args.arg1),
        SYS_ELF_INFO => super::process::sys_elf_info(args.arg0_as_ptr::<u8>(), args.arg1),
        _ => SysCallResult::Error(crate::syscall::ENOSYS),
    }
}

// === SYSTEM CALL IMPLEMENTATIONS ===

pub fn sys_elinos_version() -> SysCallResult {
    console_println!("elinOS Information:");
    console_println!("===============================================");
    console_println!();
    
    console_println!("RISC-V64 Experimental Operating System");
    console_println!("Written in Rust for research and development");
    console_println!();
    
    console_println!("Architecture:");
    console_println!("  Target: riscv64gc-unknown-none-elf");
    console_println!("  Memory Model: sv39 (future)");
    console_println!("  Privilege Level: Machine Mode");
    console_println!();
    
    console_println!("Features:");
    console_println!("  - VirtIO Block Device Support");
    console_println!("  - ext2 Filesystem");
    console_println!("  - Linux-Compatible System Calls");
    console_println!("  - Memory Management");
    console_println!("  - Simple Interactive Shell");
    console_println!();
    
    console_println!("Build Information:");
    console_println!("  Compiler: rustc (nightly)");
    console_println!("  Built: [compile time]");
    console_println!("  Kernel: elinOS");
    
    SysCallResult::Success(0)
}

fn sys_elinos_debug(msg_ptr: *const u8) -> SysCallResult {
    if msg_ptr.is_null() {
        return SysCallResult::Error(crate::syscall::EINVAL);
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
    
    console_println!("[i] DEBUG: {}", debug_msg);
    SysCallResult::Success(0)
}

pub fn sys_elinos_shutdown() -> SysCallResult {
    console_println!("[i] System shutdown requested");
    console_println!("[i] Goodbye from elinOS!");
    
    // Call the SBI shutdown function
    sbi::system_shutdown();
}

/// SYS_REBOOT - reboot the system  
pub fn sys_elinos_reboot() -> SysCallResult {
    console_println!("[i] System reboot requested");
    console_println!("[i] Rebooting elinOS...");
    
    // Call the SBI reboot function
    sbi::system_reset();
} 