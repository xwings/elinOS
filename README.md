# elinOS

**A Modern RISC-V Experimental Operating System Kernel Written in Rust**

[![Build Status](https://img.shields.io/badge/build-passing-brightgreen)](https://github.com/username/elinOS)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue)](#license)
[![RISC-V](https://img.shields.io/badge/arch-RISC--V64-orange)](https://riscv.org/)
[![Rust](https://img.shields.io/badge/language-Rust-orange.svg)](https://www.rust-lang.org/)
[![no_std](https://img.shields.io/badge/no__std-yes-green)](https://docs.rust-embedded.org/book/intro/no-std.html)

> **elinOS** is an experimental operating system kernel designed for research, learning, and exploring advanced memory management techniques. Built entirely in Rust for RISC-V architecture, it features dynamic hardware detection, sophisticated multi-tier memory allocators, real filesystem implementations, and a comprehensive Linux-compatible system call interface.

## 🌟 Key Features

### 🧠 **Advanced Memory Management**
- **Multi-Tier Architecture**: Buddy allocator + Slab allocator + Fallible operations
- **Dynamic Hardware Detection**: Automatically detects available RAM and configures allocators
- **Memory Zones**: DMA, Normal, and High memory zone support with automatic detection
- **Adaptive Sizing**: Buffer sizes and allocator configurations scale based on detected memory
- **Sophisticated Allocation**: Handles everything from 8-byte objects to multi-megabyte allocations

### 💾 **Comprehensive Filesystem Support**
- **Multi-Filesystem**: Native FAT32 and ext2 implementations with automatic detection
- **Auto-Detection**: Probes boot sectors and superblocks to identify filesystem type
- **FAT32 Features**: Boot sector parsing, directory enumeration, cluster chain management, 8.3 filenames
- **ext2 Features**: Superblock validation, inode parsing, extent tree traversal, group descriptors
- **File Operations**: Create, read, write, delete files and directories
- **VirtIO Block Device**: Full VirtIO 1.0/1.1 support with auto-detection
- **Dynamic Buffering**: File buffers scale from 4KB to 1MB+ based on available memory

### ⚙️ **System Architecture**
- **RISC-V 64-bit**: Native support for RV64GC with supervisor mode and interrupt handling
- **Linux-Compatible System Calls**: 100+ system calls across 8 categories
- **Memory Safety**: Zero-cost abstractions with comprehensive error handling
- **SBI Integration**: Full SBI (Supervisor Binary Interface) support
- **Trap Handling**: Complete interrupt and exception handling system
- **Virtual Memory**: Software MMU implementation with memory protection

### 🖥️ **Interactive Shell Interface**
- **Built-in Commands**: 20+ shell commands for system interaction
- **File System Operations**: `ls`, `cat`, `touch`, `mkdir`, `rm`, `rmdir`, `cd`, `pwd`
- **System Monitoring**: `memory`, `devices`, `config`, `syscall`, `version`
- **Real-time Diagnostics**: Live system statistics and device information
- **Path Resolution**: Full path resolution with `.` and `..` support
- **Modular Design**: Separate shell crate for clean architecture

## 🚀 Quick Start

### Prerequisites

```bash
# Install Rust toolchain
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Add RISC-V target
rustup target add riscv64gc-unknown-none-elf

# Install QEMU (example for Ubuntu/Debian)
sudo apt install qemu-system-riscv64

# Install build tools
sudo apt install build-essential git
```

### Building & Running

```bash
# Clone the repository
git clone https://github.com/username/elinOS.git
cd elinOS

# Build the kernel
make build

# Run with QEMU
make run
```

### Creating Test Filesystems

```bash
# Create a FAT32 test disk with files
make fat32-disk

# Create an ext2 test disk with files  
make ext2-disk

# Populate disk with files  
make populate-disk

# The kernel will automatically detect and mount the filesystem
make run
```

## 💻 System Requirements

### Hardware Support
- **Architecture**: RISC-V 64-bit (RV64GC)
- **Memory**: 8MB minimum, 8GB+ maximum (auto-scaling)
- **Storage**: VirtIO block devices (legacy 1.0 and modern 1.1+)
- **Platform**: QEMU `virt` machine, SiFive boards, and compatible hardware

### Host Requirements
- **Rust**: Nightly toolchain with `riscv64gc-unknown-none-elf` target
- **QEMU**: 5.0+ with RISC-V system emulation
- **Build Tools**: GNU Make, GCC toolchain

## 🏗️ Architecture Overview

```
┌─────────────────────────────────────────────────────────────┐
│                        User Space                           │
│                    (Future Development)                     │
├─────────────────────────────────────────────────────────────┤
│                   System Call Interface                     │
│              (Linux-compatible: 100+ syscalls)              │
│                     8 Categories                            │
├─────────────────────────────────────────────────────────────┤
│                      elinOS Kernel                          │
│                                                             │
│  ┌─────────────────┐ ┌─────────────────┐ ┌───────────────┐  │
│  │ Memory Manager  │ │ Filesystem      │ │ Device Mgmt   │  │
│  │                 │ │                 │ │               │  │
│  │ • Buddy Alloc   │ │ • FAT32 + ext2  │ │ • VirtIO 1.1  │  │
│  │ • Slab Alloc    │ │ • Auto-detect   │ │ • Auto-detect │  │
│  │ • Fallible Ops  │ │ • File CRUD     │ │ • SBI Runtime │  │
│  │ • Auto-scaling  │ │ • Path resolve  │ │ • Trap Handle │  │
│  └─────────────────┘ └─────────────────┘ └───────────────┘  │
├─────────────────────────────────────────────────────────────┤
│                    Hardware Abstraction                     │
│              (RISC-V + SBI + VirtIO + MMU)                  │
└─────────────────────────────────────────────────────────────┘
```

## 🔧 Available Commands

### File System Operations
```bash
elinOS> ls [path]               # List files and directories
elinOS> cat <filename>          # Display file contents
elinOS> touch <filename>        # Create empty file
elinOS> mkdir <dirname>         # Create directory
elinOS> rm <filename>           # Remove file
elinOS> rmdir <dirname>         # Remove empty directory
elinOS> cd <path>               # Change directory
elinOS> pwd                     # Show current directory
```

### System Information
```bash
elinOS> help                    # Show all available commands
elinOS> version                 # Kernel version and features
elinOS> config                  # Display system configuration
elinOS> memory                  # Memory layout and allocator stats
elinOS> heap                    # Detailed heap information
elinOS> devices                 # List detected VirtIO devices
elinOS> syscall                 # Show system call information
elinOS> fscheck                 # Filesystem status and info
```

### System Control
```bash
elinOS> echo <message>          # Print message
elinOS> shutdown                # Graceful system shutdown
elinOS> reboot                  # System reboot
```


## 🔬 Development & Research

elinOS is designed for:

- **Memory Management Research**: Testing advanced allocation strategies
- **Filesystem Development**: Real filesystem implementation learning
- **OS Kernel Development**: Understanding kernel architecture concepts
- **RISC-V Development**: Exploring RISC-V architecture features
- **System Programming**: Learning low-level Rust programming

## 🤝 Contributing

We welcome contributions! Please see our [Contributing Guide](CONTRIBUTING.md) for details.

### Development Workflow
1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Make your changes
4. Test thoroughly (`make test`)
5. Commit with clear messages
6. Push to your branch
7. Open a Pull Request

### Code Standards
- Follow Rust best practices and idioms
- Maintain `#![no_std]` compatibility
- Document public APIs thoroughly
- Include tests for new functionality
- Ensure memory safety and performance

## 🛣️ Current Status & Roadmap

### ✅ Completed (v0.1.0)
- [x] Dynamic memory management with buddy + slab allocators
- [x] Hardware auto-detection and adaptive sizing
- [x] Complete FAT32 and ext2 filesystem implementations
- [x] 100+ Linux-compatible system calls
- [x] Interactive shell with 20+ commands
- [x] VirtIO block device support
- [x] Comprehensive trap and interrupt handling
- [x] Virtual memory management (software MMU)
- [x] basic ELF program loading and execution

### 🚧 In Progress (v0.2.0)
- [ ] Advanced ELF program loading and execution
- [ ] User-space process management
- [ ] Advanced memory protection (hardware MMU)
- [ ] Improved filesystem write operations
- [ ] Network stack implementation

### 🔮 Future Goals (v0.3.0+)
- [ ] SMP (multi-core) support
- [ ] Advanced scheduler with priority queues
- [ ] Device driver framework
- [ ] IPC mechanisms (pipes, shared memory)
- [ ] Security hardening and capability system
- [ ] Performance optimizations

## 📊 Technical Specifications

### Memory Management
- **Buddy Allocator**: Powers-of-2 block allocation for large objects
- **Slab Allocator**: Efficient small object allocation (8B to 4KB)
- **Fallible Operations**: Graceful handling of OOM conditions
- **Auto-scaling**: Memory configuration adapts to detected hardware

### Filesystem Support
- **FAT32**: Complete implementation with cluster chain management
- **ext2**: Full superblock, inode, and extent tree support
- **Unified Interface**: Common API for all filesystem types
- **Auto-detection**: Automatic filesystem type identification


## 🐛 Known Limitations

- **User Space**: No user processes yet (kernel-only)
- **Networking**: System calls defined but not implemented
- **SMP**: Single-core only
- **Hardware**: Limited to QEMU and compatible platforms
- **Debugging**: Basic debugging support

## 📜 License

This project is licensed under either of

- Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## 🙏 Acknowledgments

- **Rust Community**: For the excellent `no_std` ecosystem
- **RISC-V Foundation**: For the open, extensible architecture
- **QEMU Team**: For the versatile emulation platform
- **Linux Kernel**: For system call interface inspiration
- **Maestro OS**: For memory management architecture insights

---

**elinOS** - *Where Rust meets RISC-V in kernel space* 🦀⚡