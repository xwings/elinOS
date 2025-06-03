// File I/O System Calls - Linux Compatible Numbers
// Following Linux ARM64/RISC-V syscall numbers for compatibility

use crate::UART;
use crate::filesystem;
use crate::console_println;
use core::fmt::Write;
use super::{SysCallResult, SyscallArgs, STDOUT_FD, STDERR_FD};
use spin::Mutex;
use heapless::FnvIndexMap;

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

fn sys_read(fd: i32, buf: *mut u8, count: usize) -> SysCallResult {
    console_println!("🔍 SYSCALL: sys_read(fd={}, buf={:p}, count={})", fd, buf, count);
    
    if fd == 0 { // stdin
        // TODO: Implement stdin reading
        SysCallResult::Error("Stdin read not implemented")
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
                console_println!("❌ SYSCALL: Invalid file descriptor {}", fd);
                drop(file_table);
                return SysCallResult::Error("Invalid file descriptor");
            }
        };
        drop(file_table);
        
        console_println!("📖 SYSCALL: Reading file '{}'", filename.as_str());
        
        // Read the file content using the filename
        let fs = filesystem::FILESYSTEM.lock();
        match fs.read_file(filename.as_str()) {
            Ok(content) => {
                console_println!("✅ SYSCALL: File read successful, {} bytes", content.len());
                drop(fs); // Release lock before operations
                
                if count == 0 {
                    console_println!("⚠️ SYSCALL: Zero-length read requested");
                    return SysCallResult::Success(0);
                }
                
                let bytes_to_copy = core::cmp::min(count, content.len());
                console_println!("📏 SYSCALL: Will output {} bytes (requested={}, available={})", 
                    bytes_to_copy, count, content.len());
                
                // For educational OS: if buffer is null, just print to console
                // In production OS, this would be an error, but here it's convenient for cat command
                if buf.is_null() {
                    console_println!("📄 SYSCALL: Null buffer - printing to console:");
                } else {
                    console_println!("📄 SYSCALL: Copying to user buffer and printing to console:");
                    // Copy to user buffer
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
            Err(e) => {
                drop(fs);
                console_println!("❌ SYSCALL: Failed to read file: {:?}", e);
                // Print error message
                let mut uart = crate::UART.lock();
                let _ = write!(uart, "Error reading file: {:?}\n", e);
                SysCallResult::Error("Failed to read file")
            }
        }
    } else {
        console_println!("❌ SYSCALL: Invalid file descriptor {}", fd);
        SysCallResult::Error("Invalid file descriptor")
    }
}

fn sys_openat(dirfd: i32, pathname: *const u8, flags: i32, mode: u32) -> SysCallResult {
    // For now, just handle opening files in current directory (ignore dirfd)
    // Get filename from pathname pointer
    let filename = unsafe {
        let mut len = 0;
        let mut ptr = pathname;
        
        // Find null terminator
        while len < 256 && *ptr != 0 {
            ptr = ptr.add(1);
            len += 1;
        }
        
        // Convert to string slice
        core::str::from_utf8(core::slice::from_raw_parts(pathname, len))
            .unwrap_or("")
    };
    
    // Check if file exists in filesystem
    let fs = filesystem::FILESYSTEM.lock();
    if !fs.file_exists(filename) {
        drop(fs);
        return SysCallResult::Error("File not found");
    }
    drop(fs);
    
    // Allocate new file descriptor
    let mut next_fd = NEXT_FD.lock();
    let fd = *next_fd;
    *next_fd += 1;
    drop(next_fd);
    
    // Store filename in file table
    let mut file_table = FILE_TABLE.lock();
    if let Ok(filename_string) = heapless::String::try_from(filename) {
        if file_table.insert(fd, filename_string).is_ok() {
            drop(file_table);
            SysCallResult::Success(fd as isize)
        } else {
            drop(file_table);
            SysCallResult::Error("File table full")
        }
    } else {
        drop(file_table);
        SysCallResult::Error("Filename too long")
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
            SysCallResult::Error("Invalid file descriptor")
        }
    } else {
        SysCallResult::Error("Cannot close system file descriptors")
    }
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
    let _ = buf; // We'll output directly to console instead
    let _ = buflen;
    
    // Print directory listing to console
    let mut uart = UART.lock();
    let _ = write!(uart, "Directory listing:\n");
    
    let fs = filesystem::FILESYSTEM.lock();
    match fs.list_files() {
        Ok(files) => {
            let files_len = files.len();
            for (name, size) in &files {
                let _ = write!(uart, "{:<20} {:>8} bytes\n", name.as_str(), size);
            }
            SysCallResult::Success(files_len as isize)
        }
        Err(_) => {
            let _ = write!(uart, "Failed to list files\n");
            SysCallResult::Error("Failed to list files")
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