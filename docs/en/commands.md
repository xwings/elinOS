# elinOS Shell Commands

This guide covers all available commands in the elinOS interactive shell.

## Overview

Once elinOS boots, you'll have access to an interactive shell with comprehensive commands organized into several categories:

- **System Information** - Inspect system state and configuration
- **Filesystem Operations** - Manage files and directories
- **System Control** - Shutdown, reboot

## System Information Commands

### `help`
Shows available commands with descriptions.

**Usage:**
```
elinOS> help
```

### `version`
Show elinOS version and features.

**Usage:**
```
elinOS> version
```

### `memory`
Display detected memory regions and allocator statistics.

**Usage:**
```
elinOS> memory
```

### `devices`
List detected VirtIO devices.

**Usage:**
```
elinOS> devices
```

### `syscall`
Show system call information (e.g. total count, architecture).

**Usage:**
```
elinOS> syscall
```

### `fscheck`
Check the active filesystem status and superblock/metadata information. Useful for verifying filesystem integrity after operations.

**Usage:**
```
elinOS> fscheck
```

### `config`
Display dynamic system configuration, including detected hardware parameters and kernel settings.

**Usage:**
```
elinOS> config
```

## Filesystem Operations

Note: Most filesystem commands now correctly handle relative and absolute paths. The current working directory is managed internally.

### `ls [path]`
List files and directories. If `[path]` is provided, lists the contents of that path. Otherwise, lists the contents of the current working directory.

**Usage:**
```
elinOS> ls
elinOS> ls /some/directory
elinOS> ls ../another_dir
```

### `cat <path>`
Display file contents from the specified `path`.

**Usage:**
```
elinOS> cat myfile.txt
elinOS> cat /path/to/another_file.txt
```

### `echo [message]`
Prints the specified `[message]` to the console. If no message is provided, it prints a newline.

**Usage:**
```
elinOS> echo Hello World
elinOS> echo
```

### `pwd`
Print the current working directory.

**Usage:**
```
elinOS> pwd
```

### `touch <path>`
Create a new empty file at the specified `path`.

**Usage:**
```
elinOS> touch newfile.txt
elinOS> touch /some/dir/another_new_file.txt
```

### `mkdir <path>`
Create a new directory at the specified `path`.

**Usage:**
```
elinOS> mkdir new_directory
elinOS> mkdir /some/path/another_dir
```

### `rm <path>`
Delete a file at the specified `path`.

**Usage:**
```
elinOS> rm oldfile.txt
elinOS> rm /some/dir/file_to_delete.txt
```

### `rmdir <path>`
Delete an empty directory at the specified `path`.

**Usage:**
```
elinOS> rmdir empty_directory
elinOS> rmdir /some/path/empty_dir_to_remove
```

### `cd [path]`
Change the current working directory. If `[path]` is provided, changes to that path. If no path is provided, or if the path is invalid, it may default to the root directory or print an error. `cd /` changes to the root directory. `cd ..` changes to the parent directory.

**Usage:**
```
elinOS> cd /my/new_directory
elinOS> cd ..
elinOS> cd
```

## System Control Commands

### `shutdown`
Gracefully shut down the system via SBI.

**Usage:**
```
elinOS> shutdown
```

### `reboot`
Reboot the system via SBI.

**Usage:**
```
elinOS> reboot
```

## Example Session

Here's a complete example session showing various commands:

```
elinOS v0.1.0 - RISC-V kernel
Starting interactive shell...

elinOS> help
Available commands:
  help       - Show this help
  memory     - Show memory information
  ext2check  - Check embedded ext2 filesystem
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

elinOS> ext2check
EXT2 Filesystem Check
====================

✅ EXT2 filesystem is active and healthy!

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
✅ Block 0: Contains ext2 superblock at offset 1024
   📊 Magic: 0xef53, Block size: 4096 bytes
   📁 Filesystem: elinOS embedded ext2

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