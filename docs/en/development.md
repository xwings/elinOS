# Creating User Programs for elinKernel

Since elinKernel has a built-in ELF loader, you can create and compile C programs to run on it! This guide shows how to create RISC-V binaries that elinKernel can load and execute.

## Prerequisites for User Program Development

- **RISC-V GCC toolchain**: Install `riscv64-linux-gnu-gcc` or `riscv64-unknown-elf-gcc`
- **Basic C knowledge**: For writing simple programs
- **ELF knowledge**: Understanding of executable format (optional)

## Installing RISC-V GCC Toolchain

### Ubuntu/Debian:
```bash
sudo apt update
sudo apt install gcc-riscv64-linux-gnu
```

### Arch Linux:
```bash
sudo pacman -S riscv64-linux-gnu-gcc
```

### From Source:
```bash
# Clone and build RISC-V GNU toolchain
git clone https://github.com/riscv/riscv-gnu-toolchain
cd riscv-gnu-toolchain
./configure --prefix=/opt/riscv --with-arch=rv64gc --with-abi=lp64d
make linux
```

## Creating a Hello World Program

Create a simple C program that can run on elinKernel:

**hello.c:**
```c
// Simple Hello World for elinKernel
// This program demonstrates basic execution on elinKernel

// Simple system call interface for elinKernel
// In a real implementation, you'd use proper syscall numbers
static inline long syscall_write(const char* msg, int len) {
    register long a0 asm("a0") = 1;        // stdout fd
    register long a1 asm("a1") = (long)msg; // buffer
    register long a2 asm("a2") = len;      // length
    register long a7 asm("a7") = 1;        // SYS_WRITE
    register long result asm("a0");
    
    asm volatile ("ecall"
                  : "=r" (result)
                  : "r" (a0), "r" (a1), "r" (a2), "r" (a7)
                  : "memory");
    return result;
}

static inline void syscall_exit(int status) {
    register long a0 asm("a0") = status;
    register long a7 asm("a7") = 121;      // SYS_EXIT
    
    asm volatile ("ecall"
                  :
                  : "r" (a0), "r" (a7)
                  : "memory");
}

// String length function
int strlen(const char* str) {
    int len = 0;
    while (str[len]) len++;
    return len;
}

// Main function - entry point
int main(void) {
    const char* message = "Hello, World from elinKernel user program!\n";
    syscall_write(message, strlen(message));
    
    const char* info = "This C program is running via ELF loader!\n";
    syscall_write(info, strlen(info));
    
    syscall_exit(0);
    return 0;  // Should never reach here
}

// Entry point that calls main
void _start(void) {
    int result = main();
    syscall_exit(result);
}
```

## Compiling the Program

Compile your C program to a RISC-V ELF binary:

```bash
# Compile hello.c to RISC-V ELF
riscv64-linux-gnu-gcc \
    -march=rv64gc \
    -mabi=lp64d \
    -static \
    -nostdlib \
    -nostartfiles \
    -fno-stack-protector \
    -o hello.elf \
    hello.c

# Alternative with unknown-elf toolchain:
riscv64-unknown-elf-gcc \
    -march=rv64gc \
    -mabi=lp64d \
    -static \
    -nostdlib \
    -nostartfiles \
    -fno-stack-protector \
    -o hello.elf \
    hello.c
```

## Compilation Options Explained

- **`-march=rv64gc`**: Target RISC-V 64-bit with standard extensions
- **`-mabi=lp64d`**: Use 64-bit ABI with double-precision floating point
- **`-static`**: Create statically linked executable
- **`-nostdlib`**: Don't link standard library (we provide our own syscalls)
- **`-nostartfiles`**: Don't use standard startup files
- **`-fno-stack-protector`**: Disable stack protection for simplicity

## Verifying Your ELF Binary

Check that your compiled program is a valid RISC-V ELF:

```bash
# Check ELF header
file hello.elf
# Output: hello.elf: ELF 64-bit LSB executable, UCB RISC-V, ...

# Examine ELF details
readelf -h hello.elf
# Should show Machine: RISC-V

# Check program segments
readelf -l hello.elf

# Disassemble to see the code
riscv64-linux-gnu-objdump -d hello.elf
```

## Adding Your Program to elinKernel

To make your program available in elinKernel, add it to the filesystem during initialization:

### Option 1: Add to src/filesystem.rs

Edit the `new()` function in `src/filesystem.rs`:

```rust
pub fn new() -> Self {
    let mut fs = SimpleFS {
        files: heapless::FnvIndexMap::new(),
    };
    
    // Add existing test files
    let _ = fs.create_file("hello.txt", b"Hello from elinKernel filesystem!\n");
    let _ = fs.create_file("test.txt", b"This is a test file.\nLine 2\nLine 3\n");
    let _ = fs.create_file("readme.md", b"# elinKernel\n\nA simple kernel in Rust.\n");
    
    // Add your compiled ELF binary
    let hello_elf = include_bytes!("../hello.elf");
    let _ = fs.create_file("hello.elf", hello_elf);
    
    fs
}
```

### Option 2: Convert to byte array

```bash
# Convert ELF to C-style byte array
xxd -i hello.elf > hello_elf.h

# Then manually copy the array into filesystem.rs
```

## Testing Your Program in elinKernel

Once you've added your program to the filesystem and rebuilt elinKernel:

```bash
# Rebuild elinKernel with your program
./build.sh

# Run elinKernel
./run.sh
```

In the elinKernel shell:

```
elinKernel> ls
Files:
  hello.txt (30 bytes)
  test.txt (35 bytes)
  readme.md (42 bytes)
  hello.elf (8432 bytes)

elinKernel> elf-info hello.elf
ELF Binary Information:
  Class: ELF64
  Data: Little-endian
  Machine: RISC-V
  Type: Executable
  Entry point: 0x10078
  Program header offset: 0x40
  Program header count: 2
  Section header offset: 0x1fd8
  Section header count: 8

elinKernel> elf-load hello.elf
Loading ELF binary: hello.elf
Loading ELF binary:
  Entry point: 0x10078
  Program headers: 2
  Segment 0: 0x10000 - 0x11000 (4096 bytes) flags: 0x5
  Segment 1: 0x11000 - 0x12000 (4096 bytes) flags: 0x6
ELF loaded successfully, entry at 0x10078
ELF binary loaded successfully!
Entry point: 0x10078

elinKernel> elf-exec hello.elf
Executing ELF binary: hello.elf
Loading ELF binary:
  Entry point: 0x10078
  Program headers: 2
  Segment 0: 0x10000 - 0x11000 (4096 bytes) flags: 0x5
  Segment 1: 0x11000 - 0x12000 (4096 bytes) flags: 0x6
ELF loaded successfully, entry at 0x10078
Would execute ELF at entry point: 0x10078
NOTE: Actual execution requires virtual memory and process isolation
ELF execution completed
```

## Advanced Program Examples

### Fibonacci Calculator

**fibonacci.c** - Computing Fibonacci numbers:
```c
#include "elinos_syscalls.h"  // Your syscall definitions

void print_number(int n) {
    char buffer[32];
    int len = 0;
    
    if (n == 0) {
        buffer[0] = '0';
        len = 1;
    } else {
        // Convert number to string
        int temp = n;
        while (temp > 0) {
            buffer[len++] = '0' + (temp % 10);
            temp /= 10;
        }
        
        // Reverse string
        for (int i = 0; i < len / 2; i++) {
            char t = buffer[i];
            buffer[i] = buffer[len - 1 - i];
            buffer[len - 1 - i] = t;
        }
    }
    
    syscall_write(buffer, len);
    syscall_write("\n", 1);
}

int main(void) {
    syscall_write("Fibonacci sequence:\n", 20);
    
    int a = 0, b = 1;
    print_number(a);
    print_number(b);
    
    for (int i = 0; i < 10; i++) {
        int c = a + b;
        print_number(c);
        a = b;
        b = c;
    }
    
    return 0;
}
```

### System Information Tool

**sysinfo.c** - Displaying system information:
```c
#include "elinos_syscalls.h"

int main(void) {
    syscall_write("elinKernel System Information\n", 26);
    syscall_write("========================\n", 26);
    
    // Get memory information via syscall
    syscall_memory_info();
    
    // Get device information
    syscall_device_info();
    
    // Display version
    syscall_version();
    
    return 0;
}
```

## System Call Interface

elinKernel provides the following system calls for user programs:

### File I/O Operations (1-50)
- `SYS_WRITE (1)` - Write to file descriptor
- `SYS_READ (2)` - Read from file descriptor
- `SYS_OPEN (3)` - Open file
- `SYS_CLOSE (4)` - Close file descriptor
- `SYS_UNLINK (5)` - Delete file

### Process Management (121-170)
- `SYS_EXIT (121)` - Exit process

### Memory Management (71-120)
- `SYS_GETMEMINFO (100)` - Get memory information

### Device Management (171-220)
- `SYS_GETDEVICES (200)` - Get device information

### elinKernel-Specific (900-999)
- `SYS_ELINOS_VERSION (902)` - Get OS version

## Creating System Call Headers

Create a header file for easy system call access:

**elinos_syscalls.h:**
```c
#ifndef ELINOS_SYSCALLS_H
#define ELINOS_SYSCALLS_H

// System call numbers
#define SYS_WRITE 1
#define SYS_READ 2
#define SYS_OPEN 3
#define SYS_CLOSE 4
#define SYS_UNLINK 5
#define SYS_EXIT 121
#define SYS_GETMEMINFO 100
#define SYS_GETDEVICES 200
#define SYS_ELINOS_VERSION 902

// System call wrappers
static inline long syscall_write(const char* msg, int len) {
    register long a0 asm("a0") = 1;
    register long a1 asm("a1") = (long)msg;
    register long a2 asm("a2") = len;
    register long a7 asm("a7") = SYS_WRITE;
    register long result asm("a0");
    
    asm volatile ("ecall"
                  : "=r" (result)
                  : "r" (a0), "r" (a1), "r" (a2), "r" (a7)
                  : "memory");
    return result;
}

static inline void syscall_exit(int status) {
    register long a0 asm("a0") = status;
    register long a7 asm("a7") = SYS_EXIT;
    
    asm volatile ("ecall"
                  :
                  : "r" (a0), "r" (a7)
                  : "memory");
}

static inline long syscall_memory_info(void) {
    register long a7 asm("a7") = SYS_GETMEMINFO;
    register long result asm("a0");
    
    asm volatile ("ecall"
                  : "=r" (result)
                  : "r" (a7)
                  : "memory");
    return result;
}

static inline long syscall_version(void) {
    register long a7 asm("a7") = SYS_ELINOS_VERSION;
    register long result asm("a0");
    
    asm volatile ("ecall"
                  : "=r" (result)
                  : "r" (a7)
                  : "memory");
    return result;
}

// Utility functions
static inline int strlen(const char* str) {
    int len = 0;
    while (str[len]) len++;
    return len;
}

#endif // ELINOS_SYSCALLS_H
```

## Development Workflow

1. **Write C program** using elinKernel system calls
2. **Compile** to RISC-V ELF using appropriate flags
3. **Verify** ELF binary with `readelf` and `objdump`
4. **Add to filesystem** in `src/filesystem.rs`
5. **Rebuild elinKernel** with `./build.sh`
6. **Test in QEMU** using ELF commands

## Current Limitations

⚠️ **Important Notes:**
- **No actual execution yet**: elinKernel can load and parse ELF files, but doesn't execute them yet
- **No virtual memory**: Programs would need proper memory management for execution
- **No process isolation**: Current implementation lacks process context switching
- **Limited syscalls**: Only basic syscalls are implemented

## Roadmap for Full Program Execution

To enable actual program execution, elinKernel needs:

1. **Virtual Memory Management**: Page tables and memory mapping
2. **Process Context**: CPU state management and switching
3. **User/Kernel Mode**: Proper privilege separation
4. **Complete Syscall Interface**: Full POSIX-like system calls
5. **Program Loader**: Copy segments to virtual addresses and jump to entry point

The current ELF loader provides the foundation for all of this!

## Best Practices

### Code Organization
- Use header files for system call interfaces
- Keep programs simple and focused
- Comment your assembly inline code
- Use meaningful function and variable names

### Error Handling
- Check system call return values
- Provide meaningful error messages
- Gracefully handle failures

### Memory Management
- Be aware of stack limitations
- Don't assume large amounts of memory
- Use static allocation where possible

### Testing
- Test with different input sizes
- Verify ELF structure with tools
- Use elinKernel built-in ELF commands for debugging

## Next Steps

- See [Commands](commands.md) for testing your programs
- See [Architecture](architecture.md) for understanding system internals
- See [Debugging](debugging.md) for troubleshooting issues