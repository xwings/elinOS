// elinOS-Specific System Calls (900-999)
// Handles elinOS-specific operations like debug, version, stats, etc.

use crate::{sbi, console_println};
use super::{SysCallResult, SyscallArgs};
use core::arch::asm;

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
    if let Err(e) = crate::syscall::sys_print("elinOS Version Information:\n") {
        return SysCallResult::Error(crate::syscall::EIO);
    }
    if let Err(e) = crate::syscall::sys_print("===============================================\n\n") {
        return SysCallResult::Error(crate::syscall::EIO);
    }
    
    if let Err(e) = crate::syscall::sys_print("🦀 elinOS v0.1.0\n") {
        return SysCallResult::Error(crate::syscall::EIO);
    }
    if let Err(e) = crate::syscall::sys_print("RISC-V Experimental Operating System\n") {
        return SysCallResult::Error(crate::syscall::EIO);
    }
    if let Err(e) = crate::syscall::sys_print("Written in Rust for research and development\n\n") {
        return SysCallResult::Error(crate::syscall::EIO);
    }
    
    if let Err(e) = crate::syscall::sys_print("Architecture:\n") {
        return SysCallResult::Error(crate::syscall::EIO);
    }
    if let Err(e) = crate::syscall::sys_print("  Target: riscv64gc-unknown-none-elf\n") {
        return SysCallResult::Error(crate::syscall::EIO);
    }
    if let Err(e) = crate::syscall::sys_print("  Memory Model: sv39 (future)\n") {
        return SysCallResult::Error(crate::syscall::EIO);
    }
    if let Err(e) = crate::syscall::sys_print("  Privilege Level: Machine Mode\n\n") {
        return SysCallResult::Error(crate::syscall::EIO);
    }
    
    if let Err(e) = crate::syscall::sys_print("Features:\n") {
        return SysCallResult::Error(crate::syscall::EIO);
    }
    if let Err(e) = crate::syscall::sys_print("  ✅ VirtIO Block Device Support\n") {
        return SysCallResult::Error(crate::syscall::EIO);
    }
    if let Err(e) = crate::syscall::sys_print("  ✅ FAT32/ext2 Filesystem\n") {
        return SysCallResult::Error(crate::syscall::EIO);
    }
    if let Err(e) = crate::syscall::sys_print("  ✅ Automatic Filesystem Detection\n") {
        return SysCallResult::Error(crate::syscall::EIO);
    }
    if let Err(e) = crate::syscall::sys_print("  ✅ Linux-Compatible System Calls\n") {
        return SysCallResult::Error(crate::syscall::EIO);
    }
    if let Err(e) = crate::syscall::sys_print("  ✅ Memory Management\n") {
        return SysCallResult::Error(crate::syscall::EIO);
    }
    if let Err(e) = crate::syscall::sys_print("  ✅ Simple Interactive Shell\n\n") {
        return SysCallResult::Error(crate::syscall::EIO);
    }
    
    if let Err(e) = crate::syscall::sys_print("Build Information:\n") {
        return SysCallResult::Error(crate::syscall::EIO);
    }
    if let Err(e) = crate::syscall::sys_print("  Compiler: rustc (nightly)\n") {
        return SysCallResult::Error(crate::syscall::EIO);
    }
    if let Err(e) = crate::syscall::sys_print("  Built: [compile time]\n") {
        return SysCallResult::Error(crate::syscall::EIO);
    }
    if let Err(e) = crate::syscall::sys_print("  Kernel: elinOS\n") {
        return SysCallResult::Error(crate::syscall::EIO);
    }
    
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
    
    console_println!("🔍 DEBUG: {}", debug_msg);
    SysCallResult::Success(0)
}

pub fn sys_elinos_shutdown() -> SysCallResult {
    console_println!("💤 System shutdown requested");
    console_println!("🏁 Goodbye from elinOS!");
    
    // Call the SBI shutdown function
    sbi::system_shutdown();
}

/// SYS_REBOOT - reboot the system  
pub fn sys_elinos_reboot() -> SysCallResult {
    console_println!("🔄 System reboot requested");
    console_println!("🔄 Rebooting elinOS...");
    
    // Call the SBI reboot function
    sbi::system_reset();
} 