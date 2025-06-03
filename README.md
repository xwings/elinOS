# elinOS

A RISC-V64 experimental operating system written in Rust, featuring modern system call architecture and VirtIO device support.

## Features

### System Architecture
- **RISC-V64 kernel** - Native 64-bit RISC-V implementation
- **Rust-based design** - Memory-safe kernel with zero-cost abstractions
- **Linux-compatible syscalls** - Standard system call interface for familiarity

### Storage & Filesystem
- **VirtIO block device** - Modern paravirtualized storage interface
- **FAT32 filesystem** - Full read support for FAT32 volumes
- **File operations** - List, read, and display file contents

### Memory Management
- **Dynamic memory layout** - Intelligent kernel size detection
- **Multi-tier allocation** - Efficient memory management system
- **Page-aligned operations** - Proper memory alignment and safety

### Device Support
- **VirtIO MMIO transport** - Legacy and modern VirtIO support
- **UART console** - Serial console for user interaction
- **QEMU integration** - Optimized for QEMU virtual machine

## Quick Start

### Prerequisites
- Rust nightly toolchain
- QEMU RISC-V system emulation
- Cross-compilation tools

### Building
```bash
cargo build --target riscv64gc-unknown-none-elf
```

### Running
```bash
qemu-system-riscv64 \
    -machine virt \
    -cpu rv64 \
    -smp 1 \
    -m 128M \
    -nographic \
    -bios /usr/share/qemu/opensbi-riscv64-generic-fw_dynamic.bin \
    -kernel target/riscv64gc-unknown-none-elf/debug/elinOS \
    -drive file=disk.raw,format=raw,if=none,id=virtio-disk \
    -device virtio-blk-device,drive=virtio-disk
```

## System Commands

### File Operations
- `ls` - List files in filesystem
- `cat <file>` - Display file contents
- `echo <message>` - Echo a message

### System Information
- `help` - Show available commands
- `version` - Show kernel version
- `memory` - Show memory information
- `devices` - List system devices
- `syscall` - Show system call information

### System Control
- `shutdown` - Shutdown the system
- `reboot` - Reboot the system

## System Call Interface

elinOS implements Linux-compatible system calls for experimental purposes:

### File I/O Operations
- `SYS_WRITE (64)` - Write to file descriptor
- `SYS_READ (63)` - Read from file descriptor
- `SYS_OPENAT (56)` - Open file (modern Linux openat)
- `SYS_CLOSE (57)` - Close file descriptor
- `SYS_GETDENTS64 (61)` - List directory entries

### System Information
- `SYS_GETMEMINFO (960)` - Memory information
- `SYS_GETDEVICES (950)` - Device information
- `SYS_ELINOS_VERSION (902)` - System version
- `SYS_ELINOS_SHUTDOWN (903)` - System shutdown
- `SYS_ELINOS_REBOOT (904)` - System reboot

## Architecture

### I/O Stack
```
User Commands → System Calls → Filesystem → VirtIO → QEMU
```

### Memory Layout
- Dynamic kernel size detection
- Intelligent heap allocation
- Page-aligned memory regions
- Safety guards between regions

### VirtIO Integration
- MMIO transport layer
- Legacy VirtIO 1.0 support
- Modern VirtIO 1.1+ compatibility
- Efficient descriptor chain management

## Experimental Goals

elinOS is designed for experimentation:
- Modern OS development patterns
- Rust systems programming
- VirtIO device drivers
- System call implementation
- Memory management strategies

## License

Licensed under MIT License - see LICENSE file for details.

## Contributing

Contributions welcome! Please read our contributing guidelines and submit pull requests for any improvements.