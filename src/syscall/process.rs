// Process Management System Calls (121-170)
// Handles process operations like exit, fork, execve, kill, etc.

use crate::UART;
use core::fmt::Write;
use super::SysCallResult;

// === PROCESS MANAGEMENT SYSTEM CALL CONSTANTS (121-170) ===
pub const SYS_EXIT: usize = 121;
pub const SYS_FORK: usize = 122;
pub const SYS_EXECVE: usize = 123;
pub const SYS_WAIT: usize = 124;
pub const SYS_WAITPID: usize = 125;
pub const SYS_GETPID: usize = 126;
pub const SYS_GETPPID: usize = 127;
pub const SYS_KILL: usize = 128;
pub const SYS_SIGNAL: usize = 129;
// Reserved for future process management: 130-170

// Handle process management system calls
pub fn handle_process_syscall(
    syscall_num: usize,
    arg0: usize,
    arg1: usize,
    arg2: usize,
    _arg3: usize,
) -> SysCallResult {
    match syscall_num {
        SYS_EXIT => sys_exit(arg0 as i32),
        SYS_FORK => sys_fork(),
        SYS_EXECVE => sys_execve(arg0 as *const u8, arg1 as *const *const u8, arg2 as *const *const u8),
        SYS_WAIT => sys_wait(arg0 as *mut i32),
        SYS_WAITPID => sys_waitpid(arg0 as i32, arg1 as *mut i32, arg2 as i32),
        SYS_GETPID => sys_getpid(),
        SYS_GETPPID => sys_getppid(),
        SYS_KILL => sys_kill(arg0 as i32, arg1 as i32),
        SYS_SIGNAL => sys_signal(arg0 as i32, arg1),
        _ => SysCallResult::Error("Unknown process management system call"),
    }
}

// === SYSTEM CALL IMPLEMENTATIONS ===

fn sys_exit(status: i32) -> SysCallResult {
    let mut uart = UART.lock();
    let _ = writeln!(uart, "Process exited with status: {}", status);
    // In a real OS, this would terminate the process
    // For now, we just return success
    SysCallResult::Success(status as isize)
}

fn sys_fork() -> SysCallResult {
    // TODO: Implement process forking
    SysCallResult::Error("fork not implemented")
}

fn sys_execve(_filename: *const u8, _argv: *const *const u8, _envp: *const *const u8) -> SysCallResult {
    // TODO: Implement program execution
    SysCallResult::Error("execve not implemented")
}

fn sys_wait(_status: *mut i32) -> SysCallResult {
    // TODO: Implement wait for child process
    SysCallResult::Error("wait not implemented")
}

fn sys_waitpid(_pid: i32, _status: *mut i32, _options: i32) -> SysCallResult {
    // TODO: Implement wait for specific child process
    SysCallResult::Error("waitpid not implemented")
}

fn sys_getpid() -> SysCallResult {
    // TODO: Return actual process ID
    // For now, return a fake PID
    SysCallResult::Success(1)
}

fn sys_getppid() -> SysCallResult {
    // TODO: Return actual parent process ID
    // For now, return a fake PPID
    SysCallResult::Success(0)
}

fn sys_kill(_pid: i32, _sig: i32) -> SysCallResult {
    // TODO: Implement signal sending to process
    SysCallResult::Error("kill not implemented")
}

fn sys_signal(_signum: i32, _handler: usize) -> SysCallResult {
    // TODO: Implement signal handler registration
    SysCallResult::Error("signal not implemented")
} 