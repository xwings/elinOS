// File I/O System Calls - Linux Compatible Numbers
// Following Linux ARM64/RISC-V syscall numbers for compatibility

use crate::UART;
use crate::filesystem;
use core::fmt::Write;
use super::{SysCallResult, SyscallArgs, STDOUT_FD, STDERR_FD};

// === LINUX COMPATIBLE FILE I/O SYSTEM CALL CONSTANTS ===
pub const SYS_OPENAT: usize = 56;     // Linux: openat
pub const SYS_CLOSE: usize = 57;      // Linux: close  
pub const SYS_READ: usize = 63;       // Linux: read
pub const SYS_WRITE: usize = 64;      // Linux: write
pub const SYS_READV: usize = 65;      // Linux: readv
pub const SYS_WRITEV: usize = 66;     // Linux: writev
pub const SYS_PREAD64: usize = 67;    // Linux: pread64
pub const SYS_PWRITE64: usize = 68;   // Linux: pwrite64
pub const SYS_PREADV: usize = 69;     // Linux: preadv
pub const SYS_PWRITEV: usize = 70;    // Linux: pwritev
pub const SYS_SENDFILE: usize = 71;   // Linux: sendfile
pub const SYS_PSELECT6: usize = 72;   // Linux: pselect6
pub const SYS_PPOLL: usize = 73;      // Linux: ppoll
pub const SYS_READLINKAT: usize = 78; // Linux: readlinkat
pub const SYS_NEWFSTATAT: usize = 79; // Linux: newfstatat (stat)
pub const SYS_FSTAT: usize = 80;      // Linux: fstat
pub const SYS_SYNC: usize = 81;       // Linux: sync
pub const SYS_FSYNC: usize = 82;      // Linux: fsync
pub const SYS_FDATASYNC: usize = 83;  // Linux: fdatasync
pub const SYS_LSEEK: usize = 62;      // Linux: lseek
pub const SYS_GETDENTS64: usize = 61; // Linux: getdents64

// Legacy syscall aliases for backwards compatibility
pub const SYS_OPEN: usize = SYS_OPENAT;      // Map to openat
pub const SYS_UNLINK: usize = 35;            // Linux: unlinkat (we'll handle as unlink)
pub const SYS_GETDENTS: usize = SYS_GETDENTS64; // Map to getdents64
pub const SYS_STAT: usize = SYS_NEWFSTATAT;  // Map to newfstatat
pub const SYS_TRUNCATE: usize = 45;          // Linux: truncate
pub const SYS_FTRUNCATE: usize = 46;         // Linux: ftruncate
pub const SYS_FALLOCATE: usize = 47;         // Linux: fallocate

// File operation flags
pub const O_RDONLY: i32 = 0;
pub const O_WRONLY: i32 = 1;
pub const O_RDWR: i32 = 2;
pub const O_CREAT: i32 = 64;
pub const O_TRUNC: i32 = 512;
pub const O_APPEND: i32 = 1024;

// Linux compatible file I/O syscall handler
pub fn handle_file_syscall(args: &SyscallArgs) -> SysCallResult {
    match args.syscall_num {
        SYS_WRITE => sys_write(args.arg0_as_i32(), args.arg1_as_ptr::<u8>(), args.arg2),
        SYS_READ => sys_read(args.arg0_as_i32(), args.arg1_as_mut_ptr::<u8>(), args.arg2),
        SYS_OPENAT => sys_openat(args.arg0_as_i32(), args.arg1_as_ptr::<u8>(), args.arg2_as_i32(), args.arg3 as u32),
        SYS_CLOSE => sys_close(args.arg0_as_i32()),
        35 => sys_unlinkat(args.arg0_as_i32(), args.arg1_as_ptr::<u8>(), args.arg2_as_i32()), // unlinkat
        SYS_GETDENTS64 => sys_getdents64(args.arg0_as_i32(), args.arg1_as_mut_ptr::<u8>(), args.arg2),
        SYS_NEWFSTATAT => sys_newfstatat(args.arg0_as_i32(), args.arg1_as_ptr::<u8>(), args.arg2_as_mut_ptr::<u8>(), args.arg3_as_i32()),
        SYS_LSEEK => sys_lseek(args.arg0_as_i32(), args.arg1 as isize, args.arg2_as_i32()),
        SYS_TRUNCATE => sys_truncate(args.arg0_as_ptr::<u8>(), args.arg1),
        SYS_FTRUNCATE => sys_ftruncate(args.arg0_as_i32(), args.arg1),
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

fn sys_openat(dirfd: i32, pathname: *const u8, flags: i32, _mode: u32) -> SysCallResult {
    // For now, ignore dirfd and treat as regular open
    let _ = dirfd;
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
            } else if flags & O_CREAT != 0 {
                // Create file if O_CREAT flag is set
                drop(fs); // Release lock before mutable access
                let mut fs = filesystem::FILESYSTEM.lock();
                match fs.create_file(filename, b"") {
                    Ok(_) => SysCallResult::Success(10),
                    Err(_) => SysCallResult::Error("Failed to create file")
                }
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

fn sys_unlinkat(dirfd: i32, pathname: *const u8, _flags: i32) -> SysCallResult {
    // For now, ignore dirfd and treat as regular unlink
    let _ = dirfd;
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
                Err(_) => SysCallResult::Error("Failed to delete file"),
            }
        } else {
            SysCallResult::Error("Invalid filename")
        }
    }
}

fn sys_getdents64(fd: i32, buf: *mut u8, buflen: usize) -> SysCallResult {
    let _ = fd; // Ignore fd for now, just list all files
    unsafe {
        let fs = filesystem::FILESYSTEM.lock();
        let mut written = 0;
        let mut ptr = buf;
        
        match fs.list_files() {
            Ok(files) => {
                for (name, size) in files {
                    // Manual string formatting instead of format! macro
                    let mut entry_str = heapless::String::<128>::new();
                    let _ = write!(entry_str, "{} {} bytes\n", name.as_str(), size.0);
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
            Err(_) => SysCallResult::Error("Failed to list files")
        }
    }
}

fn sys_newfstatat(dirfd: i32, pathname: *const u8, statbuf: *mut u8, _flags: i32) -> SysCallResult {
    let _ = dirfd; // Ignore dirfd for now
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
            match fs.read_file(filename) {
                Ok(content) => {
                    // Simple stat structure: just file size as usize
                    let size = content.len();
                    core::ptr::write(statbuf as *mut usize, size);
                    SysCallResult::Success(0)
                }
                Err(_) => SysCallResult::Error("File not found")
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

fn sys_sync() -> SysCallResult {
    // TODO: Implement filesystem sync
    SysCallResult::Error("sync not implemented")
}

fn sys_fsync(_fd: i32) -> SysCallResult {
    // TODO: Implement file sync
    SysCallResult::Error("fsync not implemented")
} 