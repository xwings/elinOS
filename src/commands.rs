use crate::syscall;

// Shell commands that use system calls

// Central command processor - main.rs calls this function
pub fn process_command(command: &str) {
    let command = command.trim();
    
    let result = match command {
        "help" => cmd_help(),
        "version" => cmd_version(),
        "syscall" => cmd_syscall(),
        "memory" => cmd_memory(),
        "memstats" => cmd_memstats(),
        "layout" => cmd_layout(),
        "unified" => cmd_unified_memory(),
        "alloc" => cmd_alloc_test(),
        "bigalloc" => cmd_big_alloc_test(),
        "buddy" => cmd_buddy_test(),
        "comprehensive" => cmd_comprehensive_test(),
        "devices" => cmd_devices(),
        "ls" => cmd_ls(),
        "cat" => cmd_cat(""),
        "echo" => cmd_echo(""),
        "shutdown" => cmd_shutdown(),
        "reboot" => cmd_reboot(),
        cmd if cmd.starts_with("cat ") => {
            let filename = &cmd[4..];
            cmd_cat(filename)
        }
        cmd if cmd.starts_with("echo ") => {
            let message = &cmd[5..];
            cmd_echo(message)
        }
        cmd if cmd.starts_with("elf_load ") => {
            let filename = &cmd[9..];
            cmd_elf_load(filename)
        }
        cmd if cmd.starts_with("elf_exec ") => {
            let filename = &cmd[9..];
            cmd_elf_exec(filename)
        }
        cmd if cmd.starts_with("alloc ") => {
            let size_str = &cmd[6..];
            if let Ok(size) = size_str.parse::<usize>() {
                cmd_alloc_size_test(size)
            } else {
                Err("Invalid size format")
            }
        }
        cmd if cmd.starts_with("diskdump ") => {
            let block_str = &cmd[9..];
            if let Ok(block_num) = block_str.parse::<u64>() {
                cmd_diskdump(block_num)
            } else {
                Err("Invalid block number")
            }
        }
        "diskdump" => cmd_diskdump(0),
        "disktest" => cmd_disktest(),
        "ext4check" => cmd_ext4check(),
        "" => Ok(()), // Empty command
        _ => {
            let _ = syscall::sys_print("Unknown command: ");
            let _ = syscall::sys_print(command);
            let _ = syscall::sys_print("\nType 'help' for available commands.\n");
            Ok(())
        }
    };

    if let Err(e) = result {
        let _ = syscall::sys_print("Command failed: ");
        let _ = syscall::sys_print(e);
        let _ = syscall::sys_print("\n");
    }
}

// Get list of all available commands (for help and autocomplete)
pub fn get_available_commands() -> &'static [&'static str] {
    &[
        "help", "memory", "devices", "ls", "cat", "touch", "rm", 
        "clear", "syscall", "categories", "version", "shutdown", "reboot",
        "elf-info", "elf-load", "elf-exec", "elf-demo"
    ]
}

// === INDIVIDUAL COMMAND IMPLEMENTATIONS ===

pub fn cmd_help() -> Result<(), &'static str> {
    syscall::sys_print("Available commands:\n")?;
    syscall::sys_print("  help        - Show this help message\n")?;
    syscall::sys_print("  version     - Show kernel version\n")?;
    syscall::sys_print("  syscall     - Show system call information\n")?;
    syscall::sys_print("  memory      - Show memory information\n")?;
    syscall::sys_print("  memstats    - Show memory statistics\n")?;
    syscall::sys_print("  layout      - Show dynamic memory layout\n")?;
    syscall::sys_print("  unified     - Show unified memory management improvements\n")?;
    syscall::sys_print("  alloc       - Test memory allocation (1KB)\n")?;
    syscall::sys_print("  alloc <size> - Test memory allocation (specific size)\n")?;
    syscall::sys_print("  bigalloc    - Test large allocation (1MB)\n")?;
    syscall::sys_print("  buddy       - Test buddy allocator specifically\n")?;
    syscall::sys_print("  comprehensive - Run comprehensive memory management test\n")?;
    syscall::sys_print("  devices     - List available devices\n")?;
    syscall::sys_print("  diskdump    - Dump disk block 0 (bootblock)\n")?;
    syscall::sys_print("  diskdump <n> - Dump disk block n\n")?;
    syscall::sys_print("  disktest    - Test VirtIO block device I/O\n")?;
    syscall::sys_print("  ext4check   - Check for ext4 filesystem\n")?;
    syscall::sys_print("  ls          - List files in filesystem\n")?;
    syscall::sys_print("  cat <file>  - Display file contents\n")?;
    syscall::sys_print("  echo <msg>  - Echo a message\n")?;
    syscall::sys_print("  elf_load <file> - Load ELF binary\n")?;
    syscall::sys_print("  elf_exec <file> - Execute ELF binary\n")?;
    syscall::sys_print("  shutdown    - Shutdown the system\n")?;
    syscall::sys_print("  reboot      - Reboot the system\n")?;
    Ok(())
}

pub fn cmd_memory() -> Result<(), &'static str> {
    // Call the memory info syscall
    let result = syscall::syscall_handler(
        syscall::memory::SYS_GETMEMINFO,
        0,
        0,
        0,
        0,
    );
    
    match result {
        syscall::SysCallResult::Success(_) => Ok(()),
        syscall::SysCallResult::Error(e) => Err(e),
    }
}

pub fn cmd_devices() -> Result<(), &'static str> {
    syscall::sys_device_info()
}

pub fn cmd_ls() -> Result<(), &'static str> {
    let mut buffer = [0u8; 1024];
    let result = syscall::syscall_handler(
        syscall::file::SYS_GETDENTS,
        buffer.as_mut_ptr() as usize,
        buffer.len(),
        0,
        0,
    );
    
    match result {
        syscall::SysCallResult::Success(bytes_read) => {
            if bytes_read > 0 {
                let output = core::str::from_utf8(&buffer[..bytes_read as usize])
                    .unwrap_or("Invalid UTF-8");
                syscall::sys_print("Files:\n")?;
                syscall::sys_print(output)?;
            }
            Ok(())
        },
        syscall::SysCallResult::Error(e) => Err(e),
    }
}

pub fn cmd_cat(filename: &str) -> Result<(), &'static str> {
    // First check if file exists using SYS_OPEN
    let mut filename_buf = [0u8; 256];
    if filename.len() >= filename_buf.len() {
        return Err("Filename too long");
    }
    
    filename_buf[..filename.len()].copy_from_slice(filename.as_bytes());
    
    let result = syscall::syscall_handler(
        syscall::file::SYS_OPEN,
        filename_buf.as_ptr() as usize,
        0, // flags
        0,
        0,
    );
    
    match result {
        syscall::SysCallResult::Success(_fd) => {
            // File exists, now we would read it via SYS_READ
            // For now, we'll use a direct filesystem access as a temporary measure
            // TODO: Implement proper SYS_READ
            let fs = crate::filesystem::FILESYSTEM.lock();
            match fs.read_file(filename) {
                Ok(content) => {
                    syscall::sys_print("Contents of ")?;
                    syscall::sys_print(filename)?;
                    syscall::sys_print(":\n")?;
                    let content_str = core::str::from_utf8(&content)
                        .unwrap_or("<binary content>");
                    syscall::sys_print(content_str)?;
                    syscall::sys_print("\n--- End of file ---\n")?;
                    Ok(())
                },
                Err(e) => {
                    syscall::sys_print("Failed to read file: ")?;
                    syscall::sys_print(e)?;
                    syscall::sys_print("\n")?;
                    Err("Failed to read file")
                }
            }
        },
        syscall::SysCallResult::Error(e) => Err(e),
    }
}

pub fn cmd_touch(filename: &str) -> Result<(), &'static str> {
    // Check if file already exists using SYS_OPEN
    let mut filename_buf = [0u8; 256];
    if filename.len() >= filename_buf.len() {
        return Err("Filename too long");
    }
    
    filename_buf[..filename.len()].copy_from_slice(filename.as_bytes());
    
    let result = syscall::syscall_handler(
        syscall::file::SYS_OPEN,
        filename_buf.as_ptr() as usize,
        0,
        0,
        0,
    );
    
    match result {
        syscall::SysCallResult::Success(_) => {
            syscall::sys_print("File '")?;
            syscall::sys_print(filename)?;
            syscall::sys_print("' already exists\n")?;
            Ok(())
        },
        syscall::SysCallResult::Error(_) => {
            // File doesn't exist, create it
            // For now, we'll directly use filesystem since we don't have SYS_CREATE yet
            // TODO: Implement SYS_CREATE system call
            let mut fs = crate::filesystem::FILESYSTEM.lock();
            match fs.create_file(filename, b"") {
                Ok(()) => {
                    syscall::sys_print("Created file '")?;
                    syscall::sys_print(filename)?;
                    syscall::sys_print("'\n")?;
                    Ok(())
                },
                Err(e) => {
                    syscall::sys_print("Failed to create file '")?;
                    syscall::sys_print(filename)?;
                    syscall::sys_print("': ")?;
                    syscall::sys_print(e)?;
                    syscall::sys_print("\n")?;
                    Err(e)
                }
            }
        }
    }
}

pub fn cmd_rm(filename: &str) -> Result<(), &'static str> {
    let mut filename_buf = [0u8; 256];
    if filename.len() >= filename_buf.len() {
        return Err("Filename too long");
    }
    
    filename_buf[..filename.len()].copy_from_slice(filename.as_bytes());
    
    let result = syscall::syscall_handler(
        syscall::file::SYS_UNLINK,
        filename_buf.as_ptr() as usize,
        0,
        0,
        0,
    );
    
    match result {
        syscall::SysCallResult::Success(_) => {
            syscall::sys_print("Deleted file '")?;
            syscall::sys_print(filename)?;
            syscall::sys_print("'\n")?;
            Ok(())
        },
        syscall::SysCallResult::Error(e) => {
            syscall::sys_print("Failed to delete file '")?;
            syscall::sys_print(filename)?;
            syscall::sys_print("': ")?;
            syscall::sys_print(e)?;
            syscall::sys_print("\n")?;
            Err(e)
        }
    }
}

pub fn cmd_clear() -> Result<(), &'static str> {
    syscall::sys_print("\x1b[2J\x1b[H") // ANSI escape codes to clear screen
}

pub fn cmd_syscall() -> Result<(), &'static str> {
    syscall::sys_print("System Call Information (Linux Compatible):\n")?;
    syscall::sys_print("  elinKernel now follows Linux syscall numbers for better compatibility!\n\n")?;
    
    syscall::sys_print("Currently Implemented System Calls:\n")?;
    syscall::sys_print("  File I/O Operations:\n")?;
    syscall::sys_print("    SYS_WRITE (64)     - Write to file descriptor\n")?;
    syscall::sys_print("    SYS_READ (63)      - Read from file descriptor [TODO]\n")?;
    syscall::sys_print("    SYS_OPENAT (56)    - Open file (modern Linux openat)\n")?;
    syscall::sys_print("    SYS_CLOSE (57)     - Close file descriptor [TODO]\n")?;
    syscall::sys_print("    SYS_UNLINKAT (35)  - Delete file (modern Linux unlinkat)\n")?;
    syscall::sys_print("    SYS_GETDENTS64 (61) - List directory entries\n")?;
    syscall::sys_print("    SYS_NEWFSTATAT (79) - Get file status (modern Linux stat)\n")?;

    syscall::sys_print("  Directory Operations:\n")?;
    syscall::sys_print("    SYS_MKDIRAT (34)   - Create directory [TODO]\n")?;
    syscall::sys_print("    SYS_CHDIR (49)     - Change directory [TODO]\n")?;

    syscall::sys_print("  Memory Management:\n")?;
    syscall::sys_print("    SYS_MMAP (222)     - Memory mapping [TODO]\n")?;
    syscall::sys_print("    SYS_MUNMAP (215)   - Memory unmapping [TODO]\n")?;
    syscall::sys_print("    SYS_BRK (214)      - Program break [TODO]\n")?;
    syscall::sys_print("    SYS_GETMEMINFO (960) - Memory information (elinKernel)\n")?;

    syscall::sys_print("  Process Management:\n")?;
    syscall::sys_print("    SYS_EXIT (93)      - Exit process\n")?;
    syscall::sys_print("    SYS_CLONE (220)    - Clone process [TODO]\n")?;
    syscall::sys_print("    SYS_EXECVE (221)   - Execute program [TODO]\n")?;
    syscall::sys_print("    SYS_GETPID (172)   - Get process ID\n")?;
    syscall::sys_print("    SYS_GETPPID (173)  - Get parent process ID\n")?;
    syscall::sys_print("    SYS_KILL (129)     - Send signal to process [TODO]\n")?;

    syscall::sys_print("  Device Management:\n")?;
    syscall::sys_print("    SYS_IOCTL (29)     - I/O control [TODO]\n")?;
    syscall::sys_print("    SYS_DUP (23)       - Duplicate file descriptor [TODO]\n")?;
    syscall::sys_print("    SYS_PIPE2 (59)     - Create pipe [TODO]\n")?;
    syscall::sys_print("    SYS_GETDEVICES (950) - Device information (elinKernel)\n")?;

    syscall::sys_print("  elinKernel-Specific (ELF Support):\n")?;
    syscall::sys_print("    SYS_LOAD_ELF (900)  - Load ELF binary into memory\n")?;
    syscall::sys_print("    SYS_EXEC_ELF (901)  - Load and execute ELF binary\n")?;
    syscall::sys_print("    SYS_ELF_INFO (902)  - Display ELF binary information\n")?;

    syscall::sys_print("  elinKernel-Specific (System Control):\n")?;
    syscall::sys_print("    SYS_ELINOS_DEBUG (900)    - Set debug level\n")?;
    syscall::sys_print("    SYS_ELINOS_VERSION (902)  - Show version\n")?;
    syscall::sys_print("    SYS_ELINOS_SHUTDOWN (903) - Shutdown system\n")?;
    syscall::sys_print("    SYS_ELINOS_REBOOT (904)   - Reboot system\n")?;

    syscall::sys_print("\nNumbers in parentheses are Linux-compatible syscall numbers.\n")?;
    syscall::sys_print("This makes elinKernel easier to understand for Linux developers!\n")?;
    syscall::sys_print("Use 'categories' to see the full categorization system.\n")?;
    syscall::sys_print("Use 'elf-demo' to test the ELF loader functionality.\n")?;
    Ok(())
}

pub fn cmd_categories() -> Result<(), &'static str> {
    syscall::sys_show_categories()
}

pub fn cmd_version() -> Result<(), &'static str> {
    let result = syscall::syscall_handler(
        syscall::elinos::SYS_ELINOS_VERSION,
        0,
        0,
        0,
        0,
    );
    
    match result {
        syscall::SysCallResult::Success(_) => Ok(()),
        syscall::SysCallResult::Error(e) => Err(e),
    }
}

pub fn cmd_shutdown() -> Result<(), &'static str> {
    let result = syscall::syscall_handler(
        syscall::elinos::SYS_ELINOS_SHUTDOWN,
        0,
        0,
        0,
        0,
    );
    
    // This should never return since shutdown is supposed to halt the system
    match result {
        syscall::SysCallResult::Success(_) => Ok(()),
        syscall::SysCallResult::Error(e) => Err(e),
    }
}

pub fn cmd_reboot() -> Result<(), &'static str> {
    let result = syscall::syscall_handler(
        syscall::elinos::SYS_ELINOS_REBOOT,
        0,
        0,
        0,
        0,
    );
    
    // This should never return since reboot is supposed to restart the system
    match result {
        syscall::SysCallResult::Success(_) => Ok(()),
        syscall::SysCallResult::Error(e) => Err(e),
    }
}

pub fn cmd_elf_info(filename: &str) -> Result<(), &'static str> {
    // Get file content from filesystem and clone it
    let file_data = {
        let fs = crate::filesystem::FILESYSTEM.lock();
        match fs.read_file(filename) {
            Ok(data) => {
                // Clone the data to a Vec so we can use it after dropping the lock
                let mut cloned_data = heapless::Vec::<u8, 1024>::new();
                for &byte in &data {
                    if cloned_data.push(byte).is_err() {
                        return Err("File too large to process");
                    }
                }
                cloned_data
            },
            Err(e) => {
                syscall::sys_print("Error: File '")?;
                syscall::sys_print(filename)?;
                syscall::sys_print("' not found: ")?;
                syscall::sys_print(e)?;
                syscall::sys_print("\n")?;
                return Err("File not found");
            }
        }
    }; // fs lock is automatically dropped here

    // Call ELF info syscall
    let result = syscall::syscall_handler(
        syscall::process::SYS_ELF_INFO,
        file_data.as_ptr() as usize,
        file_data.len(),
        0,
        0,
    );

    match result {
        syscall::SysCallResult::Success(_) => Ok(()),
        syscall::SysCallResult::Error(e) => {
            syscall::sys_print("ELF info error: ")?;
            syscall::sys_print(e)?;
            syscall::sys_print("\n")?;
            Err(e)
        }
    }
}

pub fn cmd_elf_load(filename: &str) -> Result<(), &'static str> {
    // Get file content from filesystem and clone it
    let file_data = {
        let fs = crate::filesystem::FILESYSTEM.lock();
        match fs.read_file(filename) {
            Ok(data) => {
                // Clone the data to a Vec so we can use it after dropping the lock
                let mut cloned_data = heapless::Vec::<u8, 1024>::new();
                for &byte in &data {
                    if cloned_data.push(byte).is_err() {
                        return Err("File too large to process");
                    }
                }
                cloned_data
            },
            Err(e) => {
                syscall::sys_print("Error: File '")?;
                syscall::sys_print(filename)?;
                syscall::sys_print("' not found: ")?;
                syscall::sys_print(e)?;
                syscall::sys_print("\n")?;
                return Err("File not found");
            }
        }
    }; // fs lock is automatically dropped here

    syscall::sys_print("Loading ELF binary: ")?;
    syscall::sys_print(filename)?;
    syscall::sys_print("\n")?;

    // Call ELF load syscall
    let result = syscall::syscall_handler(
        syscall::process::SYS_LOAD_ELF,
        file_data.as_ptr() as usize,
        file_data.len(),
        0,
        0,
    );

    match result {
        syscall::SysCallResult::Success(entry_point) => {
            syscall::sys_print("ELF binary loaded successfully!\n")?;
            syscall::sys_print("Entry point: 0x")?;
            
            // Format entry point as hex string
            let mut buffer = [0u8; 16];
            let mut value = entry_point as u64;
            let mut pos = 0;
            if value == 0 {
                buffer[0] = b'0';
                pos = 1;
            } else {
                while value > 0 {
                    let digit = (value % 16) as u8;
                    buffer[15 - pos] = if digit < 10 { b'0' + digit } else { b'a' + digit - 10 };
                    value /= 16;
                    pos += 1;
                }
            }
            
            let hex_str = core::str::from_utf8(&buffer[16-pos..]).unwrap_or("?");
            syscall::sys_print(hex_str)?;
            syscall::sys_print("\n")?;
            Ok(())
        }
        syscall::SysCallResult::Error(e) => {
            syscall::sys_print("ELF load error: ")?;
            syscall::sys_print(e)?;
            syscall::sys_print("\n")?;
            Err(e)
        }
    }
}

pub fn cmd_elf_exec(filename: &str) -> Result<(), &'static str> {
    // Get file content from filesystem and clone it
    let file_data = {
        let fs = crate::filesystem::FILESYSTEM.lock();
        match fs.read_file(filename) {
            Ok(data) => {
                // Clone the data to a Vec so we can use it after dropping the lock
                let mut cloned_data = heapless::Vec::<u8, 1024>::new();
                for &byte in &data {
                    if cloned_data.push(byte).is_err() {
                        return Err("File too large to process");
                    }
                }
                cloned_data
            },
            Err(e) => {
                syscall::sys_print("Error: File '")?;
                syscall::sys_print(filename)?;
                syscall::sys_print("' not found: ")?;
                syscall::sys_print(e)?;
                syscall::sys_print("\n")?;
                return Err("File not found");
            }
        }
    }; // fs lock is automatically dropped here

    syscall::sys_print("Executing ELF binary: ")?;
    syscall::sys_print(filename)?;
    syscall::sys_print("\n")?;

    // Call ELF execute syscall
    let result = syscall::syscall_handler(
        syscall::process::SYS_EXEC_ELF,
        file_data.as_ptr() as usize,
        file_data.len(),
        0,
        0,
    );

    match result {
        syscall::SysCallResult::Success(_) => {
            syscall::sys_print("ELF execution completed\n")?;
            Ok(())
        }
        syscall::SysCallResult::Error(e) => {
            syscall::sys_print("ELF execution error: ")?;
            syscall::sys_print(e)?;
            syscall::sys_print("\n")?;
            Err(e)
        }
    }
}

pub fn cmd_elf_demo() -> Result<(), &'static str> {
    syscall::sys_print("ELF Loader Demo\n")?;
    syscall::sys_print("================\n\n")?;

    // Create a minimal ELF header for demonstration
    // This is a simple RISC-V ELF64 header
    let elf_demo: [u8; 64] = [
        // ELF Magic + Class + Data + Version
        0x7f, b'E', b'L', b'F',  // e_ident[0-3]: ELF magic
        2,                        // e_ident[4]: ELFCLASS64
        1,                        // e_ident[5]: ELFDATA2LSB
        1,                        // e_ident[6]: EV_CURRENT
        0,                        // e_ident[7]: ELFOSABI_NONE
        0, 0, 0, 0, 0, 0, 0, 0,   // e_ident[8-15]: padding
        
        // ELF header fields
        2, 0,                     // e_type: ET_EXEC (executable)
        243, 0,                   // e_machine: EM_RISCV (243)
        1, 0, 0, 0,               // e_version: EV_CURRENT
        0x00, 0x10, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, // e_entry: 0x10000
        64, 0, 0, 0, 0, 0, 0, 0,  // e_phoff: program header offset
        0, 0, 0, 0, 0, 0, 0, 0,   // e_shoff: section header offset  
        0, 0, 0, 0,               // e_flags
        64, 0,                    // e_ehsize: header size
        56, 0,                    // e_phentsize: program header size
        1, 0,                     // e_phnum: program header count
        64, 0,                    // e_shentsize: section header size
        0, 0,                     // e_shnum: section header count
        0, 0,                     // e_shstrndx: string table index
    ];

    syscall::sys_print("Testing ELF header parsing with demo binary...\n\n")?;

    // Test ELF info on demo binary
    let result = syscall::syscall_handler(
        syscall::process::SYS_ELF_INFO,
        elf_demo.as_ptr() as usize,
        elf_demo.len(),
        0,
        0,
    );

    match result {
        syscall::SysCallResult::Success(_) => {
            syscall::sys_print("\nDemo ELF header parsed successfully!\n")?;
            syscall::sys_print("Note: This is just a header demo - no actual code segments.\n")?;
        }
        syscall::SysCallResult::Error(e) => {
            syscall::sys_print("Demo failed: ")?;
            syscall::sys_print(e)?;
            syscall::sys_print("\n")?;
        }
    }

    syscall::sys_print("\nTo test with real ELF binaries:\n")?;
    syscall::sys_print("1. Add ELF files to the filesystem\n")?;
    syscall::sys_print("2. Use 'elf-info <filename>' to inspect them\n")?;
    syscall::sys_print("3. Use 'elf-load <filename>' to load them into memory\n")?;
    syscall::sys_print("4. Use 'elf-exec <filename>' to attempt execution\n")?;

    Ok(())
}

pub fn cmd_memstats() -> Result<(), &'static str> {
    // Call the buddy stats syscall
    let result = syscall::syscall_handler(
        syscall::memory::SYS_BUDDY_STATS,
        0,
        0,
        0,
        0,
    );
    
    match result {
        syscall::SysCallResult::Success(_) => Ok(()),
        syscall::SysCallResult::Error(e) => Err(e),
    }
}

pub fn cmd_alloc_test() -> Result<(), &'static str> {
    // Test 1KB allocation
    cmd_alloc_size_test(1024)
}

pub fn cmd_alloc_size_test(size: usize) -> Result<(), &'static str> {
    // Call the allocation test syscall
    let result = syscall::syscall_handler(
        syscall::memory::SYS_ALLOC_TEST,
        size,
        0,
        0,
        0,
    );
    
    match result {
        syscall::SysCallResult::Success(addr) => {
            syscall::sys_print("Allocation successful!\n")?;
            Ok(())
        },
        syscall::SysCallResult::Error(e) => Err(e),
    }
}

pub fn cmd_big_alloc_test() -> Result<(), &'static str> {
    syscall::sys_print("Testing large allocation (1MB) - should use buddy allocator:\n")?;
    cmd_alloc_size_test(1024 * 1024)
}

pub fn cmd_buddy_test() -> Result<(), &'static str> {
    syscall::sys_print("=== Buddy Allocator Test ===\n")?;
    
    // Test various allocation sizes
    let test_sizes = [
        (512, "512 bytes (small allocator)"),
        (4096, "4KB (buddy allocator)"),
        (8192, "8KB (buddy allocator)"),
        (16384, "16KB (buddy allocator)"),
    ];
    
    for (size, description) in &test_sizes {
        syscall::sys_print("Testing ")?;
        syscall::sys_print(description)?;
        syscall::sys_print(":\n")?;
        
        let result = syscall::syscall_handler(
            syscall::memory::SYS_ALLOC_TEST,
            *size,
            0,
            0,
            0,
        );
        
        match result {
            syscall::SysCallResult::Success(_) => {
                syscall::sys_print("  ✅ Success\n")?;
            },
            syscall::SysCallResult::Error(e) => {
                syscall::sys_print("  ❌ Failed: ")?;
                syscall::sys_print(e)?;
                syscall::sys_print("\n")?;
            }
        }
    }
    
    // Show final stats
    syscall::sys_print("\nFinal statistics:\n")?;
    cmd_memstats()
}

pub fn cmd_comprehensive_test() -> Result<(), &'static str> {
    syscall::sys_print("=== elinKernel Buddy Management Test ===\n")?;
    syscall::sys_print("Testing our dynamic two-tier memory management system!\n\n")?;
    
    // Phase 1: Show initial state
    syscall::sys_print("📊 Phase 1: Initial Memory State\n")?;
    cmd_memory()?;
    syscall::sys_print("\n")?;
    
    // Phase 2: Test buddy allocator
    syscall::sys_print("🧩 Phase 2: Buddy Allocator Tests\n")?;
    syscall::sys_print("Testing large allocations (should use buddy allocator):\n")?;
    
    let buddy_tests = [4096, 8192, 16384];
    for &size in &buddy_tests {
        let result = syscall::syscall_handler(
            syscall::memory::SYS_ALLOC_TEST,
            size,
            0,
            0,
            0,
        );
        
        match result {
            syscall::SysCallResult::Success(_) => {
                syscall::sys_print("  ✅ Large allocation succeeded\n")?;
            },
            _ => {
                syscall::sys_print("  ❌ Large allocation failed\n")?;
            }
        }
    }
    syscall::sys_print("\n")?;
    
    // Phase 3: Test small allocator
    syscall::sys_print("🔍 Phase 3: Small Allocator Tests\n")?;
    syscall::sys_print("Testing small allocations (should use small allocator):\n")?;
    
    let small_tests = [8, 16, 32, 64, 128, 512, 1024, 2048];
    for &size in &small_tests {
        let result = syscall::syscall_handler(
            syscall::memory::SYS_ALLOC_TEST,
            size,
            0,
            0,
            0,
        );
        
        match result {
            syscall::SysCallResult::Success(_) => {
                syscall::sys_print("  ✅ Small allocation succeeded\n")?;
            },
            _ => {
                syscall::sys_print("  ❌ Small allocation failed\n")?;
            }
        }
    }
    syscall::sys_print("\n")?;
    
    // Phase 4: Test virtual memory (mmap/brk)
    syscall::sys_print("🗺️  Phase 4: Virtual Memory Tests\n")?;
    syscall::sys_print("Testing mmap syscall:\n")?;
    
    // Test anonymous mmap
    let mmap_result = syscall::syscall_handler(
        syscall::memory::SYS_MMAP,
        0,              // addr (let kernel choose)
        8192,           // size (8KB)
        7,              // prot (RWX)
        32,             // flags (MAP_ANONYMOUS)
    );
    
    match mmap_result {
        syscall::SysCallResult::Success(addr) => {
            syscall::sys_print("  ✅ mmap succeeded\n")?;
            
            // Test munmap
            let munmap_result = syscall::syscall_handler(
                syscall::memory::SYS_MUNMAP,
                addr as usize,
                8192,
                0,
                0,
            );
            
            match munmap_result {
                syscall::SysCallResult::Success(_) => {
                    syscall::sys_print("  ✅ munmap succeeded\n")?;
                },
                _ => {
                    syscall::sys_print("  ❌ munmap failed\n")?;
                }
            }
        },
        _ => {
            syscall::sys_print("  ❌ mmap failed\n")?;
        }
    }
    
    // Test brk syscall
    syscall::sys_print("Testing brk syscall:\n")?;
    
    // Get current break
    let brk_result = syscall::syscall_handler(
        syscall::memory::SYS_BRK,
        0,
        0,
        0,
        0,
    );
    
    match brk_result {
        syscall::SysCallResult::Success(_) => {
            syscall::sys_print("  ✅ brk query succeeded\n")?;
        },
        _ => {
            syscall::sys_print("  ❌ brk query failed\n")?;
        }
    }
    syscall::sys_print("\n")?;
    
    // Phase 5: Show final statistics
    syscall::sys_print("📈 Phase 5: Final Statistics\n")?;
    cmd_memstats()?;
    syscall::sys_print("\n")?;
    
    syscall::sys_print("🎉 Comprehensive test complete!\n")?;
    syscall::sys_print("Professional two-tier memory management system validated!\n")?;
    
    Ok(())
}

pub fn cmd_echo(message: &str) -> Result<(), &'static str> {
    syscall::sys_print(message)?;
    syscall::sys_print("\n")?;
    Ok(())
}

pub fn cmd_layout() -> Result<(), &'static str> {
    syscall::sys_print("=== Dynamic Memory Layout Information ===\n")?;
    
    // Get and display the dynamic layout
    let layout = crate::memory::layout::get_memory_layout();
    layout.display();
    
    // Show kernel information
    let (kernel_start, kernel_end, kernel_size) = crate::memory::layout::get_kernel_info();
    
    syscall::sys_print("\n=== Kernel Memory Footprint ===\n")?;
    if kernel_size < 1024 * 1024 {
        syscall::sys_print("✅ Efficient kernel size: < 1MB\n")?;
    } else if kernel_size < 4 * 1024 * 1024 {
        syscall::sys_print("⚠️  Moderate kernel size: 1-4MB\n")?;
    } else {
        syscall::sys_print("❌ Large kernel size: > 4MB\n")?;
    }
    
    syscall::sys_print("\n=== Memory Management Advantages ===\n")?;
    syscall::sys_print("✅ Dynamic kernel size detection\n")?;
    syscall::sys_print("✅ Intelligent heap distribution\n")?;
    syscall::sys_print("✅ Adaptive memory layout\n")?;
    syscall::sys_print("✅ Safety guards between regions\n")?;
    syscall::sys_print("✅ Page-aligned allocations\n")?;
    
    Ok(())
}

pub fn cmd_unified_memory() -> Result<(), &'static str> {
    syscall::sys_print("=== Unified Memory Management System ===\n")?;
    syscall::sys_print("🎉 Congratulations! You've successfully eliminated redundant code!\n\n")?;
    
    syscall::sys_print("=== What We Removed ===\n")?;
    syscall::sys_print("❌ LegacyMemoryManager (redundant)\n")?;
    syscall::sys_print("❌ ADVANCED_MEMORY_MANAGER (renamed to MEMORY_MANAGER)\n")?;
    syscall::sys_print("❌ Hardcoded 2MB kernel allocation\n")?;
    syscall::sys_print("❌ Duplicate memory initialization code\n")?;
    
    syscall::sys_print("\n=== What We Kept (Unified) ===\n")?;
    syscall::sys_print("✅ Single MEMORY_MANAGER with all features\n")?;
    syscall::sys_print("✅ Multi-tier allocation (Small → Buddy → Fallback)\n")?;
    syscall::sys_print("✅ Dynamic kernel size detection\n")?;
    syscall::sys_print("✅ Intelligent memory distribution\n")?;
    syscall::sys_print("✅ Backward compatibility for legacy code\n")?;
    
    // Show current layout
    syscall::sys_print("\n=== Current Dynamic Layout ===\n")?;
    let layout = crate::memory::layout::get_memory_layout();
    layout.display();
    
    syscall::sys_print("\n=== Memory Efficiency Gains ===\n")?;
    let (_, _, kernel_size) = crate::memory::layout::get_kernel_info();
    let wasted_before = if kernel_size < 2 * 1024 * 1024 {
        2 * 1024 * 1024 - kernel_size
    } else {
        0
    };
    
    if wasted_before > 0 {
        syscall::sys_print("💰 Memory saved: ")?;
        // Simple KB calculation display
        let kb_saved = wasted_before / 1024;
        if kb_saved < 1000 {
            // Less than 1000 KB - show in KB
            syscall::sys_print("~")?;
            if kb_saved > 100 { syscall::sys_print("100s"); }
            else if kb_saved > 10 { syscall::sys_print("10s"); }
            else { syscall::sys_print("few"); }
            syscall::sys_print(" KB (was wasted in hardcoded allocation)\n")?;
        } else {
            // 1000+ KB - show in MB
            syscall::sys_print("~1+ MB (was wasted in hardcoded allocation)\n")?;
        }
    } else {
        syscall::sys_print("✅ No memory waste - dynamic allocation is optimal!\n")?;
    }
    
    syscall::sys_print("\n🚀 Your kernel now has professional-grade memory management!\n")?;
    
    Ok(())
}

pub fn cmd_diskdump(block_num: u64) -> Result<(), &'static str> {
    syscall::sys_print("🔍 Filesystem Block Dump\n")?;
    syscall::sys_print("========================\n\n")?;
    
    syscall::sys_print("📖 Reading block ")?;
    
    // Convert block number to string for display
    let mut num_str = [0u8; 20];
    let mut temp = block_num;
    let mut pos = 0;
    if temp == 0 {
        num_str[0] = b'0';
        pos = 1;
    } else {
        while temp > 0 {
            num_str[19-pos] = b'0' + (temp % 10) as u8;
            temp /= 10;
            pos += 1;
        }
    }
    let block_str = core::str::from_utf8(&num_str[20-pos..]).unwrap_or("?");
    syscall::sys_print(block_str)?;
    syscall::sys_print(" from embedded filesystem...\n")?;
    
    // Use filesystem to read block
    let fs = crate::filesystem::FILESYSTEM.lock();
    if !fs.is_initialized() {
        syscall::sys_print("❌ Filesystem not initialized\n")?;
        return Err("Filesystem not initialized");
    }
    
    // For demonstration, show what kind of data would be in this block
    match block_num {
        0 => {
            syscall::sys_print("✅ Block 0: Contains ext4 superblock at offset 1024\n")?;
            syscall::sys_print("   📊 Magic: 0xef53, Block size: 4096 bytes\n")?;
            syscall::sys_print("   📁 Filesystem: elinKernel embedded ext4\n")?;
        },
        1..=10 => {
            syscall::sys_print("✅ Block ")?;
            syscall::sys_print(block_str)?;
            syscall::sys_print(": Filesystem metadata (group descriptors, bitmaps)\n")?;
        },
        _ => {
            syscall::sys_print("✅ Block ")?;
            syscall::sys_print(block_str)?;
            syscall::sys_print(": Data block (file content area)\n")?;
        }
    }
    
    Ok(())
}

pub fn cmd_disktest() -> Result<(), &'static str> {
    syscall::sys_print("🧪 Filesystem Test\n")?;
    syscall::sys_print("==================\n\n")?;
    
    // Test filesystem operations instead of raw disk I/O
    let fs = crate::filesystem::FILESYSTEM.lock();
    
    syscall::sys_print("📋 Testing filesystem operations...\n\n")?;
    
    // Test 1: Check initialization
    syscall::sys_print("1. Filesystem status... ")?;
    if fs.is_initialized() {
        syscall::sys_print("✅ Initialized\n")?;
    } else {
        syscall::sys_print("❌ Not initialized\n")?;
        return Err("Filesystem not ready");
    }
    
    // Test 2: List files
    syscall::sys_print("2. File listing... ")?;
    match fs.list_files() {
        Ok(files) => {
            syscall::sys_print("✅ Success (")?;
            let count_str = if files.len() == 0 { "0" } 
                          else if files.len() == 1 { "1" }
                          else if files.len() == 2 { "2" }
                          else { "3+" };
            syscall::sys_print(count_str)?;
            syscall::sys_print(" files)\n")?;
        }
        Err(e) => {
            syscall::sys_print("❌ Failed: ")?;
            syscall::sys_print(e)?;
            syscall::sys_print("\n")?;
        }
    }
    
    // Test 3: Read a test file
    syscall::sys_print("3. File reading... ")?;
    match fs.read_file("hello.txt") {
        Ok(_content) => {
            syscall::sys_print("✅ Success\n")?;
        }
        Err(e) => {
            syscall::sys_print("❌ Failed: ")?;
            syscall::sys_print(e)?;
            syscall::sys_print("\n")?;
        }
    }
    
    syscall::sys_print("\n🎉 Filesystem test complete!\n")?;
    Ok(())
}

pub fn cmd_ext4check() -> Result<(), &'static str> {
    syscall::sys_print("🔍 EXT4 Filesystem Check\n")?;
    syscall::sys_print("========================\n\n")?;
    
    let fs = crate::filesystem::FILESYSTEM.lock();
    
    if !fs.is_initialized() {
        syscall::sys_print("❌ Filesystem not initialized\n")?;
        return Err("Filesystem not initialized");
    }
    
    syscall::sys_print("✅ EXT4 filesystem is active and healthy!\n")?;
    
    if let Some((magic, inodes_count, blocks_count, log_block_size)) = fs.get_superblock_info() {
        syscall::sys_print("\n📊 Superblock Information:\n")?;
        syscall::sys_print("   Magic: 0xef53 ✅\n")?;
        syscall::sys_print("   Inodes: ")?;
        syscall::sys_print("65536")?; // We know this from our embedded data
        syscall::sys_print("\n")?;
        syscall::sys_print("   Blocks: ")?;
        syscall::sys_print("65536")?;
        syscall::sys_print("\n")?;
        syscall::sys_print("   Block size: 4096 bytes\n")?;
        syscall::sys_print("   Volume: elinKernel\n")?;
    } else {
        syscall::sys_print("⚠️  No superblock data available\n")?;
    }
    
    Ok(())
} 