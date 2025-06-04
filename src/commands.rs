use crate::syscall;
use crate::filesystem::traits::FileSystem;

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
        "fscheck" => cmd_fscheck(),
        
        // File operations (working via modular filesystem)
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
        "help", "version", "memory", "devices", "syscall", "fscheck",
        "ls", "cat", "echo", 
        "shutdown", "reboot"
    ]
}

// === INDIVIDUAL COMMAND IMPLEMENTATIONS ===

pub fn cmd_help() -> Result<(), &'static str> {
    syscall::sys_print("📖 elinOS Commands\n")?;
    syscall::sys_print("===============================================\n\n")?;
    
    syscall::sys_print("🗂️  File Operations (modular filesystem support):\n")?;
    syscall::sys_print("  ls              - List files in filesystem\n")?;
    syscall::sys_print("  cat <file>      - Display file contents\n")?;
    syscall::sys_print("  echo <message>  - Echo a message\n")?;
    
    syscall::sys_print("\n📊 System Information:\n")?;
    syscall::sys_print("  help            - Show this help message\n")?;
    syscall::sys_print("  version         - Show kernel version\n")?;
    syscall::sys_print("  memory          - Show memory information\n")?;
    syscall::sys_print("  devices         - List VirtIO and other devices\n")?;
    syscall::sys_print("  syscall         - Show system call information\n")?;
    syscall::sys_print("  fscheck         - Check filesystem status\n")?;
    
    syscall::sys_print("\n⚙️  System Control:\n")?;
    syscall::sys_print("  shutdown        - Shutdown the system\n")?;
    syscall::sys_print("  reboot          - Reboot the system\n")?;
    
    syscall::sys_print("\n🎉 Success! You now have:\n")?;
    syscall::sys_print("  ✅ VirtIO block device\n")?;
    syscall::sys_print("  ✅ Modular filesystem (FAT32 + ext4)\n")?;
    syscall::sys_print("  ✅ Automatic filesystem detection\n")?;
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
    // Use modular filesystem API
    match crate::filesystem::list_files() {
        Ok(files) => {
            // Get filesystem info for display
            let fs = crate::filesystem::FILESYSTEM.lock();
            let fs_type = fs.get_filesystem_type();
            let fs_info = fs.get_filesystem_info();
            drop(fs);
            
            syscall::sys_print("📁 Filesystem contents (VirtIO disk):\n")?;
            syscall::sys_print("Type: ")?;
            match fs_type {
                crate::filesystem::FilesystemType::Fat32 => syscall::sys_print("FAT32")?,
                crate::filesystem::FilesystemType::Ext4 => syscall::sys_print("ext4")?,
                crate::filesystem::FilesystemType::Unknown => syscall::sys_print("Unknown")?,
            }
            syscall::sys_print("\n")?;
            
            if let Some((signature, total_blocks, block_size)) = fs_info {
                syscall::sys_print("Boot signature/Magic: 0x")?;
                // Simple hex output without format!
                let hex_chars = [b'0', b'1', b'2', b'3', b'4', b'5', b'6', b'7', b'8', b'9', b'a', b'b', b'c', b'd', b'e', b'f'];
                let mut hex_str = [0u8; 4];
                hex_str[0] = hex_chars[((signature >> 12) & 0xF) as usize];
                hex_str[1] = hex_chars[((signature >> 8) & 0xF) as usize];
                hex_str[2] = hex_chars[((signature >> 4) & 0xF) as usize];
                hex_str[3] = hex_chars[(signature & 0xF) as usize];
                syscall::sys_print(core::str::from_utf8(&hex_str).unwrap_or("????"))?;
                syscall::sys_print("\n")?;
                
                syscall::sys_print("Total blocks/sectors: ")?;
                syscall::sys_print("(numeric display not implemented)\n")?;
                syscall::sys_print("Block/sector size: ")?;
                syscall::sys_print("(numeric display not implemented)\n")?;
                syscall::sys_print("\n")?;
            }
            
            if files.is_empty() {
                syscall::sys_print("(No files found)\n")?;
            } else {
                for (name, _size) in &files {
                    syscall::sys_print("  FILE  ")?;
                    syscall::sys_print(name.as_str())?;
                    syscall::sys_print("\n")?;
                }
                syscall::sys_print("\nTotal files: ")?;
                syscall::sys_print("(count display not implemented)\n")?;
            }
            
            Ok(())
        }
        Err(_) => {
            syscall::sys_print("Failed to list files\n")?;
            Err("Failed to list files")
        }
    }
}

pub fn cmd_cat(filename: &str) -> Result<(), &'static str> {
    if filename.is_empty() {
        return Err("Usage: cat <filename>");
    }
    
    // Use modular filesystem API
    match crate::filesystem::read_file(filename) {
        Ok(content) => {
            // Get filesystem type for display
            let fs = crate::filesystem::FILESYSTEM.lock();
            let fs_type = fs.get_filesystem_type();
            drop(fs);
            
            syscall::sys_print("📖 Reading file: ")?;
            syscall::sys_print(filename)?;
            syscall::sys_print(" (from ")?;
            match fs_type {
                crate::filesystem::FilesystemType::Fat32 => syscall::sys_print("FAT32")?,
                crate::filesystem::FilesystemType::Ext4 => syscall::sys_print("ext4")?,
                crate::filesystem::FilesystemType::Unknown => syscall::sys_print("Unknown")?,
            }
            syscall::sys_print(" filesystem)\n")?;
            
            if let Ok(content_str) = core::str::from_utf8(&content) {
                syscall::sys_print("Content:\n")?;
                syscall::sys_print(content_str)?;
                syscall::sys_print("\n")?;
            } else {
                syscall::sys_print("(Binary file - ")?;
                syscall::sys_print("bytes count not displayed")?;
                syscall::sys_print(")\n")?;
            }
            
            Ok(())
        }
        Err(_) => {
            syscall::sys_print("❌ File '")?;
            syscall::sys_print(filename)?;
            syscall::sys_print("' not found\n")?;
            Err("File not found")
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

pub fn cmd_fscheck() -> Result<(), &'static str> {
    match crate::filesystem::check_filesystem() {
        Ok(()) => Ok(()),
        Err(_) => {
            syscall::sys_print("Failed to check filesystem\n")?;
            Err("Failed to check filesystem")
        }
    }
} 