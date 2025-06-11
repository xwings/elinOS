# C Programs for elinOS

This directory contains C programs designed to test and demonstrate functionality on elinOS, a RISC-V experimental operating system.

## Programs

### 1. `hello_world.c`
- **Purpose**: Simple "Hello World" program using direct system calls
- **Features**: 
  - Uses elinOS `SYS_WRITE` (syscall #64) directly
  - No libc dependencies
  - RISC-V assembly syscall wrapper
- **Tests**: Basic system call interface

### 2. `file_test.c`
- **Purpose**: File operations testing
- **Features**:
  - Opens, reads, and closes files using elinOS syscalls
  - Tests `SYS_OPENAT`, `SYS_READ`, `SYS_CLOSE`
  - Error handling for file operations
- **Tests**: Filesystem integration with elinOS

### 3. `test_program.c` (original)
- **Purpose**: Basic test using standard C library
- **Features**: Uses `write()` from unistd.h
- **Note**: Requires libc support

## Current Status

⚠️ **These programs cannot run on elinOS yet** because several components are missing:

### What Works ✅
- elinOS has system call support (`SYS_WRITE`, `SYS_READ`, etc.)
- ELF parsing and header validation
- RISC-V 64-bit support in ELF loader

### What's Missing ❌
1. **Complete ELF Loading**: 
   - Memory copying from ELF segments to target addresses
   - Virtual memory setup
   - Proper memory mapping

2. **Process Execution**:
   - Setting up execution context
   - Stack initialization
   - Register setup for program entry

3. **Memory Management**:
   - User space memory allocation
   - Process memory protection
   - Stack and heap setup

4. **Runtime Support**:
   - C runtime initialization
   - Static linking support
   - Program exit handling

## How to Test (Future)

Once elinOS ELF loading is complete:

1. **Compile for RISC-V**:
   ```bash
   riscv64-unknown-elf-gcc -static -nostdlib -o hello_world hello_world.c
   ```

2. **Load in elinOS**: 
   - Copy ELF binary to filesystem
   - Use ELF loader to parse and load
   - Execute at entry point

3. **Expected Output**:
   ```
   Hello World from C on elinOS!
   ```

## Architecture Notes

- **Target**: `riscv64gc-unknown-none-elf`
- **System Calls**: Linux-compatible numbers
- **ABI**: RISC-V calling convention
- **Assembly**: Uses `ecall` instruction for system calls

## Next Steps

To make these programs work, you need to:

1. Complete the ELF loader memory copying (fix the TODOs)
2. Add process execution context setup
3. Implement proper memory management for user programs
4. Add C runtime support for static binaries

The programs are designed to be simple and test core OS functionality without complex dependencies. 