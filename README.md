# ElinOS

A minimal operating system written in Rust for RISC-V 64-bit architecture, featuring dynamic memory management, VirtIO device support, and a simple filesystem.

## 🚀 Features

### Core System
- **RISC-V 64-bit** target architecture
- **Dynamic memory detection** via OpenSBI
- **Adaptive memory management** with configurable heap and stack
- **Serial UART** communication and debugging
- **OpenSBI** integration for platform services

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

### Interactive Shell
- **Command-line interface** with history
- **Built-in commands** for system inspection and file management
- **Backspace support** and input editing
- **Help system** with command documentation

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
- `memory` - Display detected memory regions
- `devices` - Probe and list VirtIO devices

### File System Operations
- `ls` - List all files with sizes
- `cat <filename>` - Display file contents
- `touch <filename>` - Create a new empty file
- `rm <filename>` - Delete a file

### Utilities
- `clear` - Clear the screen

### Example Session
```
elinOS> help
Available commands:
  help     - Show this help
  memory   - Show memory information
  devices  - Probe for VirtIO devices
  ls       - List files
  cat <file> - Show file contents
  touch <file> - Create empty file
  rm <file> - Delete file
  clear    - Clear screen

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
```

## 🏗 Architecture

### Memory Layout
- **Kernel**: Loaded at `0x80200000` (2MB reserved)
- **Heap**: Dynamically configured based on detected RAM
- **Stack**: 2MB per hart (up to 4 harts supported)
- **Memory Detection**: Automatic via OpenSBI calls

### Project Structure
```
src/
├── main.rs          # Kernel entry point and shell
├── memory.rs        # Dynamic memory management
├── sbi.rs          # OpenSBI interface
├── virtio_blk.rs   # VirtIO block device driver
├── filesystem.rs   # In-memory filesystem
└── linker.ld       # Linker script with flexible memory layout
```

### Key Components

#### Memory Management (`memory.rs`)
- OpenSBI-based memory region detection
- Dynamic heap configuration
- Memory allocator with 8-byte alignment

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

- [ ] **Real VirtIO I/O** - Complete block device implementation
- [ ] **FAT32 Support** - Integration with `fatfs` crate
- [ ] **Network Support** - VirtIO network device driver
- [ ] **Process Management** - Basic multitasking and scheduling
- [ ] **Text Editor** - Simple file editing capabilities
- [ ] **System Calls** - User/kernel space separation

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

## 📚 Technical Details

### Boot Process
1. **OpenSBI** loads and initializes the platform
2. **Kernel** starts at `_start` in `main.rs`
3. **Memory detection** via OpenSBI calls
4. **Device probing** for VirtIO devices
5. **Filesystem initialization** with test files
6. **Shell startup** for user interaction

### VirtIO Implementation
- Follows VirtIO 1.0 specification
- MMIO-based device access
- Simplified queue management for proof-of-concept
- Ready for extension to full DMA-based I/O

### Safety and Correctness
- Written in **safe Rust** where possible
- Minimal `unsafe` blocks for hardware access
- Spin-lock based synchronization
- No heap allocation in kernel (uses stack and static storage)

## 📄 License

This project is open source. Feel free to use, modify, and distribute.

---

**ElinOS** - A minimal OS demonstrating modern kernel development practices in Rust.
