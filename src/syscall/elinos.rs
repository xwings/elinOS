// ElinOS-Specific System Calls (900-999)
// Handles ElinOS-specific operations like debug, version, stats, etc.

use crate::UART;
use crate::sbi;
use core::fmt::Write;
use super::{SysCallResult, SyscallArgs};

// === ELINOS-SPECIFIC SYSTEM CALL CONSTANTS (900-999) ===
pub const SYS_ELINOS_DEBUG: usize = 900;
pub const SYS_ELINOS_STATS: usize = 901;
pub const SYS_ELINOS_VERSION: usize = 902;
pub const SYS_ELINOS_SHUTDOWN: usize = 903;
pub const SYS_ELINOS_REBOOT: usize = 904;
// Reserved for ElinOS-specific: 905-999

// Standardized ElinOS-specific syscall handler
pub fn handle_elinos_syscall(args: &SyscallArgs) -> SysCallResult {
    match args.syscall_num {
        SYS_ELINOS_DEBUG => sys_elinos_debug(args.arg0),
        SYS_ELINOS_STATS => sys_elinos_stats(),
        SYS_ELINOS_VERSION => sys_elinos_version(),
        SYS_ELINOS_SHUTDOWN => sys_elinos_shutdown(),
        SYS_ELINOS_REBOOT => sys_elinos_reboot(),
        _ => SysCallResult::Error("Unknown ElinOS-specific system call"),
    }
}

// === SYSTEM CALL IMPLEMENTATIONS ===

fn sys_elinos_debug(level: usize) -> SysCallResult {
    let mut uart = UART.lock();
    let _ = writeln!(uart, "ElinOS debug level set to: {}", level);
    SysCallResult::Success(0)
}

fn sys_elinos_stats() -> SysCallResult {
    let mut uart = UART.lock();
    let _ = writeln!(uart, "ElinOS System Statistics:");
    let _ = writeln!(uart, "  - Syscall categories: 9");
    let _ = writeln!(uart, "  - Implemented syscalls: 10");
    let _ = writeln!(uart, "  - Total syscall range: 999");
    let _ = writeln!(uart, "  - Architecture: RISC-V 64-bit");
    let _ = writeln!(uart, "  - Language: Rust (no_std)");
    SysCallResult::Success(0)
}

fn sys_elinos_version() -> SysCallResult {
    let mut uart = UART.lock();
    let _ = writeln!(uart, "ElinOS v0.1.0 - RISC-V Operating System");
    let _ = writeln!(uart, "Built with Rust and proper syscall architecture");
    let _ = writeln!(uart, "Organized syscalls inspired by Qiling framework");
    SysCallResult::Success(0)
}

fn sys_elinos_shutdown() -> SysCallResult {
    let mut uart = UART.lock();
    let _ = writeln!(uart, "ElinOS shutting down...");
    let _ = writeln!(uart, "Goodbye!");
    drop(uart);
    
    // Shutdown the system using SBI
    sbi::shutdown();
}

fn sys_elinos_reboot() -> SysCallResult {
    let mut uart = UART.lock();
    let _ = writeln!(uart, "ElinOS rebooting...");
    drop(uart);
    
    // Reboot the system using SBI  
    sbi::reboot();
} 