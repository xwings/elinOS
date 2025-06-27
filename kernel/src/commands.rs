use crate::syscall;
use crate::filesystem::traits::{FileSystem, FilesystemError, FileEntry};
use crate::memory::{self, BufferUsage};
use heapless::String;
use core::fmt::Write;
use crate::{UART, console_println, console_print};

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
                console_println!("CRITICAL: Failed to initialize CWD to root!");
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
                    console_println!("Warning: Path too long or complex, may be truncated.");
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
    console_println!("Error: {}", err_buf);
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
        "mmap" => cmd_mmap(),
        "devices" => cmd_devices(),
        "graphics" => cmd_graphics(),
        "gfxtest" => cmd_graphics_test(),
        "syscall" => cmd_syscall(),
        "fscheck" => cmd_fscheck(),
        "config" => cmd_config(),
        
        // File operations (working via modular filesystem)
        "ls" => cmd_ls(None),
        "cat" => {
            console_println!("Usage: cat <filename>");
            Ok(())
        },
        "echo" => cmd_echo(""),
        "pwd" => cmd_pwd(),

        // New file/dir operations
        "touch" => {
            console_println!("Usage: touch <filename>");
            Ok(())
        },
        "mkdir" => {
            console_println!("Usage: mkdir <dirname>");
            Ok(())
        },
        "rm" => {
            console_println!("Usage: rm <filename>");
            Ok(())
        },
        "rmdir" => {
            console_println!("Usage: rmdir <dirname>");
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
                console_println!("Usage: cat <filename>");
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
                console_println!("Usage: touch <filename>");
                Ok(())
            } else {
                let full_path = resolve_path(path_arg);
                cmd_touch(&full_path)
            }
        },
        cmd if cmd.starts_with("mkdir ") => {
            let path_arg = cmd.strip_prefix("mkdir ").unwrap_or("").trim();
            if path_arg.is_empty() {
                console_println!("Usage: mkdir <dirname>");
                Ok(())
            } else {
                let full_path = resolve_path(path_arg);
                cmd_mkdir(&full_path)
            }
        },
        cmd if cmd.starts_with("rm ") => {
            let path_arg = cmd.strip_prefix("rm ").unwrap_or("").trim();
            if path_arg.is_empty() {
                console_println!("Usage: rm <filename>");
                Ok(())
            } else {
                let full_path = resolve_path(path_arg);
                cmd_rm(&full_path)
            }
        },
        cmd if cmd.starts_with("rmdir ") => {
            let path_arg = cmd.strip_prefix("rmdir ").unwrap_or("").trim();
            if path_arg.is_empty() {
                console_println!("Usage: rmdir <dirname>");
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
                    console_println!("[i] Debug: File size: {}", file_data.len() as u64);
                    
                    if file_data.len() >= 4 {
                        console_println!("[i] Debug: First 4 bytes: ");
                        for i in 0..4 {
                            console_print!("{:02x} ", file_data[i] as u32);
                        }
                        console_println!();
                    }
                    
                    // Check if it's an ELF file by looking at magic bytes
                    if file_data.len() >= 4 && &file_data[0..4] == b"\x7fELF" {
                        cmd_execute_elf(&full_path, &file_data)
                    } else {
                        console_println!("[x] Not an executable file: {}", command);
                        console_println!("(expected ELF magic: 7f 45 4c 46)");
                        Err("Not an executable")
                    }
                }
                Err(_) => {
                    console_println!("Unknown command: {}", command);
                    console_println!("Type 'help' for available commands.");
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
        "help", "version", "memory", "heap", "mmap", "devices", "syscall", "fscheck", "config",
        "ls", "cat", "echo", "pwd",
        "touch", "mkdir", "rm", "rmdir", "cd",
        "shutdown", "reboot"
    ]
}

// === INDIVIDUAL COMMAND IMPLEMENTATIONS ===

pub fn cmd_help() -> Result<(), &'static str> {
    console_println!("[i] ElinOS Commands");
    console_println!("===============================================");
    console_println!();
    
    console_println!("[i] System Information:");
    console_println!("  help            - Show this help message");
    console_println!("  version         - Show kernel version and features");
    console_println!("  memory          - Show memory regions and allocator statistics");
    console_println!("  heap            - Show heap usage information");
    console_println!("  mmap            - Show memory mapping information");
    console_println!("  devices         - List detected VirtIO devices");
    console_println!("  graphics        - Show graphics information");
    console_println!("  gfxtest         - Test graphics drawing");
    console_println!("  syscall         - Show system call information");
    console_println!("  fscheck         - Check filesystem status and metadata");
    console_println!("  config          - Show system configuration");

    console_println!();
    console_println!("[i]  Filesystem Operations:");
    console_println!("  ls [path]       - List files/dirs (default: current directory)");
    console_println!("  cat <path>      - Display file contents");
    console_println!("  echo [message]  - Print a message (newline if no message)");
    console_println!("  pwd             - Print current working directory");
    console_println!("  touch <path>    - Create an empty file at the specified path");
    console_println!("  mkdir <path>    - Create a directory at the specified path");
    console_println!("  rm <path>       - Remove a file at the specified path");
    console_println!("  rmdir <path>    - Remove an empty directory at the specified path");
    console_println!("  cd [path]       - Change directory (default: root, use '/', '..')");
    
    console_println!();
    console_println!("[i] Program Execution:");
    console_println!("  hello_simple    - Execute ELF binary directly by name");
    console_println!("  ./hello_simple  - Execute with explicit relative path");
    console_println!("  /programs/hello - Execute with absolute path");
    
    console_println!();
    console_println!("[i] System Control:");
    console_println!("  shutdown        - Shutdown the system via SBI");
    console_println!("  reboot          - Reboot the system via SBI");
    
    Ok(())
}

pub fn cmd_config() -> Result<(), &'static str> {
    console_println!("[i] Dynamic System Configuration");
    console_println!("=====================================");
    console_println!();
    
    // Get memory statistics
    let mem_stats = memory::get_memory_stats();
    
    console_println!("[i] Hardware Detection Results:");
    
    console_print!("  Total RAM: ");
    show_number_mb(mem_stats.detected_ram_size);
    console_println!(" MB ");
    
    console_print!("  Memory Regions: ");
    show_number(mem_stats.regions_detected);
    console_println!(" (discovered via SBI)");
    
    console_print!("  Allocator Mode: ");
    match mem_stats.allocator_mode {
        memory::AllocatorMode::SimpleHeap => console_println!("Simple Heap (low memory)"),
        memory::AllocatorMode::TwoTier => console_println!("Two-Tier (Buddy + Slab)"),
        memory::AllocatorMode::Hybrid => console_println!("Hybrid (adaptive)"),
    }
    
    console_println!();
    console_println!("[i] Calculated Memory Allocations:");
    
    console_print!("  Kernel Heap: ");
    show_number_kb(mem_stats.heap_size);
    console_println!(" KB (scaled to RAM size)");
    
    console_print!("  Heap Used: ");
    show_number_kb(mem_stats.heap_used);
    console_println!(" KB");
    
    console_print!("  Heap Utilization: ");
    if mem_stats.heap_size > 0 {
        let utilization = (mem_stats.heap_used * 100) / mem_stats.heap_size;
        show_number(utilization);
        console_println!("%");
    } else {
        console_println!("N/A");
    }
    
    console_println!();
    console_println!("[i] Dynamic Buffer Sizes:");
    
    let sector_buf_size = memory::get_optimal_buffer_size(BufferUsage::SectorIO);
    console_print!("  Sector I/O: ");
    show_number(sector_buf_size);
    console_println!(" bytes");
    
    let file_buf_size = memory::get_optimal_buffer_size(BufferUsage::FileRead);
    console_print!("  File Reading: ");
    show_number_kb(file_buf_size);
    console_println!(" KB");
    
    let cmd_buf_size = memory::get_optimal_buffer_size(BufferUsage::Command);
    console_print!("  Command Input: ");
    show_number(cmd_buf_size);
    console_println!(" bytes");
    
    let max_file_size = memory::get_max_file_size();
    console_print!("  Max File Size: ");
    show_number_kb(max_file_size);
    console_println!(" KB");
    
    Ok(())
}

// Helper functions for number display without format! macro
fn show_number(mut num: usize) {
    if num == 0 {
        console_print!("0");
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
            console_print!("{}", s);
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

    console_println!("Listing for target '{}':", list_target_path);

    // Use the new path-aware directory listing
    match crate::filesystem::list_directory(&list_target_path) {
        Ok(files) => {
            // Get filesystem info for display
            let fs = crate::filesystem::FILESYSTEM.lock();
            let fs_type = fs.get_filesystem_type();
            let fs_info = fs.get_filesystem_info();
            drop(fs);
            
            if files.is_empty() {
                console_println!("(No files found)");
            } else {
                for (name, _size, is_directory) in &files {
                    if *is_directory {
                        console_print!("  DIR   ");
                    } else {
                        console_print!("  FILE  ");
                    }
                    console_println!("{}", name.as_str());
                }
                console_print!("\nTotal files: ");
                show_number(files.len());
                console_println!();
            }
            
            Ok(())
        }
        Err(_) => {
            console_println!("Failed to list directory");
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
            
            console_println!("[i] Reading file: {}", filename);
            
            if let Ok(content_str) = core::str::from_utf8(&content) {
                console_println!(" content:");
                console_print!("{}", content_str);
                console_println!();
            } else {
                console_println!("(Binary file - ");
                console_println!("bytes count not displayed");
                console_println!(")");
            }
            
            Ok(())
        }
        Err(_) => {
            console_println!("[x] File '{}' not found", filename);
            Err("File not found")
        }
    }
}

pub fn cmd_syscall() -> Result<(), &'static str> {
    console_println!("System Call Information:");
     
    console_println!("Currently Implemented System Calls:");
    console_println!("  File I/O Operations:");
    console_println!("    SYS_WRITE (64)     - Write to file descriptor");
    console_println!("    SYS_READ (63)      - Read from file descriptor");
    console_println!("    SYS_OPENAT (56)    - Open file (modern Linux openat)");
    console_println!("    SYS_CLOSE (57)     - Close file descriptor");
    console_println!("    SYS_GETDENTS64 (61) - List directory entries");

    console_println!("  Memory Management:");
    console_println!("    SYS_GETMEMINFO (960) - Memory information (elinOS)");

    console_println!("  Process Management:");
    console_println!("    SYS_EXIT (93)      - Exit process");
    console_println!("    SYS_GETPID (172)   - Get process ID");
    console_println!("    SYS_GETPPID (173)  - Get parent process ID");
    console_println!("    SYS_FORK (220)     - Create child process");
    console_println!("    SYS_WAIT4 (260)    - Wait for child process");

    console_println!("  Device Management:");
    console_println!("    SYS_GETDEVICES (950) - Device information (elinOS)");

    console_println!("  elinOS-Specific (System Control):");
    console_println!("    SYS_ELINOS_VERSION (902)  - Show version");
    console_println!("    SYS_ELINOS_SHUTDOWN (903) - Shutdown system");
    console_println!("    SYS_ELINOS_REBOOT (904)   - Reboot system");

    console_println!();
    console_println!("Numbers in parentheses are Linux-compatible syscall numbers.");
    console_println!("This makes elinOS easier to understand for Linux developers!");
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
    console_println!("{}", message);
    Ok(())
}

pub fn cmd_fscheck() -> Result<(), &'static str> {
    match crate::filesystem::check_filesystem() {
        Ok(()) => Ok(()),
        Err(_) => {
            console_println!("Failed to check filesystem");
            Err("Failed to check filesystem")
        }
    }
}

fn cmd_pwd() -> Result<(), &'static str> {
    ensure_cwd_initialized();
    unsafe {
        console_println!("{}", CURRENT_PATH);
    }
    Ok(())
}

fn cmd_touch(path: &str) -> Result<(), &'static str> {
    match crate::filesystem::FILESYSTEM.lock().create_file(path) {
        Ok(entry) => {
            console_println!("Created file '{}' at path '{}'.", entry.name, path);
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
            console_println!("Created directory '{}' at path '{}'.", entry.name, path);
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
            console_println!("[o] Removed file '{}'.", path);
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
            console_println!("[o] Removed directory '{}'.", path);
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
            console_println!("Error: New path too long for CWD buffer.");
            return Err("Path too long");
        }
    }
    Ok(())
}

// === ELF OPERATIONS ===

fn cmd_elf_info(filename: &str) -> Result<(), &'static str> {
    console_println!("[i] ELF Binary Analysis: {}", filename);
    
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
                    console_println!("[x] ELF analysis failed");
                    Err("ELF analysis failed")
                }
            }
        }
        Err(_) => {
            console_println!("[x] File not found: {}", filename);
            Err("File not found")
        }
    }
}

fn cmd_elf_load(filename: &str) -> Result<(), &'static str> {
    console_println!("[i] Loading ELF Binary: {}", filename);
    
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
                    console_println!("[o] ELF loaded successfully!");
                    console_println!("   Entry point: 0x{:08x}", entry_point as u32);
                    Ok(())
                }
                syscall::SysCallResult::Error(_) => {
                    console_println!("[x] ELF loading failed");
                    Err("ELF loading failed")
                }
            }
        }
        Err(_) => {
            console_println!("[x] File not found: {}", filename);
            Err("File not found")
        }
    }
}

// Unified ELF execution function - parse, load, and execute in one step
fn cmd_execute_elf(filename: &str, file_data: &[u8]) -> Result<(), &'static str> {
    console_println!("[i] Executing: {}", filename);
    
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
                        match crate::elf::execute_elf(&loaded_elf) {
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
            console_println!("[o] Program completed successfully!");
            Ok(())
        }
        syscall::SysCallResult::Error(_) => {
            console_println!("[x] Execution failed");
            Err("Execution failed")
        }
    }
}

fn cmd_elf_exec(filename: &str) -> Result<(), &'static str> {
    console_println!("[i] Executing ELF Binary: {}", filename);
    
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
                    console_println!("[o] ELF execution completed successfully!");
                    console_println!("   Entry point was: 0x{:08x}", entry_point as u32);
                    Ok(())
                }
                syscall::SysCallResult::Error(_) => {
                    console_println!("[x] ELF execution failed");
                    Err("ELF execution failed")
                }
            }
        }
        Err(_) => {
            console_println!("[x] File not found: {}", filename);
            Err("File not found")
        }
    }
}

/// Show heap usage information
pub fn cmd_heap() -> Result<(), &'static str> {
    console_println!("[i] Heap Status:");
    console_println!("================");
    
    let (used, total, available) = memory::get_heap_usage();
    
    console_print!("Total heap size: ");
    show_number_kb(total);
    console_print!("\nUsed heap: ");
    show_number_kb(used);
    console_print!("\nAvailable heap: ");
    show_number_kb(available);
    
    let usage_percent = if total > 0 { (used * 100) / total } else { 0 };
    console_print!("\nUsage: ");
    show_number(usage_percent);
    console_println!("%");
    
    if available == 0 {
        console_println!("[!]  WARNING: Heap is completely exhausted!");
    } else if usage_percent > 90 {
        console_println!("[!]  WARNING: Heap usage is very high!");
    }
    
    Ok(())
}

/// Reset heap for testing (dangerous)
pub fn cmd_heap_reset() -> Result<(), &'static str> {
    console_println!("[!]  DANGER: This will reset the heap position!");
    console_println!("This may cause memory corruption if other allocations are active.");
    console_println!("Only use for testing purposes.");
    console_println!("Resetting heap...");
    
    memory::reset_heap_for_testing();
    
    console_println!("[o] Heap position reset to 0");
    
    // Show new heap status
    cmd_heap()
}

/// Show memory mapping information
pub fn cmd_mmap() -> Result<(), &'static str> {
    console_println!("=== Memory Mapping Information ===");
    crate::memory::mapping::show_memory_mappings();
    Ok(())
}

/// Show graphics information
pub fn cmd_graphics() -> Result<(), &'static str> {
    console_println!("=== Graphics System Information ===");
    
    if crate::graphics::is_graphics_available() {
        console_println!("[o] Graphics system is available");
        
        if let Some((width, height, bpp, size)) = crate::graphics::get_framebuffer_info() {
            console_println!("Framebuffer Information:");
            console_println!("  Resolution: {}x{}", width, height);
            console_println!("  Bits per pixel: {}", bpp);
            console_println!("  Size: {} KB", size / 1024);
            console_println!("  Total pixels: {}", width * height);
        } else {
            console_println!("[!] Graphics available but no framebuffer info");
        }
    } else {
        console_println!("[!] Graphics system is not available");
        console_println!("    This may be due to:");
        console_println!("    - Insufficient memory for framebuffer allocation");
        console_println!("    - Memory management API failure");
        console_println!("    - Graphics initialization error");
    }
    
    Ok(())
}

/// Test graphics drawing
pub fn cmd_graphics_test() -> Result<(), &'static str> {
    console_println!("=== Graphics Drawing Test ===");
    
    if !crate::graphics::is_graphics_available() {
        console_println!("[!] Graphics system is not available");
        return Err("Graphics not available");
    }
    
    console_println!("[i] Running graphics tests...");
    let mut test_failures = 0;
    let mut total_tests = 0;
    
    // Test 1: Clear screen to black
    console_println!("Test 1: Clearing screen to black...");
    total_tests += 1;
    match crate::graphics::clear_screen(0x00000000) {
        Ok(_) => console_println!("[o] Screen cleared successfully"),
        Err(e) => {
            console_println!("[x] FAILED: Clear screen failed: {}", e);
            test_failures += 1;
        }
    }
    
    // Test 2: Draw some colored pixels
    console_println!("Test 2: Drawing colored pixels...");
    let colors = [
        ("Red", 0xFF0000FF),
        ("Green", 0x00FF00FF),
        ("Blue", 0x0000FFFF),
        ("Yellow", 0xFFFF00FF),
        ("Magenta", 0xFF00FFFF),
        ("Cyan", 0x00FFFFFF),
        ("White", 0xFFFFFFFF),
    ];
    
    for (i, &(color_name, color)) in colors.iter().enumerate() {
        let x = (i * 80) as u32; // Fit in 640px width (0, 80, 160, 240, 320, 400, 480)
        let y = 200;
        total_tests += 1;
        
        match crate::graphics::draw_pixel(x, y, color) {
            Ok(_) => console_println!("[o] Drew {} pixel at ({}, {})", color_name, x, y),
            Err(e) => {
                console_println!("[x] FAILED: {} pixel at ({}, {}) - {}", color_name, x, y, e);
                test_failures += 1;
            }
        }
    }
    
    // Test 3: Draw rectangles
    console_println!("Test 3: Drawing rectangles...");
    let rects = [
        ("Red", 50, 300, 120, 50, 0xFF0000FF),
        ("Green", 220, 300, 120, 50, 0x00FF00FF),
        ("Blue", 390, 300, 120, 50, 0x0000FFFF),
    ];
    
    for &(color_name, x, y, w, h, color) in &rects {
        total_tests += 1;
        match crate::graphics::draw_rect(x, y, w, h, color) {
            Ok(_) => console_println!("[o] Drew {} rectangle {}x{} at ({}, {})", color_name, w, h, x, y),
            Err(e) => {
                console_println!("[x] FAILED: {} rectangle at ({}, {}) - {}", color_name, x, y, e);
                test_failures += 1;
            }
        }
    }
    
    // Test 4: Boundary tests (should fail gracefully)
    console_println!("Test 4: Boundary tests (should fail gracefully)...");
    let boundary_tests = [
        ("Out of bounds pixel", 1000, 1000),
        ("Edge pixel", 639, 479), // Should succeed (last valid pixel)
        ("Just out of bounds", 640, 480), // Should fail
    ];
    
    for &(test_name, x, y) in &boundary_tests {
        total_tests += 1;
        match crate::graphics::draw_pixel(x, y, 0xFFFFFFFF) {
            Ok(_) => console_println!("[o] {} at ({}, {}) - SUCCESS", test_name, x, y),
            Err(e) => {
                if test_name.contains("Out of bounds") || test_name.contains("Just out") {
                    console_println!("[o] {} at ({}, {}) - CORRECTLY REJECTED: {}", test_name, x, y, e);
                } else {
                    console_println!("[x] FAILED: {} at ({}, {}) - {}", test_name, x, y, e);
                    test_failures += 1;
                }
            }
        }
    }
    
    // Final results
    console_println!();
    console_println!("=== Test Results ===");
    console_println!("Total tests: {}", total_tests);
    console_println!("Passed: {}", total_tests - test_failures);
    console_println!("Failed: {}", test_failures);
    
    if test_failures == 0 {
        console_println!("[o] ALL TESTS PASSED! Graphics system is working perfectly.");
        Ok(())
    } else {
        console_println!("[x] {} TESTS FAILED! Graphics system has issues.", test_failures);
        Err("Graphics tests failed")
    }
} 