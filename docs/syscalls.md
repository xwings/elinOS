# elinOS System Call Reference

## Table of Contents
- [Overview](#overview)
- [System Call Architecture](#system-call-architecture)
- [Linux Compatibility](#linux-compatibility)
- [Implemented System Calls](#implemented-system-calls)
- [elinOS Extensions](#elinos-extensions)
- [API Reference](#api-reference)
- [Usage Examples](#usage-examples)

## Overview

elinOS implements a Linux-compatible system call interface to provide familiar APIs for developers while maintaining experimental clarity. The system call layer serves as the primary interface between user space and kernel services.

### Key Features

- **Linux Compatibility**: Uses standard Linux system call numbers and conventions
- **Modular Design**: Organized by functional categories (file I/O, memory, process)
- **Type Safety**: Rust-based implementation with comprehensive error handling
- **experimental Focus**: Clear, well-documented implementation for learning
- **Extensible**: Easy to add new system calls and functionality

## System Call Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    User Space                               │
│                (Future Applications)                        │
├─────────────────────────────────────────────────────────────┤
│                 System Call Interface                      │
│                                                             │
│  ┌─────────────────┐ ┌─────────────────┐ ┌───────────────┐  │
│  │   File I/O      │ │  Memory Mgmt    │ │   Process     │  │
│  │   syscalls      │ │   syscalls      │ │   syscalls    │  │
│  │                 │ │                 │ │               │  │
│  │ • openat (56)   │ │ • mmap (222)    │ │ • getpid (93) │  │
│  │ • read (63)     │ │ • munmap (215)  │ │ • clone (220) │  │
│  │ • write (64)    │ │ • brk (214)     │ │ • execve (221)│  │
│  │ • close (57)    │ │ • mprotect (226)│ │ • exit (60)   │  │
│  │ • getdents64(61)│ │ • madvise (233) │ │ • wait4 (114) │  │
│  └─────────────────┘ └─────────────────┘ └───────────────┘  │
├─────────────────────────────────────────────────────────────┤
│                   System Call Dispatch                     │
│                                                             │
│  syscall_handler(num, arg0, arg1, arg2, arg3) -> Result    │
│                                                             │
│  ┌─────────────────┐ ┌─────────────────┐ ┌───────────────┐  │
│  │  Range Router   │ │ Argument Parse  │ │ Result Format │  │
│  │                 │ │                 │ │               │  │
│  │ • 56-83: File   │ │ • Type Safety   │ │ • Success Val │  │
│  │ • 214-239: Mem  │ │ • Validation    │ │ • Error Codes │  │
│  │ • 93-178: Proc  │ │ • Bounds Check  │ │ • Logging     │  │
│  │ • 900-999: OS   │ │ • NULL Check    │ │ • Debug Info  │  │
│  └─────────────────┘ └─────────────────┘ └───────────────┘  │
├─────────────────────────────────────────────────────────────┤
│                    Kernel Services                         │
│                                                             │
│  ┌─────────────────┐ ┌─────────────────┐ ┌───────────────┐  │
│  │ Filesystem      │ │ Memory Manager  │ │ Process Mgr   │  │
│  │                 │ │                 │ │               │  │
│  │ • VFS Layer     │ │ • Buddy Alloc   │ │ • PID Mgmt    │  │
│  │ • FAT32/ext4    │ │ • Slab Alloc    │ │ • Task Struct │  │
│  │ • File Cache    │ │ • Fallible Ops  │ │ • Scheduling  │  │
│  │ • Error Handle  │ │ • Memory Stats  │ │ • Signals     │  │
│  └─────────────────┘ └─────────────────┘ └───────────────┘  │
└─────────────────────────────────────────────────────────────┘
```

## Linux Compatibility

elinOS maintains compatibility with Linux system call numbers and semantics to ensure familiar developer experience:

### System Call Numbers

| Range | Category | Count | Purpose |
|-------|----------|-------|---------|
| **56-83** | File I/O | 28 | File operations, directory access |
| **93-178** | Process | 86 | Process management, signals |
| **214-239** | Memory | 26 | Memory management, mmap operations |
| **900-999** | elinOS | 100 | OS-specific extensions and debugging |

### Calling Convention

```rust
// Standard Linux calling convention
// riscv64: a7=syscall_num, a0-a5=args, return in a0
pub fn syscall_handler(
    syscall_num: usize,     // a7: System call number
    arg0: usize,            // a0: First argument
    arg1: usize,            // a1: Second argument  
    arg2: usize,            // a2: Third argument
    arg3: usize,            // a3: Fourth argument
) -> SysCallResult {
    // Dispatch to appropriate handler based on syscall_num
    match syscall_num {
        56..=83 => file::handle_file_syscall(&args),
        93..=178 | 220..=221 => process::handle_process_syscall(&args),
        214..=239 | 960.. => memory::handle_memory_syscall(&args),
        900..=999 => elinos::handle_elinos_syscall(&args),
        _ => SysCallResult::Error("Unknown system call"),
    }
}
```

## Implemented System Calls

### File I/O Operations (56-83)

#### Core File Operations

| Number | Name | Signature | Status | Description |
|--------|------|-----------|--------|-------------|
| **56** | `openat` | `(dirfd, path, flags, mode) -> fd` | ✅ Full | Open file relative to directory |
| **57** | `close` | `(fd) -> result` | ✅ Full | Close file descriptor |
| **63** | `read` | `(fd, buffer, count) -> bytes` | ✅ Full | Read from file descriptor |
| **64** | `write` | `(fd, buffer, count) -> bytes` | ✅ Full | Write to file descriptor |
| **61** | `getdents64` | `(fd, buffer, count) -> bytes` | ✅ Full | Get directory entries |

#### Implementation Details

```rust
// openat system call implementation
fn sys_openat(dirfd: i32, pathname: *const u8, flags: i32, mode: u32) -> SysCallResult {
    // Convert raw pointer to string
    let filename = unsafe { 
        parse_cstring_from_user(pathname, 256)? 
    };
    
    // Check file existence in filesystem
    if crate::filesystem::file_exists(&filename) {
        // Allocate new file descriptor
        let fd = allocate_fd(filename)?;
        SysCallResult::Success(fd as isize)
    } else {
        SysCallResult::Error("File not found")
    }
}

// read system call implementation  
fn sys_read(fd: i32, buffer: *mut u8, count: usize) -> SysCallResult {
    // Validate file descriptor
    let filename = get_filename_by_fd(fd)?;
    
    // Read file content from filesystem
    match crate::filesystem::read_file(&filename) {
        Ok(content) => {
            let bytes_to_copy = count.min(content.len());
            unsafe {
                copy_to_user(buffer, content.as_ptr(), bytes_to_copy)?;
            }
            SysCallResult::Success(bytes_to_copy as isize)
        }
        Err(_) => SysCallResult::Error("Read failed")
    }
}
```

### Memory Management (214-239, 960+)

#### Memory Operations

| Number | Name | Signature | Status | Description |
|--------|------|-----------|--------|-------------|
| **214** | `brk` | `(addr) -> new_addr` | ✅ Full | Change program break |
| **215** | `munmap` | `(addr, length) -> result` | ✅ Full | Unmap memory region |
| **222** | `mmap` | `(addr, len, prot, flags, fd, off) -> addr` | ✅ Full | Map memory region |
| **226** | `mprotect` | `(addr, len, prot) -> result` | ⚠️ Stub | Change memory protection |
| **233** | `madvise` | `(addr, len, advice) -> result` | ⚠️ Stub | Memory usage advice |

#### Memory-Specific Extensions

| Number | Name | Purpose | Implementation |
|--------|------|---------|----------------|
| **960** | `getmeminfo` | Get memory statistics | Returns allocator stats |
| **961** | `alloc_test` | Test allocation | Allocate/test/free memory |
| **962** | `buddy_stats` | Buddy allocator info | Detailed allocator metrics |

```rust
// mmap implementation with buddy allocator
fn sys_mmap(addr: usize, length: usize, prot: usize, flags: usize, 
           fd: usize, offset: usize) -> SysCallResult {
    console_println!("mmap: addr=0x{:x}, len={}, prot={}, flags={}", 
                    addr, length, prot, flags);
    
    // Handle anonymous mappings
    if flags & MAP_ANONYMOUS != 0 {
        if let Some(allocated_addr) = memory::allocate_memory(length) {
            console_println!("mmap allocated: 0x{:x}", allocated_addr);
            return SysCallResult::Success(allocated_addr as isize);
        } else {
            return SysCallResult::Error("Out of memory");
        }
    }
    
    SysCallResult::Error("File-backed mmap not implemented")
}

// Advanced memory information
fn sys_getmeminfo() -> SysCallResult {
    console_println!("=== Memory Management Information ===");
    
    let stats = memory::get_memory_stats();
    console_println!("Memory Manager: {}", stats.allocator_mode);
    console_println!("  Total Memory: {} MB", stats.detected_ram_size / (1024 * 1024));
    console_println!("  Allocated: {} bytes", stats.allocated_bytes);
    console_println!("  Allocations: {}", stats.allocation_count);
    console_println!("  Regions: {}", stats.regions_detected);
    
    SysCallResult::Success(0)
}
```

### Process Management (93-178, 220-221)

#### Process Operations

| Number | Name | Signature | Status | Description |
|--------|------|-----------|--------|-------------|
| **93** | `getpid` | `() -> pid` | ✅ Full | Get process ID |
| **94** | `getppid` | `() -> ppid` | ✅ Full | Get parent process ID |
| **60** | `exit` | `(status) -> !` | ✅ Full | Exit process |
| **220** | `clone` | `(flags, stack, ptid, ctid, regs) -> pid` | ⚠️ Stub | Create new process |
| **221** | `execve` | `(filename, argv, envp) -> result` | ⚠️ Stub | Execute program |

```rust
// Process ID management
static mut CURRENT_PID: u32 = 1;
static mut PARENT_PID: u32 = 0;

fn sys_getpid() -> SysCallResult {
    unsafe {
        SysCallResult::Success(CURRENT_PID as isize)
    }
}

fn sys_exit(status: i32) -> ! {
    console_println!("Process {} exiting with status {}", 
                    unsafe { CURRENT_PID }, status);
    // In a real kernel, this would clean up process resources
    loop {
        // Infinite loop for now (no process scheduling)
        unsafe { core::arch::asm!("wfi"); }
    }
}
```

## elinOS Extensions (900-999)

### System Information

| Number | Name | Purpose | Output |
|--------|------|---------|--------|
| **902** | `elinos_version` | Get kernel version | Version string and build info |
| **903** | `elinos_shutdown` | Shutdown system | Powers down via SBI |
| **904** | `elinos_reboot` | Reboot system | Restarts via SBI |

### Debugging & Diagnostics

| Number | Name | Purpose | Output |
|--------|------|---------|--------|
| **950** | `getdevices` | List system devices | VirtIO devices, memory regions |
| **951** | `debug_info` | Kernel debug info | Memory layout, allocator state |
| **952** | `trace_syscalls` | Toggle syscall tracing | Enable/disable call logging |

```rust
// elinOS-specific system calls
fn sys_elinos_version() -> SysCallResult {
    console_println!("=== elinOS Kernel Information ===");
    console_println!("Version: 0.1.0");
    console_println!("Architecture: RISC-V 64-bit");
    console_println!("Build: Experimental Kernel");
    console_println!("Features:");
    console_println!("  ✅ Multi-tier Memory Management");
    console_println!("  ✅ VirtIO Block Device Support");
    console_println!("  ✅ FAT32/ext4 Filesystem Support");
    console_println!("  ✅ Linux-compatible System Calls");
    console_println!("  ✅ Dynamic Hardware Detection");
    
    SysCallResult::Success(0)
}

fn sys_elinos_shutdown() -> ! {
    console_println!("🔌 Shutting down elinOS...");
    console_println!("💤 Goodbye!");
    
    // Use SBI to shutdown the system
    crate::sbi::shutdown();
}
```

## API Reference

### Core Types

```rust
/// System call result type
#[derive(Debug)]
pub enum SysCallResult {
    Success(isize),        // Return value (can be negative for error codes)
    Error(&'static str),   // Error message for debugging
}

/// System call arguments structure
#[derive(Debug)]
pub struct SyscallArgs {
    pub syscall_num: usize,
    pub arg0: usize,
    pub arg1: usize, 
    pub arg2: usize,
    pub arg3: usize,
    pub arg4: usize,
    pub arg5: usize,
}
```

### Error Handling

```rust
/// Convert filesystem errors to syscall results
impl From<FilesystemError> for SysCallResult {
    fn from(err: FilesystemError) -> Self {
        match err {
            FilesystemError::FileNotFound => SysCallResult::Success(-2), // ENOENT
            FilesystemError::IoError => SysCallResult::Success(-5),      // EIO
            FilesystemError::NotMounted => SysCallResult::Success(-19),  // ENODEV
            _ => SysCallResult::Success(-1),                             // EPERM
        }
    }
}

/// Standard Linux error codes
const EPERM: isize = -1;        // Operation not permitted
const ENOENT: isize = -2;       // No such file or directory
const EIO: isize = -5;          // I/O error
const ENOMEM: isize = -12;      // Out of memory
const EACCES: isize = -13;      // Permission denied
const EFAULT: isize = -14;      // Bad address
const ENODEV: isize = -19;      // No such device
```

### User Space Interface

```rust
// User space system call wrapper (future implementation)
pub fn syscall(num: usize, arg0: usize, arg1: usize, arg2: usize, arg3: usize) -> isize {
    unsafe {
        let result: isize;
        asm!(
            "ecall",
            in("a7") num,
            in("a0") arg0,
            in("a1") arg1,
            in("a2") arg2,
            in("a3") arg3,
            lateout("a0") result,
        );
        result
    }
}

// High-level wrappers
pub fn open(path: &str, flags: i32) -> Result<i32, i32> {
    let result = syscall(56, -100, path.as_ptr() as usize, flags as usize, 0);
    if result >= 0 {
        Ok(result as i32)
    } else {
        Err(result as i32)
    }
}

pub fn read(fd: i32, buffer: &mut [u8]) -> Result<usize, i32> {
    let result = syscall(63, fd as usize, buffer.as_mut_ptr() as usize, buffer.len(), 0);
    if result >= 0 {
        Ok(result as usize)
    } else {
        Err(result as i32)
    }
}
```

## Usage Examples

### File Operations

```rust
// Open and read a file
let fd = syscall(56, -100, "hello.txt".as_ptr() as usize, 0, 0); // openat
if fd >= 0 {
    let mut buffer = [0u8; 1024];
    let bytes_read = syscall(63, fd as usize, buffer.as_mut_ptr() as usize, 1024, 0); // read
    
    if bytes_read >= 0 {
        let content = &buffer[..bytes_read as usize];
        // Process file content
    }
    
    syscall(57, fd as usize, 0, 0, 0); // close
}
```

### Memory Management

```rust
// Allocate anonymous memory
let addr = syscall(222, 0, 4096, 3, 34, -1); // mmap: 4KB, RW, anonymous
if addr as isize > 0 {
    // Use memory at addr
    syscall(215, addr, 4096, 0, 0); // munmap
}

// Get memory statistics
syscall(960, 0, 0, 0, 0); // getmeminfo
```

### Process Management

```rust
// Get process information
let pid = syscall(93, 0, 0, 0, 0);     // getpid
let ppid = syscall(94, 0, 0, 0, 0);    // getppid

console_print!("Process {} (parent: {})", pid, ppid);

// Exit process
syscall(60, 0, 0, 0, 0); // exit with status 0
```

### System Information

```rust
// Get elinOS version
syscall(902, 0, 0, 0, 0); // elinos_version

// List devices
syscall(950, 0, 0, 0, 0); // getdevices

// Shutdown system
syscall(903, 0, 0, 0, 0); // elinos_shutdown
```

### Shell Integration

```bash
# These shell commands translate to system calls:

elinOS> ls                    # openat(-100, ".", O_RDONLY) + getdents64()
elinOS> cat hello.txt         # openat(-100, "hello.txt", O_RDONLY) + read() + close()
elinOS> memory                # getmeminfo()
elinOS> version               # elinos_version()
elinOS> shutdown              # elinos_shutdown()
```

---

*The elinOS system call interface provides a familiar Linux-compatible API while showcasing modern kernel design principles and serving as an excellent experimental resource for understanding operating system internals.* 