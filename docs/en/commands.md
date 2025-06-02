# elinOS Shell Commands

This guide covers all available commands in the elinOS interactive shell.

## Overview

Once elinOS boots, you'll have access to an interactive shell with comprehensive commands organized into several categories:

- **System Information** - Inspect system state and configuration
- **Embedded Filesystem Operations** - Manage files and test ext4 implementation
- **ELF Operations** - Load and analyze ELF binaries
- **System Control** - Shutdown, reboot, and clear screen

## System Information Commands

### `help`
Shows available commands with descriptions.

**Usage:**
```
elinOS> help
```

**Output:**
```
Available commands:
  help       - Show this help
  memory     - Show memory information
  ext4check  - Check embedded ext4 filesystem
  disktest   - Test filesystem operations
  ls         - List files
  cat <file> - Show file contents
  ...
```

### `memory`
Display detected memory regions via `SYS_GETMEMINFO`.

**Usage:**
```
elinOS> memory
```

**Example Output:**
```
Memory regions:
  Region 0: 0x80000000 - 0x88000000 (128 MB) RAM
```

### `ext4check`
Check the embedded ext4 filesystem status and superblock information.

**Usage:**
```
elinOS> ext4check
```

**Example Output:**
```
EXT4 Filesystem Check
====================

✅ EXT4 filesystem is active and healthy!

📊 Superblock Information:
   Magic: 0xef53 ✅
   Inodes: 65536
   Blocks: 65536
   Block size: 4096 bytes
   Volume: elinOS
```

### `disktest`
Test filesystem operations including initialization, file listing, and reading.

**Usage:**
```
elinOS> disktest
```

**Example Output:**
```
Filesystem Test
==============

📋 Testing filesystem operations...

1. Filesystem status... ✅ Initialized
2. File listing... ✅ Success (3 files)
3. File reading... ✅ Success

🎉 Filesystem test complete!
```

### `diskdump [block_num]`
Display information about filesystem blocks (educational purposes).

**Usage:**
```
elinOS> diskdump 0
elinOS> diskdump 5
```

**Example Output:**
```
Filesystem Block Dump
====================

📖 Reading block 0 from embedded filesystem...
✅ Block 0: Contains ext4 superblock at offset 1024
   📊 Magic: 0xef53, Block size: 4096 bytes
   📁 Filesystem: elinOS embedded ext4
```

### `syscall`
Show system call information and architecture.

**Usage:**
```
elinOS> syscall
```

**Output:**
```
System Call Information:
  This shell uses categorized system calls for all kernel operations!

Currently Implemented System Calls:
  File I/O Operations:
    SYS_WRITE (1)     - Write to file descriptor
    SYS_READ (2)      - Read from file descriptor [TODO]
    SYS_OPEN (3)      - Open file
    ...
```

### `categories`
Show syscall categorization system.

**Usage:**
```
elinOS> categories
```

**Output:**
```
System Call Categories:
  1-50:   File I/O Operations
  51-70:  Directory Operations
  71-120: Memory Management
  121-170: Process Management
  ...
```

### `version`
Show elinOS version via `SYS_ELINOS_VERSION`.

**Usage:**
```
elinOS> version
```

**Output:**
```
elinOS v0.1.0 - RISC-V kernel
Built with Rust and proper syscall architecture
Organized syscalls inspired by Qiling framework
```

## Embedded Filesystem Operations

### `ls`
List all files with sizes using `SYS_GETDENTS`.

**Usage:**
```
elinOS> ls
```

**Example Output:**
```
Files:
  hello.txt (28 bytes)
  readme.md (45 bytes)
  lost+found (0 bytes)
```

### `cat <filename>`
Display file contents using `SYS_OPEN`.

**Usage:**
```
elinOS> cat hello.txt
```

**Example Output:**
```
Contents of hello.txt:
Hello from elinOS filesystem!
--- End of file ---
```

### `touch <filename>`
Create a new empty file using filesystem + `SYS_OPEN`.

**Usage:**
```
elinOS> touch newfile.txt
```

**Output:**
```
File 'newfile.txt' created successfully.
```

### `rm <filename>`
Delete a file using `SYS_UNLINK`.

**Usage:**
```
elinOS> rm oldfile.txt
```

**Output:**
```
File 'oldfile.txt' deleted successfully.
```

## ELF Operations

### `elf-info <filename>`
Analyze ELF binary structure and display detailed information.

**Usage:**
```
elinOS> elf-info hello.elf
```

**Example Output:**
```
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
```

### `elf-load <filename>`
Load ELF binary into memory and show entry point/segments.

**Usage:**
```
elinOS> elf-load hello.elf
```

**Example Output:**
```
Loading ELF binary: hello.elf
Loading ELF binary:
  Entry point: 0x10078
  Program headers: 2
  Segment 0: 0x10000 - 0x11000 (4096 bytes) flags: 0x5
  Segment 1: 0x11000 - 0x12000 (4096 bytes) flags: 0x6
ELF loaded successfully, entry at 0x10078
ELF binary loaded successfully!
Entry point: 0x10078
```

### `elf-exec <filename>`
Load ELF binary and prepare for execution (simulated).

**Usage:**
```
elinOS> elf-exec hello.elf
```

**Example Output:**
```
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

### `elf-demo`
Built-in demonstration with sample ELF header.

**Usage:**
```
elinOS> elf-demo
```

**Example Output:**
```
ELF Loader Demo
================

Testing ELF header parsing with demo binary...

ELF Binary Information:
  Class: ELF64
  Data: Little-endian
  Machine: RISC-V
  Type: Executable
  Entry point: 0x10000
  Program header offset: 0x40
  Program header count: 1
  Section header offset: 0x0
  Section header count: 0

Demo ELF header parsed successfully!
Note: This is just a header demo - no actual code segments.
```

## System Control

### `shutdown`
Gracefully shutdown elinOS and exit QEMU using `SYS_ELINOS_SHUTDOWN`.

**Usage:**
```
elinOS> shutdown
```

**Output:**
```
elinOS shutting down...
Goodbye!
# Returns to host shell automatically
```

### `reboot`
Restart the system using `SYS_ELINOS_REBOOT`.

**Usage:**
```
elinOS> reboot
```

**Output:**
```
elinOS rebooting...
# System restarts
```

### `clear`
Clear the screen using `SYS_WRITE`.

**Usage:**
```
elinOS> clear
```

**Effect:**
Clears the terminal screen and positions cursor at top.

## Example Session

Here's a complete example session showing various commands:

```
elinOS v0.1.0 - RISC-V kernel
Starting interactive shell...

elinOS> help
Available commands:
  help       - Show this help
  memory     - Show memory information
  ext4check  - Check embedded ext4 filesystem
  disktest   - Test filesystem operations
  ls         - List files
  cat <file> - Show file contents
  touch <file> - Create empty file
  rm <file>  - Delete file
  clear      - Clear screen
  syscall    - Show system call info
  categories - Show syscall categories
  version    - Show elinOS version
  elf-info <file>  - Show ELF binary information
  elf-load <file>  - Load ELF binary into memory
  elf-exec <file>  - Execute ELF binary (simulated)
  elf-demo   - ELF loader demonstration
  shutdown   - Shutdown the system
  reboot     - Reboot the system

elinOS> version
elinOS v0.1.0 - RISC-V kernel
Built with Rust and proper syscall architecture
Organized syscalls inspired by Qiling framework

elinOS> memory
Memory regions:
  Region 0: 0x80000000 - 0x88000000 (128 MB) RAM

elinOS> ls
Files:
  hello.txt (28 bytes)
  readme.md (45 bytes)
  lost+found (0 bytes)

elinOS> cat hello.txt
Contents of hello.txt:
Hello from elinOS filesystem!
--- End of file ---

elinOS> elf-info hello.elf
ELF Binary Information:
  Class: ELF64
  Data: Little-endian
  Machine: RISC-V
  Type: Executable
  Entry point: 0x10000
  Program header offset: 0x40
  Program header count: 1
  Section header offset: 0x0
  Section header count: 0

elinOS> ext4check
EXT4 Filesystem Check
====================

✅ EXT4 filesystem is active and healthy!

📊 Superblock Information:
   Magic: 0xef53 ✅
   Inodes: 65536
   Blocks: 65536
   Block size: 4096 bytes
   Volume: elinOS

elinOS> disktest
Filesystem Test
==============

📋 Testing filesystem operations...

1. Filesystem status... ✅ Initialized
2. File listing... ✅ Success (3 files)
3. File reading... ✅ Success

🎉 Filesystem test complete!

elinOS> diskdump 0
Filesystem Block Dump
====================

📖 Reading block 0 from embedded filesystem...
✅ Block 0: Contains ext4 superblock at offset 1024
   📊 Magic: 0xef53, Block size: 4096 bytes
   📁 Filesystem: elinOS embedded ext4

elinOS> syscall
System Call Information:
  This shell uses categorized system calls for all kernel operations!

Currently Implemented System Calls:
  File I/O Operations:
    SYS_WRITE (1)     - Write to file descriptor
    SYS_READ (2)      - Read from file descriptor [TODO]
    SYS_OPEN (3)      - Open file
    ...

elinOS> shutdown
elinOS shutting down...
Goodbye!
```

## Command Implementation

All commands are implemented as user-space programs that make system calls to the kernel. The shell:

1. **Parses** user input
2. **Dispatches** to appropriate command function
3. **Executes** command using system calls
4. **Reports** results to user

## Error Handling

Commands provide helpful error messages:

- **File not found**: `Error: File 'nonexistent.txt' not found`
- **Invalid arguments**: `Usage: cat <filename>`
- **System errors**: `Error: Failed to open file`

## Adding New Commands

To add a new command:

1. **Implement** command function in `src/commands.rs`
2. **Add** to command list in `process_command()`
3. **Update** help text
4. **Rebuild** and test

See [Development Guide](development.md) for details.

## Next Steps

- See [Architecture](architecture.md) for system call implementation details
- See [Development](development.md) for creating user programs
- See [Debugging](debugging.md) for troubleshooting commands 