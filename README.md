# elinKernel

**Language / 语言**: [English](README.md) | [简体中文](README_zh.md)

An **experimental** RISC-V kernel written in Rust, featuring dynamic memory management, well-organized system call architecture, **embedded ext4 filesystem**, and **ELF program execution capability**. Perfect for **learning**, **research**, and **educational purposes**.

## 🚀 Quick Start

```bash
# Build and run elinKernel
make
./run.sh
```

**What you'll see:**
- Interactive shell with comprehensive commands
- Professional system call architecture (9 categories, 15+ syscalls)
- **Embedded ext4 filesystem** with realistic superblock data
- **Simplified device management** (no complex VirtIO)
- ELF binary loading and analysis
- System information and memory management

## ✨ Key Features

### 🏗 **Clean Architecture**
- **Range-based syscall organization** (1-50: File I/O, 51-70: Directory, etc.)
- **Well-structured design** with 9 distinct categories
- **Type-safe implementation** leveraging Rust's safety features
- **Educational simplicity** - focus on core concepts, not device complexity

### 💾 **Memory Management**
- **Dynamic memory detection** via OpenSBI
- **Adaptive heap configuration** based on available RAM
- **Per-hart stack allocation** (supports up to 4 RISC-V cores)
- **Memory-safe operations** with bounds checking

### 🔧 **Simplified Device Management**
- **Direct UART communication** for debugging and I/O
- **Clean abstractions** without complex device driver overhead
- **Educational focus** on filesystem and memory management
- **Embedded approach** - perfect for learning core OS concepts

### 📁 **Embedded ext4 Filesystem**
- **Realistic ext4 superblock** with proper magic numbers and metadata
- **In-memory filesystem** with POSIX-like operations
- **File management commands** (ls, cat, touch, rm)
- **Educational ext4 implementation** demonstrating filesystem concepts
- **No complex block device drivers** - focus on filesystem logic

### 🔄 **ELF Program Support**
- **Complete ELF64 loader** with validation and parsing
- **Program header analysis** and segment information
- **Memory-safe ELF processing** using Rust type system
- **Ready for execution** (foundation for future virtual memory)

## 📖 Documentation

| Guide | Description |
|-------|-------------|
| [🚀 Getting Started](docs/en/getting-started.md) | Installation, compilation, and QEMU setup |
| [💻 Shell Commands](docs/en/commands.md) | Complete command reference and examples |
| [🏗 Architecture](docs/en/architecture.md) | Technical deep-dive into system design |
| [👨‍💻 Development](docs/en/development.md) | Creating user programs and C development |
| [🐛 Debugging](docs/en/debugging.md) | Troubleshooting and debugging techniques |
| [🗺 Roadmap](docs/en/roadmap.md) | Future development plans and phases |

## 🖥 Interactive Demo

Once elinKernel boots, explore its capabilities:

```bash
elinKernel> help                    # Show all available commands
elinKernel> version                 # Display elinKernel version info
elinKernel> memory                  # View memory layout
elinKernel> ext4check               # Check embedded ext4 filesystem
elinKernel> disktest                # Test filesystem operations
elinKernel> syscall                 # Show system call architecture
elinKernel> ls                      # List filesystem contents
elinKernel> cat hello.txt           # Display file contents
elinKernel> elf-info hello.elf      # Analyze ELF binary structure
elinKernel> elf-load hello.elf      # Load ELF into memory
elinKernel> shutdown                # Graceful system shutdown
```

## 🎯 Current Capabilities

- ✅ **Complete boot process** from OpenSBI to interactive shell
- ✅ **Professional syscall system** with 9 categories covering all OS functionality
- ✅ **Dynamic command dispatch** - easy to add new commands
- ✅ **Memory management** with automatic configuration
- ✅ **Embedded ext4 filesystem** with realistic superblock and metadata
- ✅ **File operations** create, read, delete, list files
- ✅ **ELF binary loading** parse and load RISC-V executables
- ✅ **System information** commands for debugging and monitoring
- ✅ **Clean shutdown/reboot** via OpenSBI interface
- ✅ **Educational simplicity** - focus on core OS concepts without device complexity

## 🚧 Coming Next

### Phase 1: Foundation (Short Term)
- **Complete syscall implementation** (SYS_READ, directory operations)
- **Enhanced memory management** (mmap, memory protection)
- **Extended filesystem commands** (mkdir, file permissions)

### Phase 2: Execution (Medium Term)
- **Virtual memory management** (RISC-V Sv39 page tables)
- **Process management** (fork, exec, scheduling)
- **Actual ELF program execution** with user/kernel mode separation

### Phase 3: Advanced Features (Long Term)
- **Real ext4 filesystem** with actual disk I/O (if needed)
- **Network stack** (simple TCP/IP implementation)
- **Multi-core support** (SMP)

*See [Roadmap](docs/en/roadmap.md) for detailed development plans.*

## 🛠 Prerequisites

- **Rust** with `riscv64gc-unknown-none-elf` target
- **QEMU** RISC-V system emulator
- **Linux/macOS/WSL** development environment

*See [Getting Started Guide](docs/en/getting-started.md) for detailed setup instructions.*

## 🏆 Why elinKernel?

### For Learning
- **Clear, readable code** demonstrating OS concepts
- **Well-organized architecture** following good practices
- **Comprehensive documentation** with detailed explanations
- **Incremental complexity** from basic to advanced features

### For Development
- **Educational patterns** great for learning system programming
- **Type-safe implementation** preventing common OS bugs
- **Modular design** easy to extend and experiment with
- **Modern tooling** with Rust ecosystem benefits

### For Research
- **RISC-V native** supporting latest open ISA
- **Extensible foundation** for experimental features
- **Clean abstractions** for academic use
- **Well-documented interfaces** for modification

## 🤝 Contributing

We welcome contributions! Areas of focus:

- **Core system development** - syscalls, memory management, devices
- **User applications** - shell commands, utilities
- **Testing & QA** - test cases, quality assurance
- **Documentation** - guides, tutorials, API docs

*See [Development Guide](docs/en/development.md) for contribution details.*

## 📄 License

MIT License - see the [LICENSE](LICENSE) file for details.

This project is free and open source software, allowing unrestricted use, modification, and distribution.

---

**elinKernel** demonstrates **educational-quality** kernel development in Rust, providing an excellent **learning platform** and **experimental foundation** for RISC-V system development.

**🎮 Try it now:** `./build.sh && ./run.sh`