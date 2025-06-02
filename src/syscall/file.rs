// File I/O System Calls (1-50)
// Handles file operations like read, write, open, close, etc.

use crate::UART;
use crate::filesystem;
use core::fmt::Write;
use super::{SysCallResult, SyscallArgs, STDOUT_FD, STDERR_FD};

// === FILE I/O SYSTEM CALL CONSTANTS (1-50) ===
pub const SYS_WRITE: usize = 1;
pub const SYS_READ: usize = 2;
pub const SYS_OPEN: usize = 3;
pub const SYS_CLOSE: usize = 4;
pub const SYS_UNLINK: usize = 5;
pub const SYS_GETDENTS: usize = 6;  // List directory entries
pub const SYS_STAT: usize = 9;
pub const SYS_LSEEK: usize = 10;
pub const SYS_TRUNCATE: usize = 11;
pub const SYS_FTRUNCATE: usize = 12;
pub const SYS_CHMOD: usize = 13;
pub const SYS_CHOWN: usize = 14;
pub const SYS_LINK: usize = 15;
pub const SYS_RENAME: usize = 16;
pub const SYS_SYNC: usize = 17;
pub const SYS_FSYNC: usize = 18;
// Reserved for future file I/O syscalls: 19-50

// File operation flags
pub const O_RDONLY: i32 = 0;
pub const O_WRONLY: i32 = 1;
pub const O_RDWR: i32 = 2;
pub const O_CREAT: i32 = 64;
pub const O_TRUNC: i32 = 512;
pub const O_APPEND: i32 = 1024;

// Standardized file I/O syscall handler
pub fn handle_file_syscall(args: &SyscallArgs) -> SysCallResult {
    match args.syscall_num {
        SYS_WRITE => sys_write(args.arg0_as_i32(), args.arg1_as_ptr::<u8>(), args.arg2),
        SYS_READ => sys_read(args.arg0_as_i32(), args.arg1_as_mut_ptr::<u8>(), args.arg2),
        SYS_OPEN => sys_open(args.arg0_as_ptr::<u8>(), args.arg1_as_i32()),
        SYS_CLOSE => sys_close(args.arg0_as_i32()),
        SYS_UNLINK => sys_unlink(args.arg0_as_ptr::<u8>()),
        SYS_GETDENTS => sys_getdents(args.arg0_as_mut_ptr::<u8>(), args.arg1),
        SYS_STAT => sys_stat(args.arg0_as_ptr::<u8>(), args.arg1_as_mut_ptr::<u8>()),
        SYS_LSEEK => sys_lseek(args.arg0_as_i32(), args.arg1 as isize, args.arg2_as_i32()),
        SYS_TRUNCATE => sys_truncate(args.arg0_as_ptr::<u8>(), args.arg1),
        SYS_FTRUNCATE => sys_ftruncate(args.arg0_as_i32(), args.arg1),
        SYS_CHMOD => sys_chmod(args.arg0_as_ptr::<u8>(), args.arg1 as u32),
        SYS_CHOWN => sys_chown(args.arg0_as_ptr::<u8>(), args.arg1 as u32, args.arg2 as u32),
        SYS_LINK => sys_link(args.arg0_as_ptr::<u8>(), args.arg1_as_ptr::<u8>()),
        SYS_RENAME => sys_rename(args.arg0_as_ptr::<u8>(), args.arg1_as_ptr::<u8>()),
        SYS_SYNC => sys_sync(),
        SYS_FSYNC => sys_fsync(args.arg0_as_i32()),
        _ => SysCallResult::Error("Unknown file I/O system call"),
    }
}

// === SYSTEM CALL IMPLEMENTATIONS ===

fn sys_write(fd: i32, buf: *const u8, count: usize) -> SysCallResult {
    if fd == STDOUT_FD || fd == STDERR_FD {
        // Write to console
        unsafe {
            let slice = core::slice::from_raw_parts(buf, count);
            let uart = UART.lock();
            for &byte in slice {
                uart.putchar(byte);
            }
        }
        SysCallResult::Success(count as isize)
    } else {
        // TODO: File write support with proper file descriptor management
        SysCallResult::Error("File write not implemented")
    }
}

fn sys_read(_fd: i32, _buf: *mut u8, _count: usize) -> SysCallResult {
    // TODO: Implement file/stdin reading
    SysCallResult::Error("Read not implemented")
}

fn sys_open(pathname: *const u8, _flags: i32) -> SysCallResult {
    unsafe {
        // Convert C string to Rust string
        let mut len = 0;
        let mut ptr = pathname;
        while *ptr != 0 && len < 256 {
            len += 1;
            ptr = ptr.add(1);
        }
        
        let slice = core::slice::from_raw_parts(pathname, len);
        if let Ok(filename) = core::str::from_utf8(slice) {
            let fs = filesystem::FILESYSTEM.lock();
            if fs.file_exists(filename) {
                // Return a fake file descriptor (just index + 10)
                SysCallResult::Success(10)
            } else {
                SysCallResult::Error("File not found")
            }
        } else {
            SysCallResult::Error("Invalid filename")
        }
    }
}

fn sys_close(_fd: i32) -> SysCallResult {
    // TODO: Implement proper file descriptor management
    SysCallResult::Success(0)
}

fn sys_unlink(pathname: *const u8) -> SysCallResult {
    unsafe {
        // Convert C string to Rust string
        let mut len = 0;
        let mut ptr = pathname;
        while *ptr != 0 && len < 256 {
            len += 1;
            ptr = ptr.add(1);
        }
        
        let slice = core::slice::from_raw_parts(pathname, len);
        if let Ok(filename) = core::str::from_utf8(slice) {
            let mut fs = filesystem::FILESYSTEM.lock();
            match fs.delete_file(filename) {
                Ok(()) => SysCallResult::Success(0),
                Err(e) => SysCallResult::Error(e),
            }
        } else {
            SysCallResult::Error("Invalid filename")
        }
    }
}

fn sys_getdents(buf: *mut u8, buflen: usize) -> SysCallResult {
    unsafe {
        let fs = filesystem::FILESYSTEM.lock();
        let mut written = 0;
        let mut ptr = buf;
        
        for (name, size) in fs.list_files() {
            // Manual string formatting instead of format! macro
            let mut entry_str = heapless::String::<128>::new();
            let _ = write!(entry_str, "{} {} bytes\n", name, size);
            let entry_bytes = entry_str.as_bytes();
            
            if written + entry_bytes.len() >= buflen {
                break;
            }
            
            core::ptr::copy_nonoverlapping(
                entry_bytes.as_ptr(),
                ptr,
                entry_bytes.len()
            );
            
            ptr = ptr.add(entry_bytes.len());
            written += entry_bytes.len();
        }
        
        SysCallResult::Success(written as isize)
    }
}

fn sys_stat(pathname: *const u8, statbuf: *mut u8) -> SysCallResult {
    unsafe {
        // Convert C string to Rust string
        let mut len = 0;
        let mut ptr = pathname;
        while *ptr != 0 && len < 256 {
            len += 1;
            ptr = ptr.add(1);
        }
        
        let slice = core::slice::from_raw_parts(pathname, len);
        if let Ok(filename) = core::str::from_utf8(slice) {
            let fs = filesystem::FILESYSTEM.lock();
            if let Some(content) = fs.read_file(filename) {
                // Simple stat structure: just file size as usize
                let size = content.len();
                core::ptr::write(statbuf as *mut usize, size);
                SysCallResult::Success(0)
            } else {
                SysCallResult::Error("File not found")
            }
        } else {
            SysCallResult::Error("Invalid filename")
        }
    }
}

// === TODO: IMPLEMENT ADDITIONAL FILE OPERATIONS ===

fn sys_lseek(_fd: i32, _offset: isize, _whence: i32) -> SysCallResult {
    // TODO: Implement file seek
    SysCallResult::Error("lseek not implemented")
}

fn sys_truncate(_path: *const u8, _length: usize) -> SysCallResult {
    // TODO: Implement file truncation
    SysCallResult::Error("truncate not implemented")
}

fn sys_ftruncate(_fd: i32, _length: usize) -> SysCallResult {
    // TODO: Implement file descriptor truncation
    SysCallResult::Error("ftruncate not implemented")
}

fn sys_chmod(_path: *const u8, _mode: u32) -> SysCallResult {
    // TODO: Implement file permissions
    SysCallResult::Error("chmod not implemented")
}

fn sys_chown(_path: *const u8, _owner: u32, _group: u32) -> SysCallResult {
    // TODO: Implement file ownership
    SysCallResult::Error("chown not implemented")
}

fn sys_link(_oldpath: *const u8, _newpath: *const u8) -> SysCallResult {
    // TODO: Implement hard links
    SysCallResult::Error("link not implemented")
}

fn sys_rename(_oldpath: *const u8, _newpath: *const u8) -> SysCallResult {
    // TODO: Implement file renaming
    SysCallResult::Error("rename not implemented")
}

fn sys_sync() -> SysCallResult {
    // TODO: Implement filesystem sync
    SysCallResult::Error("sync not implemented")
}

fn sys_fsync(_fd: i32) -> SysCallResult {
    // TODO: Implement file sync
    SysCallResult::Error("fsync not implemented")
} 