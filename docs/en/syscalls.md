# elinOS System Call Reference

## Overview

elinOS implements a system call interface, drawing inspiration from Linux for compatibility where appropriate, but also including OS-specific calls. This document details the available system calls, their numbers, and their current implementation status.

System calls are dispatched through a central `syscall_handler` in `src/syscall/mod.rs`. The numbers and categories listed here reflect what is actively routed and handled.

## System Call Categories and Dispatch

Syscalls in elinOS are broadly categorized. The main dispatcher routes syscall numbers to handlers for these categories:

*   **File I/O Operations**: Handled by `syscall::file`
*   **Directory Operations**: Handled by `syscall::directory`
*   **Device and I/O Management**: Handled by `syscall::device`
*   **Process Management**: Handled by `syscall::process`
*   **Time and Timer Operations**: Handled by `syscall::time`
*   **System Information**: Handled by `syscall::sysinfo`
*   **Network Operations**: Handled by `syscall::network`
*   **Memory Management**: Handled by `syscall::memory`
*   **elinOS-Specific Operations**: Handled by `syscall::elinos`

**Note on Syscall Numbers:** While many syscall constants are defined across different modules (e.g., `SYS_OPENAT` in `file.rs`), the actual syscall number recognized by the kernel is determined by the main dispatcher in `src/syscall/mod.rs` and the explicit `match` arms within each category handler. Discrepancies in numbering or unhandled constants within modules are areas for ongoing development.

## Implemented System Calls

### File I/O Operations
(Handler: `syscall::file::handle_file_syscall`)

| Number | Name (Constant)    | Description                     | Status      |
|--------|--------------------|---------------------------------|-------------|
| 35     | `SYS_UNLINK`       | Unlink/delete a file (unlinkat) | Stub        |
| 45     | `SYS_TRUNCATE`     | Truncate a file by path         | Stub        |
| 46     | `SYS_FTRUNCATE`    | Truncate a file by fd           | Stub        |
| 56     | `SYS_OPENAT`       | Open file                       | Implemented |
| 57     | `SYS_CLOSE`        | Close file descriptor           | Implemented |
| 61     | `SYS_GETDENTS64`   | Get directory entries           | Implemented |
| 62     | `SYS_LSEEK`        | Reposition file offset          | Stub        |
| 63     | `SYS_READ`         | Read from file descriptor       | Implemented |
| 64     | `SYS_WRITE`        | Write to file descriptor (stdout/stderr, file write stubbed) | Implemented |
| 79     | `SYS_NEWFSTATAT`   | Get file status (stat)          | Stub        |
| 81     | `SYS_SYNC`         | Synchronize filesystem          | Stub        |
| 82     | `SYS_FSYNC`        | Synchronize file data by fd     | Stub        |

*(Other constants like `SYS_READV`, `SYS_WRITEV`, `SYS_FSTAT` etc. are defined in `file.rs` but not currently explicitly handled or routed by these numbers in the primary dispatcher).*

### Directory Operations
(Handler: `syscall::directory::handle_directory_syscall`)

*Note: The main dispatcher routes numbers 34 (intended for `mkdirat`) and 49-55 to this handler. However, the handler in `directory.rs` currently only has explicit matches for its own constants 51-54.*

| Number | Name (Constant) | Description                     | Status      |
|--------|-----------------|---------------------------------|-------------|
| 51     | `SYS_MKDIR`     | Create directory                | Stub        |
| 52     | `SYS_RMDIR`     | Remove directory                | Stub        |
| 53     | `SYS_CHDIR`     | Change current directory        | Stub        |
| 54     | `SYS_GETCWD`    | Get current working directory   | Stub        |

### Device and I/O Management
(Handler: `syscall::device::handle_device_syscall`)

| Number | Name (Constant)         | Description                     | Status      |
|--------|-------------------------|---------------------------------|-------------|
| 23     | `SYS_DUP`               | Duplicate file descriptor       | Stub        |
| 24     | `SYS_DUP3`              | Duplicate file descriptor       | Stub        |
| 25     | `SYS_FCNTL`             | File control                    | Stub        |
| 26     | `SYS_INOTIFY_INIT1`     | Initialize inotify instance     | Stub        |
| 27     | `SYS_INOTIFY_ADD_WATCH` | Add watch to inotify instance   | Stub        |
| 28     | `SYS_INOTIFY_RM_WATCH`  | Remove watch from inotify       | Stub        |
| 29     | `SYS_IOCTL`             | I/O control                     | Stub        |
| 30     | `SYS_IOPRIO_SET`        | Set I/O priority                | Stub        |
| 31     | `SYS_IOPRIO_GET`        | Get I/O priority                | Stub        |
| 32     | `SYS_FLOCK`             | Apply or remove an advisory lock| Stub        |
| 33     | `SYS_MKNODAT`           | Create device special file      | Stub        |
| 59     | `SYS_PIPE2`             | Create pipe                     | Stub        |
| 950    | `SYS_GETDEVICES`        | Get device info (elinOS Specific) | Implemented |

### Process Management
(Handler: `syscall::process::handle_process_syscall`)

| Number | Name (Constant)       | Description                   | Status      |
|--------|-----------------------|-------------------------------|-------------|
| 93     | `SYS_EXIT`            | Terminate current process     | Implemented |
| 94     | `SYS_EXIT_GROUP`      | Terminate all threads in group| Implemented |
| 95     | `SYS_WAITID`          | Wait for child process change | Stub        |
| 129    | `SYS_KILL`            | Send signal to a process      | Stub        |
| 130    | `SYS_TKILL`           | Send signal to a thread       | Stub        |
| 131    | `SYS_TGKILL`          | Send signal to thread in group| Stub        |
| 134    | `SYS_RT_SIGACTION`    | Examine/change signal action  | Stub        |
| 172    | `SYS_GETPID`          | Get process ID                | Implemented |
| 173    | `SYS_GETPPID`         | Get parent process ID         | Implemented |
| 174    | `SYS_GETUID`          | Get real user ID              | Implemented |
| 176    | `SYS_GETGID`          | Get real group ID             | Implemented |
| 178    | `SYS_GETTID`          | Get thread ID                 | Implemented |
| 220    | `SYS_CLONE`           | Create child process (fork)   | Stub        |
| 221    | `SYS_EXECVE`          | Execute program               | Stub        |

*(Many other process-related constants like `SYS_SET_TID_ADDRESS`, `SYS_UNSHARE`, `SYS_FUTEX`, various signal calls, `SYS_REBOOT`(142), UID/GID setting calls are defined in `process.rs` but not explicitly handled by these numbers in the current `handle_process_syscall` or are superseded by elinOS specific versions).*

### Time and Timer Operations
(Handler: `syscall::time::handle_time_syscall`)

*Note: The main dispatcher routes 101-115 to this handler. The handler in `time.rs` is a generic stub and does not explicitly match these numbers. Constants like `SYS_NANOSLEEP` (101), `SYS_GETITIMER` (102), `SYS_SETITIMER` (103) are defined in `process.rs` (and intended for this range based on Linux conventions). Constants like `SYS_TIME` (271), `SYS_GETTIMEOFDAY` (272) defined in `time.rs` are not currently routed by the main dispatcher to this handler via these numbers.*

| Number Range | Description                                     | Status      |
|--------------|-------------------------------------------------|-------------|
| 101-115      | Intended for time/timer ops (e.g., nanosleep, get/setitimer) | Stub        |

### System Information
(Handler: `syscall::sysinfo::handle_sysinfo_syscall`)

*Note: The main dispatcher routes 160-168, 169-171, 179 to this handler. The handler in `sysinfo.rs` is a generic stub. Constants like `SYS_UNAME` (301), `SYS_SYSINFO` (302) defined in `sysinfo.rs` are not currently routed by these numbers to this handler.*

| Number Range        | Description                                 | Status      |
|---------------------|---------------------------------------------|-------------|
| 160-168, 169-171, 179 | Intended for sysinfo ops (e.g., uname, sysinfo) | Stub        |

### Network Operations
(Handler: `syscall::network::handle_network_syscall`)

*Note: The main dispatcher routes 198-213 to this handler. The handler in `network.rs` is a generic stub. Constants like `SYS_SOCKET` (221), `SYS_BIND` (222) defined in `network.rs` are not currently routed by these numbers to this handler.*

| Number Range | Description                                   | Status      |
|--------------|-----------------------------------------------|-------------|
| 198-213      | Intended for network ops (e.g., socket, bind) | Stub        |


### Memory Management
(Handler: `syscall::memory::handle_memory_syscall`)

| Number | Name (Constant)     | Description                       | Status      |
|--------|---------------------|-----------------------------------|-------------|
| 214    | `SYS_BRK`           | Change data segment size          | Implemented |
| 215    | `SYS_MUNMAP`        | Unmap files or devices into memory| Implemented |
| 216    | `SYS_MREMAP`        | Remap a virtual memory address    | Stub        |
| 222    | `SYS_MMAP`          | Map files or devices into memory  | Implemented (Anonymous only) |
| 226    | `SYS_MPROTECT`      | Set protection on a region of memory | Stub        |
| 227    | `SYS_MSYNC`         | Synchronize a file with a memory map | Stub        |
| 228    | `SYS_MLOCK`         | Lock memory                       | Stub        |
| 229    | `SYS_MUNLOCK`       | Unlock memory                     | Stub        |
| 230    | `SYS_MLOCKALL`      | Lock all memory mapped by process | Stub        |
| 231    | `SYS_MUNLOCKALL`    | Unlock all memory mapped by process| Stub        |
| 232    | `SYS_MINCORE`       | Determine memory residency of pages| Stub        |
| 233    | `SYS_MADVISE`       | Give advice about use of memory   | Stub        |
| 960    | `SYS_GETMEMINFO`    | Get memory info (elinOS Specific) | Implemented |

*(Other constants like `SYS_ADD_KEY`, `SYS_SWAPON`, etc. are defined in `memory.rs` but not currently explicitly handled. `SYS_ALLOC_TEST` (961) and `SYS_BUDDY_STATS` (962) are defined and handled in `memory.rs` but not routed there by the main dispatcher; they would fall into the elinOS-Specific range 900-999, whose handler does not call them).*


### elinOS-Specific Operations
(Handler: `syscall::elinos::handle_elinos_syscall`)

| Number | Name (Constant)         | Description                        | Status      |
|--------|-------------------------|------------------------------------|-------------|
| 900    | `SYS_ELINOS_DEBUG`      | Print a debug message to console   | Implemented |
| 901    | `SYS_ELINOS_STATS`      | (No specific handler from main dispatcher to an implementation) | Definition Only |
| 902    | `SYS_ELINOS_VERSION`    | Display elinOS version information | Implemented |
| 903    | `SYS_ELINOS_SHUTDOWN`   | Shutdown the system via SBI        | Implemented |
| 904    | `SYS_ELINOS_REBOOT`     | Reboot the system via SBI          | Implemented |
| 905    | `SYS_LOAD_ELF`          | Load an ELF binary (elinOS specific) | Implemented |
| 906    | `SYS_EXEC_ELF`          | "Execute" loaded ELF (elinOS specific) | Implemented (Simulated) |
| 907    | `SYS_ELF_INFO`          | Get ELF binary info (elinOS specific)| Implemented |


---

*The elinOS system call interface provides a familiar Linux-compatible API while showcasing modern kernel design principles and serving as an excellent experimental resource for understanding operating system internals.* 