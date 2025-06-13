// File I/O System Calls - Linux Compatible Numbers
// Following Linux ARM64/RISC-V syscall numbers for compatibility

use crate::UART;
use crate::filesystem;
use crate::{console_print, console_println};
use super::{SysCallResult, SyscallArgs, STDOUT_FD, STDERR_FD};
use spin::Mutex;
use heapless::{FnvIndexMap, Vec};
use crate::filesystem::traits::FileSystem;
use core::fmt::Write;

// Simple file descriptor table
static FILE_TABLE: Mutex<FnvIndexMap<i32, heapless::String<64>, 16>> = Mutex::new(FnvIndexMap::new());
static NEXT_FD: Mutex<i32> = Mutex::new(10); // File descriptors start at 10

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
    match args.syscall_number {
        SYS_WRITE => sys_write(args.arg0_as_i32(), args.arg1_as_ptr::<u8>(), args.arg2),
        SYS_READ => sys_read(args.arg0_as_i32(), args.arg1_as_mut_ptr::<u8>(), args.arg2),
        SYS_OPENAT => sys_openat(*args),
        SYS_CLOSE => sys_close(args.arg0_as_i32()),
        35 => sys_unlinkat(*args), // unlinkat
        SYS_GETDENTS64 => sys_getdents64(*args),
        SYS_NEWFSTATAT => sys_newfstatat(args.arg0_as_i32(), args.arg1_as_ptr::<u8>(), args.arg2_as_mut_ptr::<u8>(), args.arg3_as_i32()),
        SYS_LSEEK => sys_lseek(args.arg0_as_i32(), args.arg1 as isize, args.arg2_as_i32()),
        SYS_TRUNCATE => sys_truncate(args.arg0_as_ptr::<u8>(), args.arg1),
        SYS_FTRUNCATE => sys_ftruncate(args.arg0_as_i32(), args.arg1),
        SYS_SYNC => sys_sync(),
        SYS_FSYNC => sys_fsync(args.arg0_as_i32()),
        _ => SysCallResult::Error(crate::syscall::ENOSYS),
    }
}

// === SYSTEM CALL IMPLEMENTATIONS ===

fn sys_write(fd: i32, buf: *const u8, count: usize) -> SysCallResult {
    
    if fd == STDOUT_FD || fd == STDERR_FD {
        // Write to console
        unsafe {
            let slice = core::slice::from_raw_parts(buf, count);
            for &byte in slice {
                console_print!("{}", byte as char);
            }
        }
        SysCallResult::Success(count as isize)
    } else {
        // TODO: File write support with proper file descriptor management
                    SysCallResult::Error(crate::syscall::ENOSYS)
    }
}

fn sys_read(fd: i32, buf: *mut u8, count: usize) -> SysCallResult {
    console_println!("[+] SYSCALL: sys_read(fd={}, buf={:p}, count={})", fd, buf, count);
    
    if fd == 0 { // stdin
        // TODO: Implement stdin reading
        SysCallResult::Error(crate::syscall::ENOSYS)
    } else if fd >= 10 { // File descriptors start at 10
        console_println!("📂 SYSCALL: Looking up file descriptor {}", fd);
        
        // Look up filename from file descriptor table
        let file_table = FILE_TABLE.lock();
        let filename = match file_table.get(&fd) {
            Some(name) => {
                console_println!("✅ SYSCALL: Found filename '{}' for fd {}", name.as_str(), fd);
                name.clone()
            },
            None => {
                console_println!("[!] SYSCALL: Invalid file descriptor {}", fd);
                drop(file_table);
                return SysCallResult::Error(crate::syscall::EBADF);
            }
        };
        drop(file_table);
        
        console_println!("📖 SYSCALL: Reading file '{}'", filename.as_str());
        
        // Read the file content using the filesystem API
        let fs = filesystem::FILESYSTEM.lock();
        
        // Try to read the file using the filesystem trait
        match fs.read_file(&filename) {
            Ok(content) => {
                let bytes_to_copy = core::cmp::min(count, content.len());
                console_println!("📏 SYSCALL: Will output {} bytes (requested={}, available={})", 
                    bytes_to_copy, count, content.len());
                
                // If buffer is provided, copy to user buffer
                if !buf.is_null() {
                    unsafe {
                        core::ptr::copy_nonoverlapping(
                            content.as_ptr(),
                            buf,
                            bytes_to_copy
                        );
                    }
                }
                
                // Always print to console so user can see the file contents
                let uart = crate::UART.lock();
                for &byte in &content[..bytes_to_copy] {
                    uart.putchar(byte);
                }
                drop(uart);
                
                console_println!("✅ SYSCALL: File output complete");
                SysCallResult::Success(bytes_to_copy as isize)
            }
            Err(_) => {
                console_println!("❌ File not found: {}", filename);
                SysCallResult::Error(crate::syscall::ENOENT)
            }
        }
    } else {
        console_println!("❌ SYSCALL: Invalid file descriptor {}", fd);
        SysCallResult::Error(crate::syscall::EINVAL)
    }
}

pub fn sys_openat(args: SyscallArgs) -> SysCallResult {
    // For demo purposes, just check if file exists
    let filename = "hello.txt";  // Hardcoded for now
    
    console_println!("📂 sys_openat: opening file '{}'", filename);
    
    let fs = filesystem::FILESYSTEM.lock();
    
    if !fs.is_mounted() {
        console_println!("❌ Filesystem not mounted");
                    return SysCallResult::Error(crate::syscall::ENODEV);
    }
    
    // Check if file exists using the trait method
    if fs.file_exists(filename) {
        console_println!("✅ File '{}' found, returning fd=3", filename);
        SysCallResult::Success(3)  // Return a fake file descriptor
    } else {
        console_println!("❌ File '{}' not found", filename);
        SysCallResult::Error(crate::syscall::ENOENT)
    }
}

fn sys_close(fd: i32) -> SysCallResult {
    if fd >= 10 {
        let mut file_table = FILE_TABLE.lock();
        if file_table.remove(&fd).is_some() {
            drop(file_table);
            SysCallResult::Success(0)
        } else {
            drop(file_table);
            SysCallResult::Error(crate::syscall::EINVAL)
        }
    } else {
        SysCallResult::Error(crate::syscall::EPERM)
    }
}

pub fn sys_unlinkat(args: SyscallArgs) -> SysCallResult {
    // args.arg1 is the path
    let filename = "dummy.txt";  // For demonstration
    
    console_println!("🗑️ sys_unlinkat: deleting file '{}'", filename);
    
    let fs = filesystem::FILESYSTEM.lock();
    
    if !fs.file_exists(filename) {
        console_println!("❌ File '{}' doesn't exist", filename);
        return SysCallResult::Error(crate::syscall::ENOENT);
    }
    
    // We don't actually implement file deletion yet
    console_println!("⚠️ File deletion not implemented");
            SysCallResult::Error(crate::syscall::ENOSYS)
}

pub fn sys_getdents64(args: SyscallArgs) -> SysCallResult {
    let fd = args.arg0 as i32;
    
    console_println!("📂 sys_getdents64: listing directory for fd={}", fd);
    
    let fs = filesystem::FILESYSTEM.lock();
    
    match fs.list_files() {
        Ok(files) => {
            console_println!("✅ Found {} files:", files.len());
            for (name, size) in &files {
                console_println!("  📄 {} ({} bytes)", name.as_str(), size);
            }
            SysCallResult::Success(files.len() as isize)
        }
        Err(_) => {
            console_println!("❌ Failed to list files");
            SysCallResult::Error(crate::syscall::EIO)
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
                Err(_) => SysCallResult::Error(crate::syscall::ENOENT)
            }
        } else {
            SysCallResult::Error(crate::syscall::EINVAL)
        }
    }
}

// === TODO: IMPLEMENT ADDITIONAL FILE OPERATIONS ===

fn sys_lseek(_fd: i32, _offset: isize, _whence: i32) -> SysCallResult {
    // TODO: Implement file seek
    SysCallResult::Error(crate::syscall::ENOSYS)
}

fn sys_truncate(_path: *const u8, _length: usize) -> SysCallResult {
    // TODO: Implement file truncation
    SysCallResult::Error(crate::syscall::ENOSYS)
}

fn sys_ftruncate(_fd: i32, _length: usize) -> SysCallResult {
    // TODO: Implement file descriptor truncation
    SysCallResult::Error(crate::syscall::ENOSYS)
}

fn sys_sync() -> SysCallResult {
    // TODO: Implement filesystem sync
    SysCallResult::Error(crate::syscall::ENOSYS)
}

fn sys_fsync(_fd: i32) -> SysCallResult {
    // TODO: Implement file sync
    SysCallResult::Error(crate::syscall::ENOSYS)
}

// Helper function to read file with path (for testing)
pub fn read_file_by_path(filename: &str) -> Result<Vec<u8, 4096>, &'static str> {
    let fs = filesystem::FILESYSTEM.lock();
    
    match fs.read_file(filename) {
        Ok(content) => {
            // Convert from Vec<u8, 32768> to Vec<u8, 4096>
            let mut result = heapless::Vec::<u8, 4096>::new();
            let bytes_to_copy = content.len().min(4096);
            for i in 0..bytes_to_copy {
                if result.push(content[i]).is_err() {
                    break;
                }
            }
            Ok(result)
        },
        Err(_) => Err("File not found"),
    }
} 