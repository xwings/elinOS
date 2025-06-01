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
        "clear", "syscall", "categories", "version", "shutdown", "reboot"
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
    syscall::sys_print("  version    - Show ElinOS version\n")?;
    syscall::sys_print("  shutdown   - Shutdown the system\n")?;
    syscall::sys_print("  reboot     - Reboot the system\n")?;
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
    syscall::sys_print("System Call Information:\n")?;
    syscall::sys_print("  This shell uses categorized system calls for all kernel operations!\n\n")?;
    
    syscall::sys_print("Currently Implemented System Calls:\n")?;
    syscall::sys_print("  File I/O Operations:\n")?;
    syscall::sys_print("    SYS_WRITE (1)     - Write to file descriptor\n")?;
    syscall::sys_print("    SYS_READ (2)      - Read from file descriptor [TODO]\n")?;
    syscall::sys_print("    SYS_OPEN (3)      - Open file\n")?;
    syscall::sys_print("    SYS_CLOSE (4)     - Close file descriptor [TODO]\n")?;
    syscall::sys_print("    SYS_UNLINK (5)    - Delete file\n")?;
    syscall::sys_print("    SYS_GETDENTS (6)  - List directory entries\n")?;
    syscall::sys_print("    SYS_STAT (9)      - Get file status\n")?;

    syscall::sys_print("  Directory Operations:\n")?;
    syscall::sys_print("    SYS_MKDIR (51)    - Create directory [TODO]\n")?;
    syscall::sys_print("    SYS_RMDIR (52)    - Remove directory [TODO]\n")?;

    syscall::sys_print("  Memory Management:\n")?;
    syscall::sys_print("    SYS_MMAP (71)     - Memory mapping [TODO]\n")?;
    syscall::sys_print("    SYS_MUNMAP (72)   - Memory unmapping [TODO]\n")?;
    syscall::sys_print("    SYS_GETMEMINFO (100) - Memory information\n")?;

    syscall::sys_print("  Process Management:\n")?;
    syscall::sys_print("    SYS_EXIT (121)    - Exit process\n")?;

    syscall::sys_print("  Device Management:\n")?;
    syscall::sys_print("    SYS_GETDEVICES (200) - Device information\n")?;

    syscall::sys_print("  ElinOS-Specific:\n")?;
    syscall::sys_print("    SYS_ELINOS_DEBUG (900)    - Set debug level\n")?;
    syscall::sys_print("    SYS_ELINOS_VERSION (902)  - Show version\n")?;
    syscall::sys_print("    SYS_ELINOS_SHUTDOWN (903) - Shutdown system\n")?;
    syscall::sys_print("    SYS_ELINOS_REBOOT (904)   - Reboot system\n")?;

    syscall::sys_print("\nCommands are user-space programs that call these syscalls.\n")?;
    syscall::sys_print("Use 'categories' to see the full categorization system.\n")?;
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