# elinOS

**A Modern RISC-V Experimental Kernel Written in Rust**

[![Build Status](https://img.shields.io/badge/build-passing-brightgreen)](https://github.com/username/elinOS)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue)](#license)
[![RISC-V](https://img.shields.io/badge/arch-RISC--V64-orange)](https://riscv.org/)
[![Rust](https://img.shields.io/badge/language-Rust-orange.svg)](https://www.rust-lang.org/)
[![no_std](https://img.shields.io/badge/no__std-yes-green)](https://docs.rust-embedded.org/book/intro/no-std.html)

> **elinOS** is an experimental operating system kernel designed for research, experiment, and exploring advanced memory management techniques. Built from the ground up in Rust for RISC-V architecture, it features dynamic hardware detection, multi-tier memory allocators and real filesystem implementations.

## 🌟 Key Features

### 🧠 **Advanced Memory Management**
- **Multi-Tier Architecture**: Buddy allocator + Slab allocator + Fallible operations
- **Memory Zones**: DMA, Normal, and High memory zone support with automatic detection
- **Performance**: Smaller footprint, faster small allocations, faster large allocations, less fragmentation

### 💾 **Filesystem Support**
- **Multi-Filesystem**: Native FAT32 and ext2 implementations with real parsing
- **Automatic Detection**: Probes boot sectors and superblocks to identify filesystem type
- **FAT32 Features**: Boot sector parsing, directory enumeration, cluster chain following, 8.3 filenames
- **ext2 Features**: Superblock validation, inode parsing, extent tree traversal, group descriptors
- **VirtIO Block Device**: Full VirtIO 1.0/1.1 support with auto-detection and queue management
- **Dynamic Buffer Sizing**: File buffers scale based on available memory (4KB → 1MB)

### 🔧 **System Architecture**
- **RISC-V 64-bit**: Native support for RV64GC with machine mode and interrupt handling
- **Linux-Compatible Syscalls**: 50+ system calls including file I/O, memory management, and process control
- **Rust Safety**: Memory-safe kernel with zero-cost abstractions and comprehensive error handling
- **SBI Integration**: Full SBI (Supervisor Binary Interface) support for hardware abstraction

### 🛠️ **Developer Experience**
- **Interactive Shell**: Built-in command-line interface with 15+ commands
- **Comprehensive Diagnostics**: Real-time system monitoring, memory statistics, and device information
- **Comprehensive Documentation**: Extensive technical documentation with architecture diagrams
- **Experiment Focus**: Clear code structure for learning OS development concepts

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
```

### Creating Test Filesystems

```bash
# Create a FAT32 test disk with files
make fat32-disk
make populate-disk

# Create an ext2 test disk with files
make ext2-disk
make populate-disk

# The kernel will automatically detect and mount the filesystem
make run
```

## 📖 Documentation

- **[English Documentation](docs/en/)**
- **[Memory Management](docs/en/memory.md)** - Advanced memory subsystem details
- **[Filesystem Support](docs/en/filesystem.md)** - Storage and filesystem implementation
- **[System Calls](docs/en/syscalls.md)** - API reference and Linux compatibility
- **[Building & Development](docs/en/development.md)** - Developer setup and workflow
- **[Commands](docs/en/commands.md)** - List of available shell commands
- **[Debugging](docs/en/debugging.md)** - Debugging tips and techniques
- **[Translation](docs/en/translation.md)** - Guidelines for translating documentation

## 🎯 System Requirements

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
│              (Linux-compatible: 50+ syscalls)               │
├─────────────────────────────────────────────────────────────┤
│                      elinOS Kernel                          │
│                                                             │
│  ┌─────────────────┐ ┌─────────────────┐ ┌───────────────┐  │
│  │ Memory Manager  │ │ Filesystem      │ │ Device Mgmt   │  │
│  │                 │ │                 │ │               │  │
│  │ • Buddy Alloc   │ │ • Real FAT32    │ │ • VirtIO 1.1  │  │
│  │ • Slab Alloc    │ │ • Real ext2     │ │ • Auto-detect │  │
│  │ • Fallible Ops  │ │ • Auto-detect   │ │ • SBI Runtime │  │
│  │ • Transactions  │ │ • Boot Sectors  │ │ • MMIO Queues │  │
│  └─────────────────┘ └─────────────────┘ └───────────────┘  │
├─────────────────────────────────────────────────────────────┤
│                    Hardware Abstraction                     │
│              (RISC-V + SBI + VirtIO)                        │
└─────────────────────────────────────────────────────────────┘
```

## 🧪 Available Commands

```bash
elinOS> help                    # Show all available commands
elinOS> config                  # Display dynamic system configuration
elinOS> memory                  # Show memory layout and allocator statistics
elinOS> devices                 # List detected VirtIO devices
elinOS> ls                      # List files (auto-detects FAT32/ext2)
elinOS> cat filename.txt        # Read file contents from filesystem
elinOS> touch filename.txt      # create empty file
elinOS> rm filename.txt         # remove empty
elinOS> cd dirname              # change dir path
elinOS> mkdir dirname           # create dir
elinOS> rmdir dirname           # remove dir
elinOS> filesystem              # Show filesystem type and mount status
elinOS> syscall                 # Show system call information
elinOS> version                 # Kernel version and features
elinOS> shutdown                # Graceful system shutdown via SBI
elinOS> reboot                  # System reboot via SBI
```

## 🔬 Research Applications

elinOS is designed for:

- **Memory Management Research**: Testing advanced allocation strategies and fallible operations
- **Filesystem Development**: Implementing and testing new filesystem types
- **OS experiment**: Learning kernel development concepts with real implementations
- **Hardware Bring-up**: Porting to new RISC-V platforms and devices

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

## 🛣️ Roadmap

### Current Focus (v0.2.0)
- [ ] SMP (multi-core) support with per-CPU allocators
- [ ] Network stack implementation with VirtIO-net
- [ ] Advanced scheduler with priority queues
- [ ] Memory protection (MMU/paging) with virtual memory

### Future Goals (v0.3.0+)
- [ ] Device driver framework with hot-plug support
- [ ] User-space processes with ELF loading
- [ ] IPC mechanisms (pipes, shared memory)
- [ ] Security hardening and capability system

### Filesystem Enhancements
- [ ] File caching and buffer management


## 📄 License

This project is dual-licensed under:

- **MIT License** ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

---

**elinOS** - *Where hardware meets software, safely and efficiently* 🦀✨