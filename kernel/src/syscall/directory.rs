// Directory Operations System Calls (51-70)
// Handles directory operations like mkdir, rmdir, chdir, etc.

use super::{SysCallResult, SyscallArgs};

// === DIRECTORY OPERATIONS SYSTEM CALL CONSTANTS (51-70) ===
pub const SYS_MKDIR: usize = 51;
pub const SYS_RMDIR: usize = 52;
pub const SYS_CHDIR: usize = 53;
pub const SYS_GETCWD: usize = 54;
// Reserved for future directory operations: 55-70

// Standardized directory syscall handler
pub fn handle_directory_syscall(args: &SyscallArgs) -> SysCallResult {
    match args.syscall_number {
        SYS_MKDIR => sys_mkdir(args.arg0_as_ptr::<u8>(), args.arg1 as u32),
        SYS_RMDIR => sys_rmdir(args.arg0_as_ptr::<u8>()),
        SYS_CHDIR => sys_chdir(args.arg0_as_ptr::<u8>()),
        SYS_GETCWD => sys_getcwd(args.arg0_as_mut_ptr::<u8>(), args.arg1),
        _ => SysCallResult::Error(crate::syscall::ENOSYS),
    }
}

// === SYSTEM CALL IMPLEMENTATIONS ===

fn sys_mkdir(_pathname: *const u8, _mode: u32) -> SysCallResult {
    // TODO: Implement directory creation
    SysCallResult::Error(crate::syscall::ENOSYS)
}

fn sys_rmdir(_pathname: *const u8) -> SysCallResult {
    // TODO: Implement directory removal
    SysCallResult::Error(crate::syscall::ENOSYS)
}

fn sys_chdir(_path: *const u8) -> SysCallResult {
    // TODO: Implement change directory
    SysCallResult::Error(crate::syscall::ENOSYS)
}

fn sys_getcwd(_buf: *mut u8, _size: usize) -> SysCallResult {
    // TODO: Implement get current working directory
    SysCallResult::Error(crate::syscall::ENOSYS)
} 