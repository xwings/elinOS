# elinOS Technical Architecture

This document provides detailed technical information about elinOS architecture, design principles, and implementation details.

## Design Philosophy

elinOS prioritizes **experimental clarity** over production complexity:

- **Embedded Filesystem**: Simple ext4 implementation without complex device drivers
- **Direct Hardware Access**: UART communication without VirtIO overhead  
- **Focus on Core Concepts**: Memory management, syscalls, and ELF loading
- **Rust Safety**: Type-safe implementation preventing common OS bugs
- **Modular Design**: Clean separation of concerns for easy learning

## System Call Architecture

elinOS implements a **well-structured** kernel architecture with industry-standard organization inspired by the **Qiling framework**.

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
| 900-999 | elinOS-Specific Operations | OS-specific features | debug, version, shutdown |

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
│   └── elinos.rs        # elinOS-specific operations (900-999)
├── commands.rs          # User space commands with dynamic dispatch
├── memory.rs            # Dynamic memory management
├── sbi.rs              # OpenSBI interface with shutdown support
├── filesystem.rs       # Embedded ext4 filesystem implementation
├── elf.rs              # ELF loader implementation
└── linker.ld           # Linker script with flexible memory layout
```

## Key Components

### System Call Interface

**File Organization:**
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

**Implementation:**
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

**Experimental Implementation:**
- Real ext4 superblock with correct magic numbers and metadata
- In-memory filesystem with POSIX-like operations
- File management commands (ls, cat, touch, rm)
- Experimental ext4 implementation demonstrating filesystem concepts
- No complex block device drivers - focus on filesystem logic

**Advantages:**
- **Experimental Focus**: Learn ext4 structure without device complexity
- **Real Data**: Correct ext4 superblock and magic numbers
- **Simple Architecture**: No VirtIO, MMIO, or complex device protocols
- **Easy Debugging**: All data in memory, no hardware dependencies

## Design Principles

### Clear Architecture
- **Extensible Organization**: Easy to add new features within categories
- **Maintainable Structure**: Clear ownership and responsibilities
- **Industry Standards**: Follows mature frameworks like Qiling

### Type Safety
- **Leverage Rust**: Full utilization of Rust's type system
- **Compile-Time Guarantees**: Prevents entire classes of bugs
- **Safe Abstractions**: Hardware access wrapped in safe interfaces

## Development Patterns

### Adding New System Calls

1. **Choose Category**: Determine appropriate numerical range
2. **Implement Handler**: Add function in category file
3. **Update Dispatcher**: Add case in central router
4. **Add Command**: Optional user space command in `commands.rs`
5. **Document**: Update help and documentation

### Adding New Commands

1. **Implement Function**: Add `cmd_name()` function in `commands.rs`
2. **Update Dispatcher**: Add match case in `process_command()`
3. **Update Help**: Add to help command output
4. **Test**: Command immediately available in shell

### Memory Management

**Safe Allocation:**
```rust
// Memory-safe heap allocation
let memory = memory::allocate_aligned(size, alignment)?;
defer! { memory::deallocate(memory, size); }
```

**Stack Management:**
- Per-hart stacks prevent interference
- 2MB size supports deep call chains
- Automatic overflow detection

## Testing and Debugging

### Built-in Debugging
- Memory layout visualization with `memory` command
- System call tracing capabilities
- Real-time system information

### QEMU Integration
- Native RISC-V 64-bit execution
- GDB debugging support
- Console output for development

### Error Handling
- Comprehensive error messages
- Graceful failure modes
- Recovery mechanisms

## Future Architecture

### Planned Extensions
- **Virtual Memory**: RISC-V Sv39 page tables
- **Process Isolation**: User/kernel mode separation
- **Network Stack**: TCP/IP implementation
- **Device Drivers**: Additional hardware support

### Scalability Considerations
- **Multi-core Support**: SMP-safe data structures
- **Performance**: Lock-free algorithms where possible
- **Modularity**: Plugin architecture for extensions

This architecture provides a solid foundation for an experimental kernel while maintaining the simplicity and safety that makes elinOS an excellent learning and development platform.

For implementation details and code examples, see:
- [Getting Started Guide](getting-started.md)
- [Development Guide](development.md)
- [Command Reference](commands.md) 