// Device and I/O Management System Calls - Linux Compatible Numbers  
// Following Linux ARM64/RISC-V syscall numbers for compatibility

use core::fmt::Write;
use spin::Mutex;
use crate::UART;
use super::{SysCallResult, SyscallArgs};

// === LINUX COMPATIBLE DEVICE AND I/O MANAGEMENT SYSTEM CALL CONSTANTS ===
pub const SYS_DUP: usize = 23;         // Linux: dup
pub const SYS_DUP3: usize = 24;        // Linux: dup3
pub const SYS_FCNTL: usize = 25;       // Linux: fcntl
pub const SYS_INOTIFY_INIT1: usize = 26; // Linux: inotify_init1
pub const SYS_INOTIFY_ADD_WATCH: usize = 27; // Linux: inotify_add_watch
pub const SYS_INOTIFY_RM_WATCH: usize = 28;  // Linux: inotify_rm_watch
pub const SYS_IOCTL: usize = 29;       // Linux: ioctl
pub const SYS_IOPRIO_SET: usize = 30;  // Linux: ioprio_set
pub const SYS_IOPRIO_GET: usize = 31;  // Linux: ioprio_get
pub const SYS_FLOCK: usize = 32;       // Linux: flock
pub const SYS_MKNODAT: usize = 33;     // Linux: mknodat
pub const SYS_PIPE2: usize = 59;       // Linux: pipe2

// Legacy syscall aliases for backwards compatibility
pub const SYS_PIPE: usize = SYS_PIPE2; // Map pipe to pipe2
pub const SYS_DUP2: usize = SYS_DUP3;  // Map dup2 to dup3

// elinOS-specific device syscalls (keeping high numbers to avoid conflicts)
pub const SYS_GETDEVICES: usize = 950; // elinOS: get device info

// Linux compatible device management syscall handler
pub fn handle_device_syscall(args: &SyscallArgs) -> SysCallResult {
    match args.syscall_number {
        SYS_IOCTL => sys_ioctl(args.arg0_as_i32(), args.arg1, args.arg2),
        SYS_FCNTL => sys_fcntl(args.arg0_as_i32(), args.arg1_as_i32(), args.arg2),
        SYS_PIPE2 => sys_pipe2(args.arg0_as_mut_ptr::<i32>(), args.arg1_as_i32()),
        SYS_DUP => sys_dup(args.arg0_as_i32()),
        SYS_DUP3 => sys_dup3(args.arg0_as_i32(), args.arg1_as_i32(), args.arg2_as_i32()),
        SYS_FLOCK => sys_flock(args.arg0_as_i32(), args.arg1_as_i32()),
        SYS_MKNODAT => sys_mknodat(args.arg0_as_i32(), args.arg1_as_ptr::<u8>(), args.arg2 as u32, args.arg3 as u32),
        SYS_INOTIFY_INIT1 => sys_inotify_init1(args.arg0_as_i32()),
        SYS_INOTIFY_ADD_WATCH => sys_inotify_add_watch(args.arg0_as_i32(), args.arg1_as_ptr::<u8>(), args.arg2 as u32),
        SYS_INOTIFY_RM_WATCH => sys_inotify_rm_watch(args.arg0_as_i32(), args.arg1_as_i32()),
        SYS_IOPRIO_SET => sys_ioprio_set(args.arg0_as_i32(), args.arg1_as_i32(), args.arg2_as_i32()),
        SYS_IOPRIO_GET => sys_ioprio_get(args.arg0_as_i32(), args.arg1_as_i32()),
        SYS_GETDEVICES => sys_getdevices(),
        _ => SysCallResult::Error(crate::syscall::ENOSYS),
    }
}

// === SYSTEM CALL IMPLEMENTATIONS ===

fn sys_ioctl(_fd: i32, _request: usize, _arg: usize) -> SysCallResult {
    // TODO: Implement I/O control
    SysCallResult::Error(crate::syscall::ENOSYS)
}

fn sys_fcntl(_fd: i32, _cmd: i32, _arg: usize) -> SysCallResult {
    // TODO: Implement file control
    SysCallResult::Error(crate::syscall::ENOSYS)
}

fn sys_pipe2(_pipefd: *mut i32, _flags: i32) -> SysCallResult {
    // TODO: Implement pipe creation with flags
    SysCallResult::Error(crate::syscall::ENOSYS)
}

fn sys_dup(_oldfd: i32) -> SysCallResult {
    // TODO: Implement file descriptor duplication
    SysCallResult::Error(crate::syscall::ENOSYS)
}

fn sys_dup3(_oldfd: i32, _newfd: i32, _flags: i32) -> SysCallResult {
    // TODO: Implement file descriptor duplication to specific fd with flags
    SysCallResult::Error(crate::syscall::ENOSYS)
}

fn sys_flock(_fd: i32, _operation: i32) -> SysCallResult {
    // TODO: Implement file locking
    SysCallResult::Error(crate::syscall::ENOSYS)
}

fn sys_mknodat(_dirfd: i32, _pathname: *const u8, _mode: u32, _dev: u32) -> SysCallResult {
    // TODO: Implement device node creation
    SysCallResult::Error(crate::syscall::ENOSYS)
}

fn sys_inotify_init1(_flags: i32) -> SysCallResult {
    // TODO: Implement inotify initialization
    SysCallResult::Error(crate::syscall::ENOSYS)
}

fn sys_inotify_add_watch(_fd: i32, _pathname: *const u8, _mask: u32) -> SysCallResult {
    // TODO: Implement inotify watch addition
    SysCallResult::Error(crate::syscall::ENOSYS)
}

fn sys_inotify_rm_watch(_fd: i32, _wd: i32) -> SysCallResult {
    // TODO: Implement inotify watch removal
    SysCallResult::Error(crate::syscall::ENOSYS)
}

fn sys_ioprio_set(_which: i32, _who: i32, _ioprio: i32) -> SysCallResult {
    // TODO: Implement I/O priority setting
    SysCallResult::Error(crate::syscall::ENOSYS)
}

fn sys_ioprio_get(_which: i32, _who: i32) -> SysCallResult {
    // TODO: Implement I/O priority getting
    SysCallResult::Error(crate::syscall::ENOSYS)
}

fn sys_getdevices() -> SysCallResult {
    SysCallResult::Success(0)
} 