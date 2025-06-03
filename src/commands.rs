use crate::syscall;

// Shell commands that use system calls

// Central command processor - main.rs calls this function
pub fn process_command(command: &str) {
    let command = command.trim();
    
    let result = match command {
        // Essential system commands
        "help" => cmd_help(),
        "version" => cmd_version(),
        "memory" => cmd_memory(),
        "devices" => cmd_devices(),
        "syscall" => cmd_syscall(),
        
        // File operations (working via VirtIO)
        "ls" => cmd_ls(),
        "cat" => cmd_cat(""),
        "echo" => cmd_echo(""),
        
        // System control
        "shutdown" => cmd_shutdown(),
        "reboot" => cmd_reboot(),
        
        // Commands with arguments
        cmd if cmd.starts_with("cat ") => {
            let filename = &cmd[4..];
            cmd_cat(filename)
        }
        cmd if cmd.starts_with("echo ") => {
            let message = &cmd[5..];
            cmd_echo(message)
        }
        
        // Empty command
        "" => Ok(()),
        
        // Unknown command
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
        "help", "version", "memory", "devices", "syscall",
        "ls", "cat", "echo", 
        "shutdown", "reboot"
    ]
}

// === INDIVIDUAL COMMAND IMPLEMENTATIONS ===

pub fn cmd_help() -> Result<(), &'static str> {
    syscall::sys_print("📖 elinOS Commands\n")?;
    syscall::sys_print("===============================================\n\n")?;
    
    syscall::sys_print("🗂️  File Operations (via VirtIO block device):\n")?;
    syscall::sys_print("  ls              - List files in filesystem\n")?;
    syscall::sys_print("  cat <file>      - Display file contents\n")?;
    syscall::sys_print("  echo <message>  - Echo a message\n")?;
    
    syscall::sys_print("\n📊 System Information:\n")?;
    syscall::sys_print("  help            - Show this help message\n")?;
    syscall::sys_print("  version         - Show kernel version\n")?;
    syscall::sys_print("  memory          - Show memory information\n")?;
    syscall::sys_print("  devices         - List VirtIO and other devices\n")?;
    syscall::sys_print("  syscall         - Show system call information\n")?;
    
    syscall::sys_print("\n⚙️  System Control:\n")?;
    syscall::sys_print("  shutdown        - Shutdown the system\n")?;
    syscall::sys_print("  reboot          - Reboot the system\n")?;
    
    syscall::sys_print("\n🎉 Success! You now have:\n")?;
    syscall::sys_print("  ✅ VirtIO block device\n")?;
    syscall::sys_print("  ✅ FAT32 filesystem\n")?;
    syscall::sys_print("  ✅ Working syscalls (openat, read, close)\n")?;
    syscall::sys_print("  ✅ Legacy VirtIO 1.0 support\n")?;
    syscall::sys_print("  ✅ Complete I/O stack: command → syscall → filesystem → VirtIO → QEMU\n")?;
    
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
    // Use SYS_GETDENTS64 syscall to read directory entries
    let result = syscall::syscall_handler(
        syscall::file::SYS_GETDENTS64,
        0, // fd for current directory (or root)
        0, // buffer (kernel will handle)
        0, // count
        0,
    );
    
    match result {
        syscall::SysCallResult::Success(_) => Ok(()),
        syscall::SysCallResult::Error(_) => {
            // Fallback to filesystem interface for now
            match crate::filesystem::list_files() {
                Ok(()) => Ok(()),
                Err(_) => {
                    syscall::sys_print("Failed to list files\n")?;
                    Err("Failed to list files")
                }
            }
        }
    }
}

pub fn cmd_cat(filename: &str) -> Result<(), &'static str> {
    if filename.is_empty() {
        return Err("Usage: cat <filename>");
    }
    
    // Use proper syscalls: OPENAT -> READ -> CLOSE
    let mut filename_buf = [0u8; 256];
    if filename.len() >= filename_buf.len() {
        return Err("Filename too long");
    }
    
    filename_buf[..filename.len()].copy_from_slice(filename.as_bytes());
    
    // Step 1: Open the file with SYS_OPENAT
    // SYS_OPENAT signature: (dirfd, pathname, flags, mode)
    let open_result = syscall::syscall_handler(
        syscall::file::SYS_OPENAT,
        -100isize as usize, // AT_FDCWD (current working directory)
        filename_buf.as_ptr() as usize, // pathname
        0, // flags (O_RDONLY)
        0, // mode (not used for opening existing files)
    );
    
    match open_result {
        syscall::SysCallResult::Success(fd) => {
            // Step 2: Read file content with SYS_READ
            let read_result = syscall::syscall_handler(
                syscall::file::SYS_READ,
                fd as usize, // file descriptor
                0, // buffer (kernel handles)
                4096, // max bytes to read
                0,
            );
            
            // Step 3: Close file (when SYS_CLOSE is implemented)
            let _ = syscall::syscall_handler(
                syscall::file::SYS_CLOSE,
                fd as usize,
                0, 0, 0,
            );
            
            match read_result {
                syscall::SysCallResult::Success(_) => Ok(()),
                syscall::SysCallResult::Error(_) => {
                    syscall::sys_print("Failed to read file content\n")?;
                    Err("Failed to read file content")
                }
            }
        }
        syscall::SysCallResult::Error(_) => {
            // Fallback to filesystem interface
            match crate::filesystem::read_file(filename) {
                Ok(()) => Ok(()),
                Err(_) => {
                    syscall::sys_print("Failed to read file\n")?;
                    Err("Failed to read file")
                }
            }
        }
    }
}

pub fn cmd_syscall() -> Result<(), &'static str> {
    syscall::sys_print("System Call Information:\n")?;
     
    syscall::sys_print("Currently Implemented System Calls:\n")?;
    syscall::sys_print("  File I/O Operations:\n")?;
    syscall::sys_print("    SYS_WRITE (64)     - Write to file descriptor\n")?;
    syscall::sys_print("    SYS_READ (63)      - Read from file descriptor\n")?;
    syscall::sys_print("    SYS_OPENAT (56)    - Open file (modern Linux openat)\n")?;
    syscall::sys_print("    SYS_CLOSE (57)     - Close file descriptor\n")?;
    syscall::sys_print("    SYS_GETDENTS64 (61) - List directory entries\n")?;

    syscall::sys_print("  Memory Management:\n")?;
    syscall::sys_print("    SYS_GETMEMINFO (960) - Memory information (elinOS)\n")?;

    syscall::sys_print("  Process Management:\n")?;
    syscall::sys_print("    SYS_EXIT (93)      - Exit process\n")?;
    syscall::sys_print("    SYS_GETPID (172)   - Get process ID\n")?;
    syscall::sys_print("    SYS_GETPPID (173)  - Get parent process ID\n")?;

    syscall::sys_print("  Device Management:\n")?;
    syscall::sys_print("    SYS_GETDEVICES (950) - Device information (elinOS)\n")?;

    syscall::sys_print("  elinOS-Specific (System Control):\n")?;
    syscall::sys_print("    SYS_ELINOS_VERSION (902)  - Show version\n")?;
    syscall::sys_print("    SYS_ELINOS_SHUTDOWN (903) - Shutdown system\n")?;
    syscall::sys_print("    SYS_ELINOS_REBOOT (904)   - Reboot system\n")?;

    syscall::sys_print("\nNumbers in parentheses are Linux-compatible syscall numbers.\n")?;
    syscall::sys_print("This makes elinOS easier to understand for Linux developers!\n")?;
    Ok(())
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

pub fn cmd_echo(message: &str) -> Result<(), &'static str> {
    syscall::sys_print(message)?;
    syscall::sys_print("\n")?;
    Ok(())
} 