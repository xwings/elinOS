# Building and Testing C Programs on elinOS

This guide shows how to use the updated Makefile to compile C programs and test them with the elinOS ELF loader.

## Prerequisites

You need a RISC-V cross-compiler toolchain:

```bash
# On Ubuntu/Debian:
sudo apt install gcc-riscv64-unknown-elf

# On Arch Linux:
yay -S riscv64-elf-gcc

# Or download from SiFive/RISC-V official sources
```

## Quick Start

1. **Build everything at once:**
   ```bash
   make all
   ```
   This will:
   - Build the elinOS kernel
   - Compile all C programs
   - Create and populate qcow2 disk image
   - Copy C programs to the disk

2. **Run elinOS with the populated disk:**
   ```bash
   make run-qcow2
   ```

## Step-by-Step Process

### 1. Check Environment
```bash
make env-check
```
This verifies you have:
- Rust toolchain
- RISC-V target
- QEMU
- RISC-V cross-compiler

### 2. Compile C Programs
```bash
make c-programs
```
This compiles all `.c` files in `examples/c_programs/` to RISC-V ELF binaries.

### 3. Show C Program Information
```bash
make c-programs-info
```
This displays information about the compiled binaries.

### 4. Create and Populate Disk
```bash
make prepare-disk
```
This creates a qcow2 image, formats it with FAT32, and copies:
- Test files (`hello.txt`, `test.txt`)
- All compiled C programs to `/programs/` directory

### 5. Run elinOS
```bash
make run-qcow2
```

## Testing ELF Loading in elinOS

Once elinOS is running, you can test the ELF loader:

### List Available Programs
```
elinOS> ls
elinOS> ls programs
```

### Analyze ELF Structure
```
elinOS> elf-info programs/hello_world
```
This shows:
- ELF header information
- Program headers
- Segment permissions
- Entry point

### Load ELF into Memory
```
elinOS> elf-load programs/hello_world
```
This parses and loads the ELF segments (simulated).

### Prepare for Execution
```
elinOS> elf-exec programs/hello_world
```
This prepares the ELF for execution (simulation).

### ELF Demo
```
elinOS> elf-demo
```
Shows available ELF files and loader status.

## Available C Programs

- `hello_world.c` - Direct syscall example
- `file_test.c` - File operation testing
- `test_program.c` - Standard C library example

## Current Limitations

The ELF loader currently:
- ✅ Parses ELF64 headers correctly
- ✅ Validates RISC-V architecture
- ✅ Displays program segments
- ⚠️ **Cannot execute programs yet** (needs virtual memory)
- ⚠️ **No memory copying** (needs implementation)
- ⚠️ **No process isolation** (needs virtual memory)

## Making Programs Executable

To make the C programs actually run, elinOS needs:

1. **Complete ELF Loading** - Copy segments to target addresses
2. **Virtual Memory** - Set up page tables and memory protection
3. **Process Management** - Create execution context
4. **Stack Setup** - Initialize program stack
5. **System Call Interface** - Handle program syscalls

## Troubleshooting

### "Command not found: riscv64-unknown-elf-gcc"
Install the RISC-V cross-compiler toolchain.

### "No ELF files found"
Run `make c-programs` first to compile the C examples.

### "File not found" in elinOS
Make sure you ran `make prepare-disk` to copy files to the disk image.

### ELF analysis fails
Check that the file is a valid RISC-V ELF binary with `file programs/hello_world`.

## Next Steps

1. Implement memory copying in `src/elf.rs`
2. Add virtual memory support
3. Create process execution context
4. Add actual program execution capability

The foundation is there - you can see ELF parsing working and the integration between filesystem, syscalls, and ELF loading! 