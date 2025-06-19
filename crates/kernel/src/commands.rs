use crate::syscall;
use crate::filesystem::traits::{FileSystem, FilesystemError, FileEntry};
use crate::memory::{self, BufferUsage};
use heapless::String;
use core::fmt::Write;
use crate::{UART, console_println};

// Shell commands that use system calls

const MAX_PATH_LEN: usize = 256;
static mut CURRENT_PATH: String<MAX_PATH_LEN> = String::new();

// Helper to initialize CWD if it's empty (e.g., on first command)
fn ensure_cwd_initialized() {
    unsafe {
        if CURRENT_PATH.is_empty() {
            // Initialize to root path "/"
            CURRENT_PATH.clear(); // Ensure it's empty before pushing
            if CURRENT_PATH.push('/').is_err() {
                // This should ideally not fail with a single char if MAX_PATH_LEN > 0
                // Consider a panic or a more robust error handling if path initialization is critical
                syscall::sys_print("CRITICAL: Failed to initialize CWD to root!\n").unwrap_or_default();
            }
        }
    }
}

// Helper function to resolve a path argument to an absolute path
fn resolve_path(path_arg: &str) -> String<MAX_PATH_LEN> {
    ensure_cwd_initialized(); // Ensure CURRENT_PATH is valid before use
    unsafe { // To access CURRENT_PATH
        let mut components: heapless::Vec<&str, 32> = heapless::Vec::new();

        // Determine the starting components based on whether path_arg is absolute
        if path_arg.starts_with('/') {
            // Absolute path, start fresh. Add a placeholder if path_arg is only "/" to avoid empty components later.
            // if path_arg == "/" { let _ = components.push(""); } // This logic is tricky, split handles it.
        } else {
            // Relative path, start with CURRENT_PATH components
            // Trim CURRENT_PATH to avoid empty strings if it's just "/"
            // and split by '/'
            for component in CURRENT_PATH.trim_matches('/').split('/') {
                if !component.is_empty() { // Avoid pushing empty strings from multiple slashes or root
                    let _ = components.push(component);
                }
            }
        }

        // Process path_arg components
        // Trim path_arg to handle cases like "dir/" or "/abs/path/"
        for component in path_arg.trim_matches('/').split('/') {
            if component.is_empty() || component == "." {
                continue; // Skip empty parts (e.g. '//') or current dir '.'
            }
            if component == ".." {
                if !components.is_empty() {
                    components.pop(); // Go up one level
                }
                // If components is empty, ".." at root stays at root, effectively.
            } else {
                if components.push(component).is_err() {
                    // Path too deep or too many components, handle error or truncate
                    syscall::sys_print("Warning: Path too long or complex, may be truncated.\n").unwrap_or_default();
                    break; 
                }
            }
        }

        // Construct the final path
        let mut final_path = String::<MAX_PATH_LEN>::new();
        if final_path.push('/').is_err() { /* error handling */ } // Always starts with /

        for (i, comp) in components.iter().enumerate() {
            if i > 0 { // Add separator for subsequent components
                if final_path.push('/').is_err() { break; }
            }
            if final_path.push_str(comp).is_err() { break; }
        }
        
        // If final_path is still just "/", it's correct.
        // If it became empty somehow (shouldn't with this logic), reset to "/"
        if final_path.is_empty() && components.is_empty() {
             // This case should ideally be covered by final_path.push('/') above
             // but as a safeguard:
            final_path.clear();
            final_path.push('/').unwrap_or_default();
        }
        final_path
    }
}

// Helper to print FilesystemError
fn print_filesystem_error(e: &FilesystemError) {
    // This is a simplified way to print. Ideally, FilesystemError would implement Display
    // or have a method to get a &'static str.
    // Using a temporary buffer to format the debug representation.
    let mut err_buf: String<128> = String::new();
    let _ = write!(err_buf, "{:?}", e); // Using Debug format
    let _ = syscall::sys_print("Error: ");
    let _ = syscall::sys_print(&err_buf);
    let _ = syscall::sys_print("\n");
}

// Central command processor - main.rs calls this function
pub fn process_command(command: &str) -> Result<(), &'static str> {
    ensure_cwd_initialized(); // Initialize CWD on first command
    let command = command.trim();
    
    let result = match command {
        // Essential system commands
        "help" => cmd_help(),
        "version" => cmd_version(),
        "memory" => cmd_memory(),
        "heap" => cmd_heap(),
        "heap-reset" => cmd_heap_reset(),
        "devices" => cmd_devices(),
        "syscall" => cmd_syscall(),
        "fscheck" => cmd_fscheck(),
        "config" => cmd_config(),
        
        // File operations (working via modular filesystem)
        "ls" => cmd_ls(None),
        "cat" => {
            syscall::sys_print("Usage: cat <filename>\n")?;
            Ok(())
        },
        "echo" => cmd_echo(""),
        "pwd" => cmd_pwd(),

        // New file/dir operations
        "touch" => {
            syscall::sys_print("Usage: touch <filename>\n")?;
            Ok(())
        },
        "mkdir" => {
            syscall::sys_print("Usage: mkdir <dirname>\n")?;
            Ok(())
        },
        "rm" => {
            syscall::sys_print("Usage: rm <filename>\n")?;
            Ok(())
        },
        "rmdir" => {
            syscall::sys_print("Usage: rmdir <dirname>\n")?;
            Ok(())
        },
        "cd" => {
            cmd_cd("/")
        },
                
        // System control
        "shutdown" => cmd_shutdown(),
        "reboot" => cmd_reboot(),
        
        // Commands with arguments
        cmd if cmd.starts_with("ls ") => {
            let path_arg = &cmd[3..].trim();
            cmd_ls(Some(path_arg))
        },
        cmd if cmd.starts_with("cat ") => {
            let path_arg = &cmd[4..].trim();
            if path_arg.is_empty() {
                syscall::sys_print("Usage: cat <filename>\n")?;
                Ok(())
            } else {
                let full_path = resolve_path(path_arg);
                cmd_cat(&full_path)
            }
        },
        cmd if cmd.starts_with("echo ") => {
            let message = &cmd[5..];
            cmd_echo(message)
        },
        
        // Commands with arguments for new fs operations
        cmd if cmd.starts_with("touch ") => {
            let path_arg = cmd.strip_prefix("touch ").unwrap_or("").trim();
            if path_arg.is_empty() {
                syscall::sys_print("Usage: touch <filename>\n")?;
                Ok(())
            } else {
                let full_path = resolve_path(path_arg);
                cmd_touch(&full_path)
            }
        },
        cmd if cmd.starts_with("mkdir ") => {
            let path_arg = cmd.strip_prefix("mkdir ").unwrap_or("").trim();
            if path_arg.is_empty() {
                syscall::sys_print("Usage: mkdir <dirname>\n")?;
                Ok(())
            } else {
                let full_path = resolve_path(path_arg);
                cmd_mkdir(&full_path)
            }
        },
        cmd if cmd.starts_with("rm ") => {
            let path_arg = cmd.strip_prefix("rm ").unwrap_or("").trim();
            if path_arg.is_empty() {
                syscall::sys_print("Usage: rm <filename>\n")?;
                Ok(())
            } else {
                let full_path = resolve_path(path_arg);
                cmd_rm(&full_path)
            }
        },
        cmd if cmd.starts_with("rmdir ") => {
            let path_arg = cmd.strip_prefix("rmdir ").unwrap_or("").trim();
            if path_arg.is_empty() {
                syscall::sys_print("Usage: rmdir <dirname>\n")?;
                Ok(())
            } else {
                let full_path = resolve_path(path_arg);
                cmd_rmdir(&full_path)
            }
        },
        cmd if cmd.starts_with("cd ") => {
            let path_arg = cmd.strip_prefix("cd ").unwrap_or("").trim();
            cmd_cd(path_arg)
        },
        

        
        // Empty command
        "" => Ok(()),
        
        // Unknown command - try to execute as ELF binary
        _ => {
            // Try to execute as ELF binary
            let full_path = resolve_path(command);
            
            // Check if file exists and try to execute it
            match crate::filesystem::read_file(&full_path) {
                Ok(file_data) => {
                    // Debug: Show file size and first few bytes
                    let _ = syscall::sys_print("[i] Debug: File size: ");
                    let _ = syscall::sys_print_num(file_data.len() as u64);
                    let _ = syscall::sys_print(" bytes\n");
                    
                    if file_data.len() >= 4 {
                        let _ = syscall::sys_print("[i] Debug: First 4 bytes: ");
                        for i in 0..4 {
                            let _ = syscall::sys_print_hex(file_data[i] as u32, 2);
                            let _ = syscall::sys_print(" ");
                        }
                        let _ = syscall::sys_print("\n");
                    }
                    
                    // Check if it's an ELF file by looking at magic bytes
                    if file_data.len() >= 4 && &file_data[0..4] == b"\x7fELF" {
                        cmd_execute_elf(&full_path, &file_data)
                    } else {
                        let _ = syscall::sys_print("[x] Not an executable file: ");
                        let _ = syscall::sys_print(command);
                        let _ = syscall::sys_print(" (expected ELF magic: 7f 45 4c 46)\n");
                        Err("Not an executable")
                    }
                }
                Err(_) => {
                    let _ = syscall::sys_print("Unknown command: ");
                    let _ = syscall::sys_print(command);
                    let _ = syscall::sys_print("\nType 'help' for available commands.\n");
                    Ok(())
                }
            }
        }
    };

    result
}

// Get list of all available commands (for help and autocomplete)
pub fn get_available_commands() -> &'static [&'static str] {
    &[
        "help", "version", "memory", "devices", "syscall", "fscheck", "config",
        "ls", "cat", "echo", "pwd",
        "touch", "mkdir", "rm", "rmdir", "cd",
        "shutdown", "reboot"
    ]
}

// === INDIVIDUAL COMMAND IMPLEMENTATIONS ===

pub fn cmd_help() -> Result<(), &'static str> {
    syscall::sys_print("[i] ElinOS Commands\n")?;
    syscall::sys_print("===============================================\n\n")?;
    
    syscall::sys_print("[i] System Information:\n")?;
    syscall::sys_print("  help            - Show this help message\n")?;
    syscall::sys_print("  version         - Show kernel version and features\n")?;
    syscall::sys_print("  memory          - Show memory regions and allocator statistics\n")?;
    syscall::sys_print("  devices         - List detected VirtIO devices\n")?;
    syscall::sys_print("  syscall         - Show system call information\n")?;
    syscall::sys_print("  fscheck         - Check filesystem status and metadata\n")?;
    syscall::sys_print("  config          - Show system configuration\n")?;

    syscall::sys_print("\n[i]  Filesystem Operations:\n")?;
    syscall::sys_print("  ls [path]       - List files/dirs (default: current directory)\n")?;
    syscall::sys_print("  cat <path>      - Display file contents\n")?;
    syscall::sys_print("  echo [message]  - Print a message (newline if no message)\n")?;
    syscall::sys_print("  pwd             - Print current working directory\n")?;
    syscall::sys_print("  touch <path>    - Create an empty file at the specified path\n")?;
    syscall::sys_print("  mkdir <path>    - Create a directory at the specified path\n")?;
    syscall::sys_print("  rm <path>       - Remove a file at the specified path\n")?;
    syscall::sys_print("  rmdir <path>    - Remove an empty directory at the specified path\n")?;
    syscall::sys_print("  cd [path]       - Change directory (default: root, use '/', '..')\n")?;
    
    syscall::sys_print("\n[i] Program Execution:\n")?;
    syscall::sys_print("  hello_simple    - Execute ELF binary directly by name\n")?;
    syscall::sys_print("  ./hello_simple  - Execute with explicit relative path\n")?;
    syscall::sys_print("  /programs/hello - Execute with absolute path\n")?;
    
    syscall::sys_print("\n[i] System Control:\n")?;
    syscall::sys_print("  shutdown        - Shutdown the system via SBI\n")?;
    syscall::sys_print("  reboot          - Reboot the system via SBI\n")?;
    
    Ok(())
}

pub fn cmd_config() -> Result<(), &'static str> {
    syscall::sys_print("[i] Dynamic System Configuration\n")?;
    syscall::sys_print("=====================================\n\n")?;
    
    // Get memory statistics
    let mem_stats = memory::get_memory_stats();
    
    syscall::sys_print("[i] Hardware Detection Results:\n")?;
    
    syscall::sys_print("  Total RAM: ")?;
    show_number_mb(mem_stats.detected_ram_size);
    syscall::sys_print(" MB \n")?;
    
    syscall::sys_print("  Memory Regions: ")?;
    show_number(mem_stats.regions_detected);
    syscall::sys_print(" (discovered via SBI)\n")?;
    
    syscall::sys_print("  Allocator Mode: ")?;
    match mem_stats.allocator_mode {
        memory::AllocatorMode::SimpleHeap => syscall::sys_print("Simple Heap (low memory)\n")?,
        memory::AllocatorMode::TwoTier => syscall::sys_print("Two-Tier (Buddy + Slab)\n")?,
        memory::AllocatorMode::Hybrid => syscall::sys_print("Hybrid (adaptive)\n")?,
    }
    
    syscall::sys_print("\n[i] Calculated Memory Allocations:\n")?;
    
    syscall::sys_print("  Kernel Heap: ")?;
    show_number_kb(mem_stats.heap_size);
    syscall::sys_print(" KB (scaled to RAM size)\n")?;
    
    syscall::sys_print("  Heap Used: ")?;
    show_number_kb(mem_stats.heap_used);
    syscall::sys_print(" KB\n")?;
    
    syscall::sys_print("  Heap Utilization: ")?;
    if mem_stats.heap_size > 0 {
        let utilization = (mem_stats.heap_used * 100) / mem_stats.heap_size;
        show_number(utilization);
        syscall::sys_print("%\n")?;
    } else {
        syscall::sys_print("N/A\n")?;
    }
    
    syscall::sys_print("\n[i] Dynamic Buffer Sizes:\n")?;
    
    let sector_buf_size = memory::get_optimal_buffer_size(BufferUsage::SectorIO);
    syscall::sys_print("  Sector I/O: ")?;
    show_number(sector_buf_size);
    syscall::sys_print(" bytes\n")?;
    
    let file_buf_size = memory::get_optimal_buffer_size(BufferUsage::FileRead);
    syscall::sys_print("  File Reading: ")?;
    show_number_kb(file_buf_size);
    syscall::sys_print(" KB\n")?;
    
    let cmd_buf_size = memory::get_optimal_buffer_size(BufferUsage::Command);
    syscall::sys_print("  Command Input: ")?;
    show_number(cmd_buf_size);
    syscall::sys_print(" bytes\n")?;
    
    let max_file_size = memory::get_max_file_size();
    syscall::sys_print("  Max File Size: ")?;
    show_number_kb(max_file_size);
    syscall::sys_print(" KB\n")?;
    
    Ok(())
}

// Helper functions for number display without format! macro
fn show_number(mut num: usize) {
    if num == 0 {
        let _ = syscall::sys_print("0");
        return;
    }
    
    let mut digits = heapless::Vec::<u8, 20>::new();
    while num > 0 {
        let _ = digits.push((num % 10) as u8 + b'0');
        num /= 10;
    }
    
    // Print digits in reverse order
    for &digit in digits.iter().rev() {
        let digit_str = [digit];
        if let Ok(s) = core::str::from_utf8(&digit_str) {
            let _ = syscall::sys_print(s);
        }
    }
}

fn show_number_kb(bytes: usize) {
    show_number(bytes / 1024);
}

fn show_number_mb(bytes: usize) {
    show_number(bytes / (1024 * 1024));
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
        syscall::SysCallResult::Error(_) => Err("Syscall failed"),
    }
}

pub fn cmd_devices() -> Result<(), &'static str> {
    syscall::sys_device_info()
}

pub fn cmd_ls(path_arg_opt: Option<&str>) -> Result<(), &'static str> {
    ensure_cwd_initialized();
    let list_target_path: String<MAX_PATH_LEN>;
    unsafe { // Access CURRENT_PATH
        list_target_path = match path_arg_opt {
            Some(path_arg) => resolve_path(path_arg),
            None => String::try_from(CURRENT_PATH.as_str()).unwrap_or_default(),
        };
    }

    syscall::sys_print("Listing for target '")?;
    syscall::sys_print(&list_target_path)?;
    syscall::sys_print("':\n")?;

    // Use the new path-aware directory listing
    match crate::filesystem::list_directory(&list_target_path) {
        Ok(files) => {
            // Get filesystem info for display
            let fs = crate::filesystem::FILESYSTEM.lock();
            let fs_type = fs.get_filesystem_type();
            let fs_info = fs.get_filesystem_info();
            drop(fs);
            
            //syscall::sys_print("[i] Filesystem contents (VirtIO disk):\n")?;
            //syscall::sys_print("Type: ")?;
            //match fs_type {
            //    crate::filesystem::FilesystemType::Fat32 => syscall::sys_print("FAT32")?,
            //    crate::filesystem::FilesystemType::Ext2 => syscall::sys_print("ext2")?,
            //    crate::filesystem::FilesystemType::Unknown => syscall::sys_print("Unknown")?,
            //}
            // syscall::sys_print("\n")?;
            
            // if let Some((signature, _total_blocks, _block_size)) = fs_info {
            //     //syscall::sys_print("Boot signature/Magic: 0x")?;
            //     // Simple hex output without format!
            //     let hex_chars = [b'0', b'1', b'2', b'3', b'4', b'5', b'6', b'7', b'8', b'9', b'a', b'b', b'c', b'd', b'e', b'f'];
            //     let mut hex_str = [0u8; 4];
            //     hex_str[0] = hex_chars[((signature >> 12) & 0xF) as usize];
            //     hex_str[1] = hex_chars[((signature >> 8) & 0xF) as usize];
            //     hex_str[2] = hex_chars[((signature >> 4) & 0xF) as usize];
            //     hex_str[3] = hex_chars[(signature & 0xF) as usize];
            //     syscall::sys_print(core::str::from_utf8(&hex_str).unwrap_or("????"))?;
            //     syscall::sys_print("\n")?;
                
            //     // syscall::sys_print("Total blocks/sectors: ")?;
            //     // syscall::sys_print("(numeric display not implemented)\n")?;
            //     // syscall::sys_print("Block/sector size: ")?;
            //     // syscall::sys_print("(numeric display not implemented)\n")?;
            //     syscall::sys_print("\n")?;
            // }
            
            if files.is_empty() {
                syscall::sys_print("(No files found)\n")?;
            } else {
                for (name, _size, is_directory) in &files {
                    if *is_directory {
                        syscall::sys_print("  DIR   ")?;
                    } else {
                        syscall::sys_print("  FILE  ")?;
                    }
                    syscall::sys_print(name.as_str())?;
                    syscall::sys_print("\n")?;
                }
                syscall::sys_print("\nTotal files: ")?;
                show_number(files.len());
                syscall::sys_print("\n")?;
            }
            
            Ok(())
        }
        Err(_) => {
            syscall::sys_print("Failed to list directory\n")?;
            Err("Failed to list directory")
        }
    }
}

pub fn cmd_cat(filename: &str) -> Result<(), &'static str> {
    if filename.is_empty() {
        return Err("Filename cannot be empty for cat");
    }
    
    // Use modular filesystem API
    match crate::filesystem::read_file(filename) {
        Ok(content) => {
            // Get filesystem type for display
            let fs = crate::filesystem::FILESYSTEM.lock();
            let fs_type = fs.get_filesystem_type();
            drop(fs);
            
            syscall::sys_print("[i] Reading file: ")?;
            syscall::sys_print(filename)?;
            // syscall::sys_print(" (from ")?;
            // match fs_type {
            //     crate::filesystem::FilesystemType::Fat32 => syscall::sys_print("FAT32")?,
            //     crate::filesystem::FilesystemType::Ext2 => syscall::sys_print("ext2")?,
            //     crate::filesystem::FilesystemType::Unknown => syscall::sys_print("Unknown")?,
            // }
            // syscall::sys_print(" filesystem)\n")?;
            
            if let Ok(content_str) = core::str::from_utf8(&content) {
                syscall::sys_print(" ontent:\n")?;
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
            syscall::sys_print("[x] File '")?;
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
        syscall::SysCallResult::Error(_) => Err("Syscall failed"),
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
        syscall::SysCallResult::Error(_) => Err("Syscall failed"),
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
        syscall::SysCallResult::Error(_) => Err("Syscall failed"),
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

fn cmd_pwd() -> Result<(), &'static str> {
    ensure_cwd_initialized();
    unsafe {
        syscall::sys_print(&CURRENT_PATH)?;
    }
    syscall::sys_print("\n")?;
    Ok(())
}

fn cmd_touch(path: &str) -> Result<(), &'static str> {
    match crate::filesystem::FILESYSTEM.lock().create_file(path) {
        Ok(entry) => {
            syscall::sys_print("Created file '")?;
            syscall::sys_print(&entry.name)?; // Name from returned FileEntry might be just the basename
            syscall::sys_print("' at path '")?;
            syscall::sys_print(path)?;
            syscall::sys_print("'.\n")?;
            Ok(())
        }
        Err(e) => {
            print_filesystem_error(&e);
            Err("Failed to create file")
        }
    }
}

fn cmd_mkdir(path: &str) -> Result<(), &'static str> {
    match crate::filesystem::FILESYSTEM.lock().create_directory(path) {
        Ok(entry) => {
            syscall::sys_print("Created directory '")?;
            syscall::sys_print(&entry.name)?;
            syscall::sys_print("' at path '")?;
            syscall::sys_print(path)?;
            syscall::sys_print("'.\n")?;
            Ok(())
        }
        Err(e) => {
            print_filesystem_error(&e);
            Err("Failed to create directory")
        }
    }
}

fn cmd_rm(path: &str) -> Result<(), &'static str> { // For files
    match crate::filesystem::FILESYSTEM.lock().delete_file(path) {
        Ok(()) => {
            syscall::sys_print("Removed file '")?;
            syscall::sys_print(path)?;
            syscall::sys_print("'.\n")?;
            Ok(())
        }
        Err(e) => {
            print_filesystem_error(&e);
            Err("Failed to remove file")
        }
    }
}

fn cmd_rmdir(path: &str) -> Result<(), &'static str> { // For directories
    match crate::filesystem::FILESYSTEM.lock().delete_directory(path) {
        Ok(()) => {
            syscall::sys_print("Removed directory '")?;
            syscall::sys_print(path)?;
            syscall::sys_print("'.\n")?;
            Ok(())
        }
        Err(e) => {
            print_filesystem_error(&e);
            Err("Failed to remove directory")
        }
    }
}

fn cmd_cd(path_arg: &str) -> Result<(), &'static str> {
    let new_path_str = resolve_path(path_arg);
    // Optimistic CD: we just set the path.
    // Validation would ideally occur here by checking if new_path_str is a directory.
    // For now, we update and print.
    unsafe {
        CURRENT_PATH.clear();
        if CURRENT_PATH.push_str(&new_path_str).is_err() {
            syscall::sys_print("Error: New path too long for CWD buffer.\n")?;
            return Err("Path too long");
        }
    }
    // syscall::sys_print("Current directory: ")?; // cmd_pwd can be used
    // syscall::sys_print(&new_path_str)?;
    // syscall::sys_print("\n")?;
    Ok(())
}

// === ELF OPERATIONS ===

fn cmd_elf_info(filename: &str) -> Result<(), &'static str> {
    syscall::sys_print("[i] ELF Binary Analysis: ")?;
    syscall::sys_print(filename)?;
    syscall::sys_print("\n")?;
    
    // Read file from filesystem
    match crate::filesystem::read_file(filename) {
        Ok(file_data) => {
            // Call ELF info syscall
            let result = syscall::syscall_handler(
                crate::syscall::elinos::SYS_ELF_INFO,
                file_data.as_ptr() as usize,
                file_data.len(),
                0,
                0,
            );
            
            match result {
                syscall::SysCallResult::Success(_) => Ok(()),
                syscall::SysCallResult::Error(_) => {
                    syscall::sys_print("[x] ELF analysis failed\n")?;
                    Err("ELF analysis failed")
                }
            }
        }
        Err(_) => {
            syscall::sys_print("[x] File not found: ")?;
            syscall::sys_print(filename)?;
            syscall::sys_print("\n")?;
            Err("File not found")
        }
    }
}

fn cmd_elf_load(filename: &str) -> Result<(), &'static str> {
    syscall::sys_print("[i] Loading ELF Binary: ")?;
    syscall::sys_print(filename)?;
    syscall::sys_print("\n")?;
    
    // Read file from filesystem
    match crate::filesystem::read_file(filename) {
        Ok(file_data) => {
            // Call ELF load syscall
            let result = syscall::syscall_handler(
                crate::syscall::elinos::SYS_LOAD_ELF,
                file_data.as_ptr() as usize,
                file_data.len(),
                0,
                0,
            );
            
            match result {
                syscall::SysCallResult::Success(entry_point) => {
                    syscall::sys_print("[o] ELF loaded successfully!\n")?;
                    syscall::sys_print("   Entry point: 0x")?;
                    let _ = syscall::sys_print_hex(entry_point as u32, 8);
                    syscall::sys_print("\n")?;
                    Ok(())
                }
                syscall::SysCallResult::Error(_) => {
                    syscall::sys_print("[x] ELF loading failed\n")?;
                    Err("ELF loading failed")
                }
            }
        }
        Err(_) => {
            syscall::sys_print("[x] File not found: ")?;
            syscall::sys_print(filename)?;
            syscall::sys_print("\n")?;
            Err("File not found")
        }
    }
}

// Unified ELF execution function - parse, load, and execute in one step
fn cmd_execute_elf(filename: &str, file_data: &[u8]) -> Result<(), &'static str> {
    syscall::sys_print("[i] Executing: ")?;
    syscall::sys_print(filename)?;
    syscall::sys_print("\n")?;
    
    // Handle ELF execution (like "./hello_simple")
    if filename.starts_with("./") || filename.starts_with("/") {
        let elf_filename = if filename.starts_with("./") {
            &filename[2..]
        } else {
            &filename[1..]
        };
        
        console_println!("[i] Executing: {}", filename);
        
        // Use the new ELF file reader that supports larger files
        match crate::filesystem::read_elf_file(elf_filename) {
            Ok(elf_data) => {
                console_println!("[i] Read {} bytes from {}", elf_data.len(), elf_filename);
                
                let loader = crate::elf::ElfLoader::new();
                
                // Load the ELF binary
                match loader.load_elf(&elf_data) {
                    Ok(loaded_elf) => {
                        console_println!("[o] ELF loaded, attempting execution...");
                        
                        // Execute the loaded ELF
                        match loader.execute_elf(&loaded_elf) {
                            Ok(()) => {
                                console_println!("[o] ELF execution completed successfully!");
                            }
                            Err(err) => {
                                console_println!("[x] Execution failed: {:?}", err);
                            }
                        }
                    }
                    Err(err) => {
                        console_println!("[x] ELF loading failed: {:?}", err);
                    }
                }
            }
            Err(err) => {
                console_println!("[x] Failed to read ELF file '{}': {}", elf_filename, err);
            }
        }
        return Ok(());
    }
    
    // Execute the ELF via syscall
    let result = syscall::syscall_handler(
        crate::syscall::elinos::SYS_EXEC_ELF,
        file_data.as_ptr() as usize,
        file_data.len(),
        0,
        0,
    );
    
    match result {
        syscall::SysCallResult::Success(entry_point) => {
            syscall::sys_print("[o] Program completed successfully!\n")?;
            Ok(())
        }
        syscall::SysCallResult::Error(_) => {
            syscall::sys_print("[x] Execution failed\n")?;
            Err("Execution failed")
        }
    }
}

fn cmd_elf_exec(filename: &str) -> Result<(), &'static str> {
    syscall::sys_print("[i] Executing ELF Binary: ")?;
    syscall::sys_print(filename)?;
    syscall::sys_print("\n")?;
    
    // Read file from filesystem
    match crate::filesystem::read_file(filename) {
        Ok(file_data) => {
            // Call ELF exec syscall
            let result = syscall::syscall_handler(
                crate::syscall::elinos::SYS_EXEC_ELF,
                file_data.as_ptr() as usize,
                file_data.len(),
                0,
                0,
            );
            
            match result {
                syscall::SysCallResult::Success(entry_point) => {
                    syscall::sys_print("[o] ELF execution completed successfully!\n")?;
                    syscall::sys_print("   Entry point was: 0x")?;
                    let _ = syscall::sys_print_hex(entry_point as u32, 8);
                    syscall::sys_print("\n")?;
                    Ok(())
                }
                syscall::SysCallResult::Error(_) => {
                    syscall::sys_print("[x] ELF execution failed\n")?;
                    Err("ELF execution failed")
                }
            }
        }
        Err(_) => {
            syscall::sys_print("[x] File not found: ")?;
            syscall::sys_print(filename)?;
            syscall::sys_print("\n")?;
            Err("File not found")
        }
    }
}

/// Show heap usage information
pub fn cmd_heap() -> Result<(), &'static str> {
    syscall::sys_print("[i] Heap Status:\n")?;
    syscall::sys_print("================\n")?;
    
    let (used, total, available) = memory::get_heap_usage();
    
    syscall::sys_print("Total heap size: ")?;
    show_number_kb(total);
    syscall::sys_print("\nUsed heap: ")?;
    show_number_kb(used);
    syscall::sys_print("\nAvailable heap: ")?;
    show_number_kb(available);
    
    let usage_percent = if total > 0 { (used * 100) / total } else { 0 };
    syscall::sys_print("\nUsage: ")?;
    show_number(usage_percent);
    syscall::sys_print("%\n")?;
    
    if available == 0 {
        syscall::sys_print("[!]  WARNING: Heap is completely exhausted!\n")?;
    } else if usage_percent > 90 {
        syscall::sys_print("[!]  WARNING: Heap usage is very high!\n")?;
    }
    
    Ok(())
}

/// Reset heap for testing (dangerous)
pub fn cmd_heap_reset() -> Result<(), &'static str> {
    syscall::sys_print("[!]  DANGER: This will reset the heap position!\n")?;
    syscall::sys_print("This may cause memory corruption if other allocations are active.\n")?;
    syscall::sys_print("Only use for testing purposes.\n")?;
    syscall::sys_print("Resetting heap...\n")?;
    
    memory::reset_heap_for_testing();
    
    syscall::sys_print("[o] Heap position reset to 0\n")?;
    
    // Show new heap status
    cmd_heap()
} 