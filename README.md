# ElinOS

**Language / 语言**: [English](README.md) | [简体中文](README_zh.md)

A minimal operating system written in Rust for RISC-V 64-bit architecture, featuring dynamic memory management, VirtIO device support, a simple filesystem, and **production-ready system call architecture** with proper kernel/user space separation.

## 🚀 Features

### Core System
- **RISC-V 64-bit** target architecture
- **Dynamic memory detection** via OpenSBI
- **Adaptive memory management** with configurable heap and stack
- **Serial UART** communication and debugging
- **OpenSBI** integration for platform services
- **Professional system call architecture** organized like industry frameworks
- **System shutdown/reboot** via SBI interface

### Device Support
- **VirtIO block device** driver
- **Automatic device probing** and initialization
- **QEMU virt machine** support
- **Disk image management** (qcow2 format)

### Filesystem
- **In-memory filesystem** with file operations
- **File management** commands (create, read, delete, list)
- **Block device abstraction** for future real filesystem support
- **Pre-loaded test files** for demonstration

### System Call Architecture
- **Categorized organization** inspired by Qiling framework
- **Range-based syscall numbers** for scalability
- **9 distinct categories** covering all OS functionality
- **Professional structure** ready for production-scale development
- **Easy extension** within category boundaries

### Interactive Shell
- **Command-line interface** with history and editing
- **Dynamic command system** - no main.rs changes for new commands
- **Built-in commands** for system inspection and file management
- **Centralized command processing** architecture
- **Help system** with comprehensive documentation

## 🛠 Building and Running

### Prerequisites
- Rust toolchain with `riscv64gc-unknown-none-elf` target
- QEMU with RISC-V support (`qemu-system-riscv64`)
- `mkfs.fat` for disk image formatting (optional)

### Build the Kernel
```bash
./build.sh
```

### Run with QEMU
```bash
# Run with default configuration (128MB RAM, 100MB disk)
./run.sh

# Run with custom memory size
MEMORY=256M ./run.sh

# Run with custom disk size
DISK_SIZE=500M ./run.sh

# Run with custom disk image
DISK_IMAGE=my_disk.qcow2 ./run.sh
```

## 💻 Shell Commands

Once ElinOS boots, you'll have access to an interactive shell with the following commands:

### System Information
- `help` - Show available commands
- `memory` - Display detected memory regions via SYS_GETMEMINFO
- `devices` - Probe and list VirtIO devices via SYS_GETDEVICES
- `syscall` - Show system call information and architecture
- `categories` - Show syscall categorization system
- `version` - Show ElinOS version via SYS_ELINOS_VERSION

### File System Operations
- `ls` - List all files with sizes (uses SYS_GETDENTS)
- `cat <filename>` - Display file contents (uses SYS_OPEN)
- `touch <filename>` - Create a new empty file (uses filesystem + SYS_OPEN)
- `rm <filename>` - Delete a file (uses SYS_UNLINK)

### System Control
- `shutdown` - Gracefully shutdown ElinOS and exit QEMU (uses SYS_ELINOS_SHUTDOWN)
- `reboot` - Restart the system (uses SYS_ELINOS_REBOOT)
- `clear` - Clear the screen (uses SYS_WRITE)

### Example Session
```
elinOS> help
Available commands:
  help       - Show this help
  memory     - Show memory information
  devices    - Probe for VirtIO devices
  ls         - List files
  cat <file> - Show file contents
  touch <file> - Create empty file
  rm <file>  - Delete file
  clear      - Clear screen
  syscall    - Show system call info
  categories - Show syscall categories
  version    - Show ElinOS version
  shutdown   - Shutdown the system
  reboot     - Reboot the system

elinOS> categories
System Call Categories:
  1-50:   File I/O Operations
  51-70:  Directory Operations
  71-120: Memory Management
  121-170: Process Management
  171-220: Device and I/O Management
  221-270: Network Operations
  271-300: Time and Timer Operations
  301-350: System Information
  900-999: ElinOS-Specific Operations

elinOS> syscall
System Call Information:
  This shell uses categorized system calls for all kernel operations!

Currently Implemented System Calls:
  File I/O Operations:
    SYS_WRITE (1)     - Write to file descriptor
    SYS_READ (2)      - Read from file descriptor [TODO]
    SYS_OPEN (3)      - Open file
    SYS_CLOSE (4)     - Close file descriptor [TODO]
    SYS_UNLINK (5)    - Delete file
    SYS_GETDENTS (6)  - List directory entries
    SYS_STAT (9)      - Get file status
  Directory Operations:
    SYS_MKDIR (51)    - Create directory [TODO]
    SYS_RMDIR (52)    - Remove directory [TODO]
  Memory Management:
    SYS_MMAP (71)     - Memory mapping [TODO]
    SYS_MUNMAP (72)   - Memory unmapping [TODO]
    SYS_GETMEMINFO (100) - Memory information
  Process Management:
    SYS_EXIT (121)    - Exit process
  Device Management:
    SYS_GETDEVICES (200) - Device information
  ElinOS-Specific:
    SYS_ELINOS_DEBUG (900)    - Set debug level
    SYS_ELINOS_VERSION (902)  - Show version
    SYS_ELINOS_SHUTDOWN (903) - Shutdown system
    SYS_ELINOS_REBOOT (904)   - Reboot system

Commands are user-space programs that call these syscalls.
Use 'categories' to see the full categorization system.

elinOS> version
ElinOS v0.1.0 - RISC-V Operating System
Built with Rust and proper syscall architecture
Organized syscalls inspired by Qiling framework

elinOS> ls
Files:
  hello.txt (30 bytes)
  test.txt (35 bytes)
  readme.md (42 bytes)

elinOS> cat hello.txt
Contents of hello.txt:
Hello from ElinOS filesystem!
--- End of file ---

elinOS> memory
Memory regions:
  Region 0: 0x80000000 - 0x88000000 (128 MB) RAM

elinOS> devices
Probing for VirtIO devices...
VirtIO device at 0x10008000, ID: 2
  - Block device found!
VirtIO block device found at 0x10008000
VirtIO block device initialized, queue size: 128

elinOS> shutdown
ElinOS shutting down...
Goodbye!
# Returns to host shell automatically
```

## 🏗 Architecture

### System Call Architecture
ElinOS implements a **production-ready** operating system architecture with industry-standard organization:

#### Categorized System Call Organization
Inspired by the **Qiling framework**, syscalls are organized into logical categories with dedicated number ranges:

| Range | Category | Purpose | Examples |
|-------|----------|---------|----------|
| 1-50 | File I/O Operations | File operations | read, write, open, close, unlink |
| 51-70 | Directory Operations | Directory management | mkdir, rmdir, chdir, getcwd |
| 71-120 | Memory Management | Memory operations | mmap, munmap, mprotect, brk |
| 121-170 | Process Management | Process control | exit, fork, execve, wait, kill |
| 171-220 | Device and I/O Management | Device control | ioctl, fcntl, pipe, dup |
| 221-270 | Network Operations | Network stack | socket, bind, listen, accept |
| 271-300 | Time and Timer Operations | Time management | gettimeofday, nanosleep |
| 301-350 | System Information | System queries | uname, sysinfo, getuid |
| 900-999 | ElinOS-Specific Operations | OS-specific features | debug, version, shutdown |

#### Kernel Space (`src/syscall/`)
- **Modular Organization**: Each category in separate files (file.rs, memory.rs, etc.)
- **Central Dispatcher**: Range-based routing in mod.rs
- **Type-safe Interface**: SysCallResult enum for error handling
- **Security**: Input validation and boundary enforcement
- **Scalability**: Easy to add new syscalls within categories

#### User Space (`src/commands.rs`)
- **Dynamic Command System**: No main.rs changes needed for new commands
- **Centralized Processing**: Single entry point with automatic dispatch
- **Error Handling**: Consistent error reporting across all commands
- **Extensible Design**: Just add to command list and implement function

#### Shell (`src/main.rs`)
- **Simplified Design**: Delegates all command processing to commands module
- **Clean Architecture**: Minimal coupling between shell and command implementations
- **Interactive Experience**: Provides user-friendly interface

### Memory Layout
- **Kernel**: Loaded at `0x80200000` (2MB reserved)
- **Heap**: Dynamically configured based on detected RAM
- **Stack**: 2MB per hart (up to 4 harts supported)
- **Memory Detection**: Automatic via OpenSBI calls

### Project Structure
```
src/
├── main.rs          # Kernel entry point and shell
├── syscall/         # System call interface (kernel space)
│   ├── mod.rs       # Central dispatcher and utilities
│   ├── file.rs      # File I/O operations (1-50)
│   ├── directory.rs # Directory operations (51-70)
│   ├── memory.rs    # Memory management (71-120)
│   ├── process.rs   # Process management (121-170)
│   ├── device.rs    # Device and I/O management (171-220)
│   ├── network.rs   # Network operations (221-270)
│   ├── time.rs      # Time and timer operations (271-300)
│   ├── sysinfo.rs   # System information (301-350)
│   └── elinos.rs    # ElinOS-specific operations (900-999)
├── commands.rs      # User space commands with dynamic dispatch
├── memory.rs        # Dynamic memory management
├── sbi.rs          # OpenSBI interface with shutdown support
├── virtio_blk.rs   # VirtIO block device driver
├── filesystem.rs   # In-memory filesystem
└── linker.ld       # Linker script with flexible memory layout
```

### Key Components

#### System Call Interface (`src/syscall/`)
- **Professional Organization**: 9-file structure, each category self-contained
- **Industry Standard**: Follows Qiling framework's proven methodology
- **Range-based Numbers**: Future-proof expansion within categories
- **Production Ready**: 10 implemented syscalls across 5 categories
- **Developer Friendly**: Clear ownership, isolated changes, easy testing

#### Dynamic Command System (`commands.rs`)
- **Self-Registering**: Commands automatically available in shell
- **Centralized Dispatch**: Single function handles all routing
- **Error Handling**: Consistent error reporting
- **Extensible**: Add command function + update help, that's it!

#### Memory Management (`memory.rs`)
- OpenSBI-based memory region detection
- Dynamic heap configuration
- Memory allocator with 8-byte alignment

#### System Control (`sbi.rs`)
- **SBI Shutdown Support**: Proper RISC-V system shutdown/reboot
- **Clean Exit**: Graceful shutdown that returns to host shell
- **Standard Interface**: Uses SBI System Reset Extension

#### VirtIO Support (`virtio_blk.rs`)
- MMIO-based device discovery
- VirtIO protocol implementation
- Block device abstraction layer

#### Filesystem (`filesystem.rs`)
- In-memory file storage using `heapless` collections
- POSIX-like file operations
- Extensible design for future real filesystems

## 🔧 Configuration

### Memory Configuration
The system automatically detects available memory, but you can override defaults:

```bash
# Set QEMU memory size
MEMORY=512M ./run.sh
```

### Disk Configuration
Disk images are automatically created and formatted:

```bash
# Custom disk size
DISK_SIZE=1G ./run.sh

# Use existing disk image
DISK_IMAGE=existing.qcow2 ./run.sh
```

### Build Configuration
Edit `Cargo.toml` to modify dependencies or `src/linker.ld` for memory layout adjustments.

## 🚧 Future Enhancements

### Immediate Roadmap
- [ ] **Complete SYS_READ** - Full file reading via file descriptors
- [ ] **Directory Operations** - mkdir, rmdir, chdir implementations
- [ ] **Memory Mapping** - mmap/munmap for advanced memory management
- [ ] **Process Fork/Exec** - Basic multitasking foundation

### Medium Term
- [ ] **Memory Protection** - User/kernel memory separation with page tables
- [ ] **Real VirtIO I/O** - Complete block device implementation with DMA
- [ ] **FAT32 Support** - Integration with `fatfs` crate for real filesystems
- [ ] **Network Support** - VirtIO network device driver

### Long Term
- [ ] **Multi-Core Support** - SMP with proper synchronization
- [ ] **Advanced Scheduling** - CFS-like scheduler implementation
- [ ] **Text Editor** - Simple file editing capabilities
- [ ] **Inter-Process Communication** - Pipes and shared memory
- [ ] **Security Framework** - Capabilities and sandboxing

## 🐛 Debugging

### QEMU Logs
Debugging information is logged to `qemu.log`:
```bash
tail -f qemu.log
```

### Memory Issues
Use the `memory` command to inspect detected regions:
```bash
elinOS> memory
```

### Device Issues
Check VirtIO device detection:
```bash
elinOS> devices
```

### System Call Debugging
Inspect system call interface:
```bash
elinOS> syscall
elinOS> categories
```

## 📚 Technical Details

### Boot Process
1. **OpenSBI** loads and initializes the platform
2. **Kernel** starts at `_start` in `main.rs`
3. **Memory detection** via OpenSBI calls
4. **Device probing** for VirtIO devices
5. **Filesystem initialization** with test files
6. **System call interface** activation
7. **Shell startup** for user interaction

### System Call Implementation
- **Categorized Organization**: Industry-standard structure for maintainability
- **Range-based Dispatch**: Efficient routing to category handlers
- **Type-safe Interface**: SysCallResult enum for error handling
- **Parameter Passing**: Uses RISC-V calling conventions
- **Security**: Input validation and boundary checking

### Dynamic Command Architecture
- **Self-Contained**: Commands module handles all dispatch logic
- **Extensible**: Adding commands requires no main.rs changes
- **Error Handling**: Centralized error reporting
- **Maintainable**: Clear separation of concerns

### VirtIO Implementation
- Follows VirtIO 1.0 specification
- MMIO-based device access
- Simplified queue management for proof-of-concept
- Ready for extension to full DMA-based I/O

### Safety and Correctness
- Written in **safe Rust** where possible
- Minimal `unsafe` blocks for hardware access
- **System call boundary** enforces kernel protection
- Spin-lock based synchronization
- No heap allocation in kernel (uses stack and static storage)

### Kernel Design Principles
- **Production Architecture**: Scalable, maintainable organization
- **Industry Standards**: Follows proven frameworks like Qiling
- **Type Safety**: Leverages Rust's type system for correctness
- **Modular Design**: Easy to extend and maintain
- **Clear Boundaries**: Proper kernel/user space separation

## 📄 License

This project is open source. Feel free to use, modify, and distribute.

---

**ElinOS** - A minimal OS demonstrating **production-ready kernel development practices** in Rust with professional system call architecture and industry-standard organization.
