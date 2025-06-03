# elinOS

**Language / 语言**: [English](README.md) | [简体中文](README_zh.md)

An **experimental** RISC-V64 kernel written in Rust, featuring dynamic memory management, well-organized system call architecture, **embedded ext4 filesystem**, **ELF program execution capability**, and **modern console abstraction**. Perfect for **learning**, **research**, and **educational purposes**.

## 🚀 Quick Start

```bash
# Build and run elinOS
make
./run.sh
```

**What you'll see:**
- Interactive shell with comprehensive commands
- Professional system call architecture (9 categories, 15+ syscalls)
- **Embedded ext4 filesystem** with realistic superblock data
- **Simplified device management** (no complex VirtIO)
- ELF binary loading and analysis
- **Multi-device console output** (UART + HDMI support)
- System information and memory management

## ✨ Key Features

### 🖥️ **Advanced Console System**
- **Multi-device output support** - UART, HDMI, framebuffer, network console
- **Environment-aware configuration** (development, production, headless)
- **Clean macro interface** - `console_println!()` vs manual UART locking
- **RISC-V board ready** - designed for real hardware with multiple outputs
- **Performance optimized** - single lock per operation instead of manual management

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
- **Abstracted console output** supporting multiple devices
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

## 🖥️ Console System Usage

The new console abstraction supports multiple output devices for real RISC-V boards:

```rust
// Simple output (goes to default device or all devices)
console_println!("System message: boot complete");

// Targeted output for specific scenarios  
console_print_to!(OutputDevice::Uart, "Debug: internal state = {}\n", 42);
console_print_to!(OutputDevice::Hdmi, "User: Welcome to elinOS GUI\n");
console_print_to!(OutputDevice::All, "Critical: low memory warning\n");
```

### Console Configurations

- **Development**: UART for debugging, HDMI for user interface
- **Production**: Both UART and HDMI for redundancy
- **Headless**: UART only for remote monitoring

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

Once elinOS boots, explore its capabilities:

```bash
elinOS> help                    # Show all available commands
elinOS> version                 # Display elinOS version info
elinOS> memory                  # View memory layout
elinOS> ext4check               # Check embedded ext4 filesystem
elinOS> disktest                # Test filesystem operations
elinOS> syscall                 # Show system call architecture
elinOS> ls                      # List filesystem contents
elinOS> cat hello.txt           # Display file contents
elinOS> elf-info hello.elf      # Analyze ELF binary structure
elinOS> elf-load hello.elf      # Load ELF into memory
elinOS> shutdown                # Graceful system shutdown
```

## 🎯 Current Capabilities

- ✅ **Complete boot process** from OpenSBI to interactive shell
- ✅ **Professional syscall system** with 9 categories covering all OS functionality
- ✅ **Dynamic command dispatch** - easy to add new commands
- ✅ **Memory management** with automatic configuration
- ✅ **Embedded ext4 filesystem** with realistic superblock and metadata
- ✅ **File operations** create, read, delete, list files
- ✅ **ELF binary loading** parse and load RISC-V executables
- ✅ **Multi-device console** UART + HDMI support for real RISC-V boards
- ✅ **System information** commands for debugging and monitoring
- ✅ **Clean shutdown/reboot** via OpenSBI interface
- ✅ **Educational simplicity** - focus on core OS concepts without device complexity

## 🚧 Coming Next

### Phase 1: Foundation (Short Term)
- **Complete syscall implementation** (SYS_READ, directory operations)
- **Enhanced memory management** (mmap, memory protection)
- **Extended filesystem commands** (mkdir, file permissions)
- **HDMI framebuffer implementation** for visual output

### Phase 2: Execution (Medium Term)
- **Virtual memory management** (RISC-V Sv39 page tables)
- **Process management** (fork, exec, scheduling)
- **Actual ELF program execution** with user/kernel mode separation

### Phase 3: Advanced Features (Long Term)
- **Real ext4 filesystem** with actual disk I/O (if needed)
- **Network console** (remote debugging/management)
- **Network stack** (simple TCP/IP implementation)
- **Multi-core support** (SMP)

*See [Roadmap](docs/en/roadmap.md) for detailed development plans.*

## 🛠 Prerequisites

- **Rust** with `riscv64gc-unknown-none-elf` target
- **QEMU** RISC-V system emulator
- **Linux/macOS/WSL** development environment

*See [Getting Started Guide](docs/en/getting-started.md) for detailed setup instructions.*

## 🏆 Why elinOS?

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

### For Real Hardware
- **Multi-device console** supporting UART + HDMI output
- **Environment-aware** configuration for different deployment scenarios
- **Performance optimized** console operations
- **RISC-V board ready** architecture

## 🤝 Contributing

We welcome contributions! Areas of focus:

- **Core system development** - syscalls, memory management, devices
- **User applications** - shell commands, utilities
- **Testing & QA** - test cases, quality assurance
- **Documentation** - guides, tutorials, API docs
- **Hardware support** - HDMI framebuffer, additional RISC-V boards

*See [Development Guide](docs/en/development.md) for contribution details.*

## 📄 License

MIT License - see the [LICENSE](LICENSE) file for details.

This project is free and open source software, allowing unrestricted use, modification, and distribution.

---

**elinOS** demonstrates **educational-quality** kernel development in Rust, providing an excellent **learning platform** and **experimental foundation** for RISC-V system development with **modern console abstraction** ready for real hardware deployment.

**🎮 Try it now:** `make && ./run.sh`