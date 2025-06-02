use crate::syscall;

// Shell commands that use system calls

// Central command processor - main.rs calls this function
pub fn process_command(command: &str) {
    let parts: heapless::Vec<&str, 8> = command.split_whitespace().collect();
    
    if parts.is_empty() {
        return;
    }

    // Dispatch to appropriate command handler
    let result = match parts[0] {
        "help" => cmd_help(),
        "memory" => cmd_memory(),
        "devices" => cmd_devices(), 
        "ls" => cmd_ls(),
        "cat" => {
            if parts.len() > 1 {
                cmd_cat(parts[1])
            } else {
                let _ = syscall::sys_print("Usage: cat <filename>\n");
                Ok(())
            }
        },
        "touch" => {
            if parts.len() > 1 {
                cmd_touch(parts[1])
            } else {
                let _ = syscall::sys_print("Usage: touch <filename>\n");
                Ok(())
            }
        },
        "rm" => {
            if parts.len() > 1 {
                cmd_rm(parts[1])
            } else {
                let _ = syscall::sys_print("Usage: rm <filename>\n");
                Ok(())
            }
        },
        "clear" => cmd_clear(),
        "syscall" => cmd_syscall(),
        "categories" => cmd_categories(),
        "version" => cmd_version(),
        "shutdown" => cmd_shutdown(),
        "reboot" => cmd_reboot(),
        "elf-info" => {
            if parts.len() > 1 {
                cmd_elf_info(parts[1])
            } else {
                let _ = syscall::sys_print("Usage: elf-info <filename>\n");
                Ok(())
            }
        },
        "elf-load" => {
            if parts.len() > 1 {
                cmd_elf_load(parts[1])
            } else {
                let _ = syscall::sys_print("Usage: elf-load <filename>\n");
                Ok(())
            }
        },
        "elf-exec" => {
            if parts.len() > 1 {
                cmd_elf_exec(parts[1])
            } else {
                let _ = syscall::sys_print("Usage: elf-exec <filename>\n");
                Ok(())
            }
        },
        "elf-demo" => cmd_elf_demo(),
        _ => {
            let _ = syscall::sys_print("Unknown command: ");
            let _ = syscall::sys_print(parts[0]);
            let _ = syscall::sys_print(". Type 'help' for available commands.\n");
            Ok(())
        }
    };

    // Handle any command errors
    if let Err(e) = result {
        let _ = syscall::sys_print("Command error: ");
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
    syscall::sys_print("  help       - Show this help\n")?;
    syscall::sys_print("  memory     - Show memory information\n")?;
    syscall::sys_print("  devices    - Probe for VirtIO devices\n")?;
    syscall::sys_print("  ls         - List files\n")?;
    syscall::sys_print("  cat <file> - Show file contents\n")?;
    syscall::sys_print("  touch <file> - Create empty file\n")?;
    syscall::sys_print("  rm <file>  - Delete file\n")?;
    syscall::sys_print("  clear      - Clear screen\n")?;
    syscall::sys_print("  syscall    - Show system call info\n")?;
    syscall::sys_print("  categories - Show syscall categories\n")?;
    syscall::sys_print("  version    - Show elinKernel version\n")?;
    syscall::sys_print("  shutdown   - Shutdown the system\n")?;
    syscall::sys_print("  reboot     - Reboot the system\n")?;
    syscall::sys_print("\nELF Binary Support:\n")?;
    syscall::sys_print("  elf-info <file> - Show ELF binary information\n")?;
    syscall::sys_print("  elf-load <file> - Load ELF binary into memory\n")?;
    syscall::sys_print("  elf-exec <file> - Load and execute ELF binary\n")?;
    syscall::sys_print("  elf-demo        - Demonstrate ELF loading with sample binary\n")?;
    Ok(())
}

pub fn cmd_memory() -> Result<(), &'static str> {
    syscall::sys_memory_info()
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
            if let Some(content) = fs.read_file(filename) {
                syscall::sys_print("Contents of ")?;
                syscall::sys_print(filename)?;
                syscall::sys_print(":\n")?;
                let content_str = core::str::from_utf8(content)
                    .unwrap_or("<binary content>");
                syscall::sys_print(content_str)?;
                syscall::sys_print("\n--- End of file ---\n")?;
                Ok(())
            } else {
                Err("Failed to read file")
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
            Some(data) => {
                // Clone the data to a Vec so we can use it after dropping the lock
                let mut cloned_data = heapless::Vec::<u8, 1024>::new();
                for &byte in data {
                    if cloned_data.push(byte).is_err() {
                        return Err("File too large to process");
                    }
                }
                cloned_data
            }
            None => {
                syscall::sys_print("Error: File '")?;
                syscall::sys_print(filename)?;
                syscall::sys_print("' not found\n")?;
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
            Some(data) => {
                // Clone the data to a Vec so we can use it after dropping the lock
                let mut cloned_data = heapless::Vec::<u8, 1024>::new();
                for &byte in data {
                    if cloned_data.push(byte).is_err() {
                        return Err("File too large to process");
                    }
                }
                cloned_data
            }
            None => {
                syscall::sys_print("Error: File '")?;
                syscall::sys_print(filename)?;
                syscall::sys_print("' not found\n")?;
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
            Some(data) => {
                // Clone the data to a Vec so we can use it after dropping the lock
                let mut cloned_data = heapless::Vec::<u8, 1024>::new();
                for &byte in data {
                    if cloned_data.push(byte).is_err() {
                        return Err("File too large to process");
                    }
                }
                cloned_data
            }
            None => {
                syscall::sys_print("Error: File '")?;
                syscall::sys_print(filename)?;
                syscall::sys_print("' not found\n")?;
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