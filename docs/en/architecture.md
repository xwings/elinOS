# elinKernel Technical Architecture

This document provides detailed technical information about elinKernel architecture, design principles, and implementation details.

## Design Philosophy

elinKernel prioritizes **educational clarity** over production complexity:

- **Embedded Filesystem**: Simple ext4 implementation without complex device drivers
- **Direct Hardware Access**: UART communication without VirtIO overhead  
- **Focus on Core Concepts**: Memory management, syscalls, and ELF loading
- **Rust Safety**: Type-safe implementation preventing common OS bugs
- **Modular Design**: Clean separation of concerns for easy learning

## System Call Architecture

elinKernel implements a **well-structured** kernel architecture with industry-standard organization inspired by the **Qiling framework**.

### Categorized System Call Organization

System calls are organized into logical categories with dedicated number ranges:

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
| 900-999 | elinKernel-Specific Operations | OS-specific features | debug, version, shutdown |

### Kernel Space Design (`src/syscall/`)

**Modular Organization:**
- Each category managed in separate files (file.rs, memory.rs, etc.)
- Clear ownership and isolated changes
- Easy testing and development

**Central Dispatcher:**
- Range-based routing in `mod.rs`
- Efficient O(1) category lookup
- Type-safe parameter passing

**Error Handling:**
- `SysCallResult` enum for consistent error handling
- Input validation and boundary enforcement
- Security through parameter checking

**Implementation Structure:**
```rust
// src/syscall/mod.rs - Central dispatcher
pub fn handle_syscall(syscall_num: usize, args: &[usize]) -> SysCallResult {
    match syscall_num {
        1..=50 => file::handle_file_syscall(syscall_num, args),
        51..=70 => directory::handle_directory_syscall(syscall_num, args),
        71..=120 => memory::handle_memory_syscall(syscall_num, args),
        121..=170 => process::handle_process_syscall(syscall_num, args),
        // ... other categories
        _ => SysCallResult::Error("Invalid system call number"),
    }
}
```

## Memory Layout

### Physical Memory Organization

```
0x80000000  ┌─────────────────┐  ← Kernel Start
            │ Kernel Code     │  (2MB reserved)
0x80200000  ├─────────────────┤  ← Kernel End
            │ Heap Space      │  (Dynamically sized)
            ├─────────────────┤
            │ Hart 0 Stack    │  (2MB)
            ├─────────────────┤
            │ Hart 1 Stack    │  (2MB)
            ├─────────────────┤
            │ Hart 2 Stack    │  (2MB)
            ├─────────────────┤
            │ Hart 3 Stack    │  (2MB)
0x88000000  └─────────────────┘  ← Memory End (128MB default)
```

### Memory Detection

Dynamic memory discovery through OpenSBI:

```rust
// src/memory.rs
pub fn detect_memory() -> &'static [MemoryRegion] {
    // Query OpenSBI for memory layout
    // Configure heap based on available RAM
    // Set up per-hart stacks
}
```

### Memory Management

- **Heap**: Configured based on detected RAM size
- **Stack**: 2MB per hart (supports up to 4 harts)
- **Kernel**: Fixed 2MB region at start
- **Alignment**: 8-byte aligned allocations

## Project Structure

```
src/
├── main.rs              # Kernel entry point and shell
├── syscall/             # System call interface (kernel space)
│   ├── mod.rs           # Central dispatcher and utilities
│   ├── file.rs          # File I/O operations (1-50)
│   ├── directory.rs     # Directory operations (51-70)
│   ├── memory.rs        # Memory management (71-120)
│   ├── process.rs       # Process management (121-170)
│   ├── device.rs        # Device and I/O management (171-220)
│   ├── network.rs       # Network operations (221-270)
│   ├── time.rs          # Time and timer operations (271-300)
│   ├── sysinfo.rs       # System information (301-350)
│   └── elinos.rs        # elinKernel-specific operations (900-999)
├── commands.rs          # User space commands with dynamic dispatch
├── memory.rs            # Dynamic memory management
├── sbi.rs              # OpenSBI interface with shutdown support
├── filesystem.rs       # Embedded ext4 filesystem implementation
├── elf.rs              # ELF loader implementation
└── linker.ld           # Linker script with flexible memory layout
```

## Key Components

### System Call Interface

**Professional Organization:**
- 9-file structure, each category self-contained
- Industry standard following Qiling framework methodology
- Range-based numbers for future-proof expansion
- Well-organized design with 15+ implemented syscalls across 6 categories

**Development Benefits:**
- Clear ownership - each developer can work on specific categories
- Isolated changes - modifications don't affect other categories
- Easy testing - each category can be unit tested independently
- Scalable design - new syscalls fit naturally within categories

### Dynamic Command System

**Architecture:**
```rust
// src/commands.rs
pub fn process_command(input: &str) -> Result<(), &'static str> {
    let parts: Vec<&str> = input.trim().split_whitespace().collect();
    let command = parts[0];
    
    match command {
        "help" => cmd_help(),
        "memory" => cmd_memory(),
        "ls" => cmd_ls(),
        // Automatically dispatched - no main.rs changes needed
        _ => Err("Unknown command"),
    }
}
```

**Benefits:**
- **Self-Registering**: Commands automatically available in shell
- **Centralized Dispatch**: Single function handles all routing
- **Error Handling**: Consistent error reporting
- **Extensible**: Add command function + update help, that's it!

### ELF Loader

**Professional Implementation:**
```rust
// src/elf.rs
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ElfHeader {
    pub e_ident: [u8; 16],    // ELF identification
    pub e_type: u16,          // Object file type
    pub e_machine: u16,       // Machine type
    pub e_version: u32,       // Object file version
    pub e_entry: u64,         // Entry point address
    pub e_phoff: u64,         // Program header offset
    pub e_shoff: u64,         // Section header offset
    pub e_flags: u32,         // Processor-specific flags
    pub e_ehsize: u16,        // ELF header size
    pub e_phentsize: u16,     // Program header entry size
    pub e_phnum: u16,         // Program header entries
    pub e_shentsize: u16,     // Section header entry size
    pub e_shnum: u16,         // Section header entries
    pub e_shstrndx: u16,      // Section header string table index
}
```

**Features:**
- Complete ELF64 parsing and validation
- Program header analysis and segment information
- Memory-safe implementation using Rust's type system
- Integration with syscall system through process management category

### Embedded ext4 Filesystem

**Educational Implementation:**
```rust
// src/filesystem.rs
struct EmbeddedBlockDevice {
    // Simple in-memory block device
}

impl EmbeddedBlockDevice {
    fn read_block(&mut self, block_num: u64, buffer: &mut [u8]) -> Result<(), &'static str> {
        match block_num {
            0 => {
                // Block 0: Contains ext4 superblock at offset 1024
                let superblock = Ext4Superblock {
                    s_magic: EXT4_SUPER_MAGIC,
                    s_inodes_count: 65536,
                    s_blocks_count_lo: 65536,
                    // ... realistic ext4 metadata
                };
                // Write to buffer at correct offset
            },
            _ => {
                // Other blocks: metadata or data blocks
            }
        }
    }
}
```

**Benefits:**
- **Educational Focus**: Learn ext4 structure without device complexity
- **Realistic Data**: Proper ext4 superblock with correct magic numbers
- **Simple Architecture**: No VirtIO, MMIO, or complex device protocols
- **Easy Debugging**: All data in memory, no hardware dependencies

### Memory Management

**Dynamic Configuration:**
```rust
// src/memory.rs
pub fn configure_memory(total_ram: usize) -> MemoryLayout {
    let kernel_size = 2 * 1024 * 1024; // 2MB
    let stack_per_hart = 2 * 1024 * 1024; // 2MB
    let max_harts = 4;
    let heap_size = total_ram - kernel_size - (stack_per_hart * max_harts);
    
    MemoryLayout {
        kernel: 0x80000000..0x80200000,
        heap: 0x80200000..(0x80200000 + heap_size),
        stacks: calculate_stack_regions(max_harts),
    }
}
```

### System Control

**OpenSBI Integration:**
```rust
// src/sbi.rs
pub fn shutdown() -> ! {
    sbi_call(SBI_SHUTDOWN, 0, 0, 0, 0, 0, 0);
    loop {}
}

pub fn reboot() -> ! {
    sbi_call(SBI_RESET, RESET_TYPE_COLD_REBOOT, RESET_REASON_NO_REASON, 0, 0, 0, 0);
    loop {}
}
```

## Boot Process

1. **OpenSBI Firmware** initializes platform and loads kernel
2. **Kernel Entry** starts at `_start` symbol in `main.rs`
3. **Memory Detection** via OpenSBI calls to discover RAM layout
4. **Memory Configuration** sets up heap, stacks, and kernel regions
5. **Device Probing** scans for VirtIO devices at known MMIO addresses
6. **Filesystem Initialization** creates in-memory filesystem with test files
7. **System Call Interface** activates all categories and handlers
8. **Shell Startup** begins interactive user session

## Safety and Correctness

### Rust Safety Features

- **Memory Safety**: No null pointer dereferences or buffer overflows
- **Type Safety**: Compile-time guarantees about data types
- **Concurrency Safety**: Send/Sync traits prevent data races
- **Bounds Checking**: Array and vector access validated at runtime

### Minimal Unsafe Code

```rust
// Only hardware access requires unsafe
unsafe {
    // MMIO register access
    ptr::read_volatile(addr as *const u32)
}
```

### System Call Boundary

- **Input Validation**: All parameters checked before use
- **Boundary Enforcement**: User/kernel memory separation
- **Error Propagation**: Consistent error handling throughout

### Synchronization

- **Spin Locks**: Simple but effective for single-core operation
- **Atomic Operations**: Where needed for device interactions
- **No Heap Allocation**: Kernel uses stack and static storage only

## Design Principles

### Clean Architecture
- **Scalable Organization**: Easy to add new features within categories
- **Maintainable Structure**: Clear ownership and responsibility
- **Industry Standards**: Follows proven frameworks like Qiling

### Type Safety
- **Leverages Rust**: Full advantage of Rust's type system
- **Compile-time Guarantees**: Prevents entire classes of bugs
- **Safe Abstractions**: Hardware access wrapped in safe interfaces

### Modular Design
- **Loose Coupling**: Components interact through well-defined interfaces
- **High Cohesion**: Related functionality grouped together
- **Easy Extension**: Adding features doesn't require architectural changes

### Clear Boundaries
- **Kernel/User Separation**: System calls provide controlled interface
- **Category Isolation**: System call categories are independent
- **Component Interfaces**: Well-defined APIs between major components

## Performance Characteristics

### System Call Overhead
- **O(1) Dispatch**: Range-based routing is constant time
- **Minimal Copying**: Parameters passed by reference where possible
- **Zero Allocations**: No heap allocations in critical paths

### Memory Efficiency
- **Static Sizing**: Most data structures have compile-time bounds
- **Stack Allocation**: Kernel operations use stack memory
- **Efficient Collections**: `heapless` provides zero-allocation data structures

### Boot Time
- **Fast Detection**: Memory and device discovery is efficient
- **Minimal Initialization**: Only essential components initialized
- **Direct Boot**: No complex initialization sequences

## Future Architecture Considerations

### Virtual Memory
- **Page Tables**: RISC-V Sv39 virtual memory support
- **Address Spaces**: Separate user and kernel address spaces
- **Memory Protection**: Hardware-enforced access controls

### Multiprocessing
- **SMP Support**: Symmetric multiprocessing with proper synchronization
- **Load Balancing**: Work distribution across multiple harts
- **NUMA Awareness**: Non-uniform memory access optimization

### Advanced I/O
- **DMA Support**: Direct memory access for high-performance I/O
- **Interrupt Handling**: Asynchronous device notifications
- **I/O Scheduling**: Efficient ordering of device operations

### Security Framework
- **Capabilities**: Fine-grained access control
- **Sandboxing**: Process isolation and resource limits
- **Secure Boot**: Verified boot process with cryptographic signatures

This architecture provides a solid foundation for an educational kernel while maintaining the simplicity and safety that makes elinKernel an excellent learning and development platform. 