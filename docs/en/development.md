# elinOS Development Guide

## Table of Contents
- [Development Environment Setup](#development-environment-setup)
- [Build System](#build-system)
- [Testing & Debugging](#testing--debugging)
- [Code Structure](#code-structure)
- [Contributing Guidelines](#contributing-guidelines)
- [Advanced Development](#advanced-development)

## Development Environment Setup

### Prerequisites

#### Rust Toolchain
```bash
# Install Rust via rustup
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env

# Install nightly toolchain (required for no_std features)
rustup toolchain install nightly
rustup default nightly

# Add RISC-V target
rustup target add riscv64gc-unknown-none-elf
```

#### QEMU RISC-V Emulation
```bash
# Ubuntu/Debian
sudo apt update
sudo apt install qemu-system-riscv64

# Arch Linux
sudo pacman -S qemu-system-riscv

# macOS (with Homebrew)
brew install qemu

# Verify installation
qemu-system-riscv64 --version
```

#### Build Tools
```bash
# Ubuntu/Debian
sudo apt install build-essential git make

# Arch Linux  
sudo pacman -S base-devel git make

# macOS
xcode-select --install
```

### Recommended Development Tools

#### Visual Studio Code Extensions
- **rust-analyzer**: Advanced Rust language support
- **CodeLLDB**: Debugging support for Rust
- **Even Better TOML**: Better TOML file support
- **GitLens**: Enhanced Git integration

#### Command Line Tools
```bash
# Rust development tools
cargo install cargo-expand    # Macro expansion
cargo install cargo-edit      # Easy dependency management
cargo install cargo-watch     # Auto-rebuild on changes

# Optional: Advanced debugging
cargo install gdb-multiarch   # Cross-platform debugging
```

## Build System

### Makefile Targets

elinOS uses a comprehensive Makefile for development workflow:

```bash
# Core build commands
make build          # Build the kernel
make clean          # Clean build artifacts
make rebuild        # Clean and build

# Running the kernel
make run            # Run in QEMU (console mode)
make run-graphics   # Run in QEMU with graphics
make run-debug      # Run with GDB debugging enabled

# Testing
make test           # Run unit tests
make integration    # Run integration tests
make bench          # Run benchmarks

# Development helpers
make format         # Format code with rustfmt
make clippy         # Run Clippy linter
make doc            # Generate documentation
make check-all      # Run all checks (format, clippy, tests)

# Disk image creation
make create-disk    # Create test FAT32 disk image
make create-ext2    # Create test ext2 disk image
```

### Build Configuration

#### Cargo.toml Features
```toml
[features]
default = ["console"]
console = []
graphics = []
debug = []
test-allocator = []
benchmark = []
```

#### Custom Build Scripts

The build process uses several custom components:

1. **Linker Script** (`kernel.ld`): Defines memory layout
2. **Assembly Boot Code** (`boot.S`): Initial hardware setup
3. **Build Script** (`build.rs`): Custom build logic

### Cross-Compilation Details

```bash
# Target triple: riscv64gc-unknown-none-elf
# - riscv64: 64-bit RISC-V
# - gc: General-purpose + Compressed instruction sets
# - unknown: Unknown vendor
# - none: No operating system
# - elf: ELF binary format
```

## Testing & Debugging

### Unit Testing

elinOS supports unit testing in a `no_std` environment:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_memory_allocation() {
        // Test memory allocator functionality
        let ptr = allocate_memory(1024);
        assert!(ptr.is_some());
        deallocate_memory(ptr.unwrap(), 1024);
    }
}
```

Run tests with:
```bash
make test
# or
cargo test --target riscv64gc-unknown-none-elf
```

### Integration Testing

Integration tests verify system-level functionality:

```bash
# Run kernel in test mode
make run ARGS="--test-mode"

# Automated testing with expect scripts
./scripts/test-integration.sh
```

### Debugging Strategies

#### QEMU Monitor
```bash
# Run with monitor enabled
make run-debug

# In QEMU monitor (Ctrl+Alt+2):
(qemu) info registers     # Show CPU registers
(qemu) info mem          # Show memory layout
(qemu) x/10i $pc         # Disassemble at PC
```

#### Serial Console Debugging
```rust
// Use console_println! for debugging output
console_println!("Debug: value = {}", value);

// Conditional debug output
#[cfg(feature = "debug")]
console_println!("Debug info: {:?}", structure);
```

#### GDB Integration
```bash
# Start QEMU with GDB stub
make run-gdb

# In another terminal
gdb target/riscv64gc-unknown-none-elf/debug/elinOS
(gdb) target remote :1234
(gdb) break main
(gdb) continue
```

### Performance Profiling

#### Memory Usage Analysis
```bash
# Show memory statistics in kernel
elinOS> memory

# Advanced memory debugging
elinOS> syscall SYS_GETMEMINFO
```

#### Timing Analysis
```rust
// Simple timing measurement
let start = get_time();
perform_operation();
let elapsed = get_time() - start;
console_println!("Operation took {} cycles", elapsed);
```

## Code Structure

### Directory Layout
```
elinOS/
├── src/                    # Kernel source code
│   ├── main.rs            # Kernel entry point
│   ├── console/           # Console system
│   ├── memory/            # Memory management
│   │   ├── mod.rs         # Memory manager
│   │   ├── buddy.rs       # Buddy allocator
│   │   ├── slab.rs        # Slab allocator
│   │   └── fallible.rs    # Fallible operations
│   ├── filesystem/        # Filesystem support
│   │   ├── mod.rs         # VFS layer
│   │   ├── fat32.rs       # FAT32 implementation
│   │   └── ext2.rs        # ext2 implementation
│   ├── syscall/           # System call interface
│   ├── virtio_blk.rs      # VirtIO block driver
│   └── sbi.rs             # SBI interface
├── docs/                  # Documentation
├── scripts/               # Build/test scripts
├── Makefile              # Build system
├── kernel.ld             # Linker script
└── Cargo.toml           # Rust package manifest
```

### Coding Standards

#### Rust Idioms
```rust
// Use descriptive names
fn calculate_optimal_heap_size(ram_size: usize) -> usize { ... }

// Prefer Option/Result over panics
fn try_allocate_memory(size: usize) -> Option<*mut u8> { ... }

// Use const generics for type safety
struct SlabAllocator<const SIZE: usize> { ... }

// Document public APIs
/// Allocates memory using the best available allocator
/// 
/// # Arguments
/// * `size` - Number of bytes to allocate
/// 
/// # Returns
/// * `Some(ptr)` - Pointer to allocated memory
/// * `None` - Allocation failed
pub fn allocate_memory(size: usize) -> Option<*mut u8> { ... }
```

#### Error Handling Patterns
```rust
// Use custom error types
#[derive(Debug)]
pub enum MemoryError {
    OutOfMemory,
    InvalidSize,
    AlignmentError,
}

// Implement From trait for error conversion
impl From<AllocError> for MemoryError {
    fn from(err: AllocError) -> Self {
        match err {
            AllocError::OutOfMemory => MemoryError::OutOfMemory,
            AllocError::InvalidSize => MemoryError::InvalidSize,
        }
    }
}
```

#### Safety Guidelines
```rust
// Always document unsafe blocks
unsafe {
    // SAFETY: We know ptr is valid because we just allocated it
    // and checked for null
    *ptr = value;
}

// Use safe abstractions where possible
fn safe_write_memory(addr: usize, value: u8) -> Result<(), MemoryError> {
    if addr == 0 {
        return Err(MemoryError::InvalidAddress);
    }
    
    unsafe {
        // SAFETY: Address validated above
        *(addr as *mut u8) = value;
    }
    
    Ok(())
}
```

## Contributing Guidelines

### Development Workflow

1. **Fork & Clone**
   ```bash
   git clone https://github.com/yourusername/elinOS.git
   cd elinOS
   ```

2. **Create Feature Branch**
   ```bash
   git checkout -b feature/amazing-new-feature
   ```

3. **Make Changes**
   - Follow coding standards
   - Add tests for new functionality
   - Update documentation

4. **Test Thoroughly**
   ```bash
   make check-all      # Run all checks
   make test           # Unit tests
   make integration    # Integration tests
   ```

5. **Commit with Clear Messages**
   ```bash
   git commit -m "memory: Add fallible allocation with rollback
   
   - Implement transaction-based allocation system
   - Add automatic rollback on failure
   - Include comprehensive test coverage
   - Update documentation with usage examples"
   ```

6. **Push and Submit PR**
   ```bash
   git push origin feature/amazing-new-feature
   ```

### Code Review Process

#### PR Requirements
- [ ] All tests pass
- [ ] Code follows style guidelines
- [ ] Documentation updated
- [ ] Performance impact assessed
- [ ] Memory safety verified

#### Review Checklist
- **Correctness**: Does the code do what it claims?
- **Safety**: Are unsafe blocks properly justified?
- **Performance**: Are there any performance regressions?
- **Style**: Does it follow project conventions?
- **Tests**: Are edge cases covered?

### Commit Message Guidelines

```
component: Brief description (50 chars or less)

Longer explanation of the change, motivation, and impact.
Include any breaking changes or migration notes.

Fixes #123
```

Examples:
```
memory: Implement dynamic memory zone detection

virtio: Add support for VirtIO 1.1 specification

fs: Fix directory traversal in FAT32 implementation
```

## Advanced Development

### Adding New Filesystems

1. **Implement FileSystem Trait**
   ```rust
   pub struct MyFileSystem {
       // filesystem-specific data
   }
   
   impl FileSystem for MyFileSystem {
       fn get_filesystem_type(&self) -> FilesystemType {
           FilesystemType::MyFS
       }
       
       // ... implement other methods
   }
   ```

2. **Add Detection Logic**
   ```rust
   fn detect_myfs(boot_sector: &[u8]) -> bool {
       // Check for filesystem signature
       boot_sector[0..4] == b"MYFS"
   }
   ```

3. **Register in VFS**
   ```rust
   // In filesystem/mod.rs
   match filesystem_type {
       FilesystemType::MyFS => Box::new(MyFileSystem::new(device)?),
       // ... other filesystems
   }
   ```

### Memory Allocator Development

1. **Implement Allocator Trait**
   ```rust
   pub struct MyAllocator {
       // allocator state
   }
   
   impl Allocator for MyAllocator {
       fn allocate(&mut self, size: usize) -> Result<NonNull<u8>, AllocError> {
           // allocation logic
       }
       
       fn deallocate(&mut self, ptr: NonNull<u8>, size: usize) {
           // deallocation logic
       }
   }
   ```

2. **Add to Memory Manager**
   ```rust
   // In memory/mod.rs
   pub enum AllocatorType {
       Simple,
       Buddy,
       Slab,
       MyAllocator, // Add your allocator
   }
   ```

### Device Driver Development

1. **Implement Device Trait**
   ```rust
   pub struct MyDevice {
       base_addr: usize,
       // device-specific state
   }
   
   impl Device for MyDevice {
       fn init(&mut self) -> Result<(), DeviceError> {
           // device initialization
       }
       
       fn read(&self, buffer: &mut [u8]) -> Result<usize, DeviceError> {
           // read implementation
       }
   }
   ```

2. **Add Device Detection**
   ```rust
   fn probe_my_device(base_addr: usize) -> Option<MyDevice> {
       // Check device signature
       let signature = unsafe { *(base_addr as *const u32) };
       if signature == MY_DEVICE_SIGNATURE {
           Some(MyDevice::new(base_addr))
       } else {
           None
       }
   }
   ```

### Performance Optimization

#### Profiling
```rust
// Add timing measurements
#[cfg(feature = "benchmark")]
fn benchmark_function() {
    let start = read_cycle_counter();
    
    // Function to benchmark
    perform_operation();
    
    let cycles = read_cycle_counter() - start;
    console_println!("Operation took {} cycles", cycles);
}
```

#### Memory Layout Optimization
```rust
// Use repr(C) for predictable layout
#[repr(C)]
struct OptimizedStruct {
    // Order fields by size (largest first)
    large_field: u64,
    medium_field: u32,
    small_field: u8,
}
```

#### Assembly Integration
```rust
// Use inline assembly for critical paths
#[inline(always)]
fn fast_memory_copy(dest: *mut u8, src: *const u8, len: usize) {
    unsafe {
        asm!(
            "call memcpy",
            in("a0") dest,
            in("a1") src,
            in("a2") len,
            options(nostack)
        );
    }
}
```

---

*This development guide should help you contribute effectively to elinOS while maintaining code quality and project standards.* 