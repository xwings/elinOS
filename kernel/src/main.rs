#![no_std]
#![no_main]

use core::panic::PanicInfo;
use core::arch::asm;
use spin::Mutex;
use heapless::{String, Vec};

// Module declarations
pub mod console;
pub mod uart;
pub mod commands;
pub mod sbi;
pub mod memory;
pub mod filesystem;  // Now points to filesystem/mod.rs
pub mod elf;
pub mod syscall;
pub mod virtio;
pub mod trap;  // Add trap module
pub mod graphics; // Simple framebuffer graphics

use crate::uart::Uart;

// Global UART instance
pub static UART: Mutex<Uart> = Mutex::new(Uart::new());

/// Escape sequences for terminal input
#[derive(Debug, Clone, Copy)]
enum EscapeSequence {
    UpArrow,
    DownArrow,
    LeftArrow,
    RightArrow,
}

/// Shell constants
const MAX_COMMAND_LEN: usize = 1024;
const MAX_HISTORY_ENTRIES: usize = 100;
const HISTORY_FILE_PATH: &str = "/.shell_history";

/// Shell state for history and input management
pub struct ShellState {
    command_buffer: Vec<u8, MAX_COMMAND_LEN>,
    history: Vec<String<MAX_COMMAND_LEN>, MAX_HISTORY_ENTRIES>,
    history_index: Option<usize>,
    current_input: String<MAX_COMMAND_LEN>,
}

impl ShellState {
    fn new() -> Self {
        Self {
            command_buffer: Vec::new(),
            history: Vec::new(),
            history_index: None,
            current_input: String::new(),
        }
    }
}

// Global shell state
static SHELL_STATE: Mutex<ShellState> = Mutex::new(ShellState {
    command_buffer: heapless::Vec::new(),
    history: heapless::Vec::new(),
    history_index: None,
    current_input: heapless::String::new(),
});

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    // Print the panic message
    console_println!("[x]  KERNEL PANIC: {}", info.message());
    
    if let Some(location) = info.location() {
        console_println!("[i] Location: {}:{}:{}", location.file(), location.line(), location.column());
    }
    
    loop {
        unsafe {
            asm!("wfi");
        }
    }
}

#[link_section = ".text.boot"]
#[no_mangle]
pub extern "C" fn _start() -> ! {
    unsafe {
        asm!(
            "la sp, {stack_top}",
            "li t0, 0x80200000",
            "mv sp, t0",
            "j {main}",
            stack_top = sym _STACK_TOP,
            main = sym main,
            options(noreturn)
        );
    }
}

#[no_mangle]
pub extern "C" fn main() -> ! {
    console_println!();
    console_println!();
    console_println!("elinOS Starting...");

    // Initialize trap handling (CRITICAL: must be early!)
    trap::init_trap_handling();
    console_println!("[o] Trap handling ready");

    // Initialize console system
    if let Err(e) = console::init_console() {
        panic!("Failed to initialize console: {}", e);
    }
    
    // Initialize memory management
    {
        let mut memory_mgr = memory::MEMORY_MANAGER.lock();
        memory_mgr.init();
    }
    console_println!("[o] Memory management ready");

    // Initialize Virtual Memory Management (Software MMU)
    if let Err(e) = memory::mmu::init_mmu() {
        console_println!("[x] Virtual Memory initialization failed: {}", e);
        console_println!("[!] Continuing in physical memory mode");
    } else {
        console_println!("[o] Virtual Memory Management enabled!");
    }

    // Initialize VirtIO block device  
    console_println!("[i] Initializing VirtIO block device...");
    if let Err(_) = virtio::init_virtio_memory() {
        console_println!("[x] Failed to initialize VirtIO memory manager");
    }
    
    if let Err(e) = virtio::init_virtio_blk() {
        console_println!("[x] VirtIO disk initialization failed: {}", e);
    } else {
        console_println!("[o] VirtIO disk ready");
    }

    // Initialize filesystem
    match filesystem::init_filesystem() {
        Ok(()) => {
            // console_println!("[o] Filesystem initialization successful!");
        }
        Err(e) => {
            console_println!("[x] Filesystem initialization failed: {:?}", e);
        }
    }

    // Initialize graphics (optional)
    match graphics::init_graphics() {
        Ok(_) => console_println!("[o] Graphics system initialized"),
        Err(e) => console_println!("[!] Graphics initialization failed: {}", e),
    }
    
    console_println!();
    
    // Load shell history and start enhanced shell
    load_shell_history();
    show_welcome();
    enhanced_shell_loop();
}

/// Load command history from filesystem
fn load_shell_history() {
    if let Ok(data) = filesystem::read_file(HISTORY_FILE_PATH) {
        if let Ok(content) = core::str::from_utf8(&data) {
            let mut shell_state = SHELL_STATE.lock();
            shell_state.history.clear();
            
            for line in content.lines() {
                let trimmed = line.trim();
                if !trimmed.is_empty() {
                    if let Ok(cmd_string) = String::try_from(trimmed) {
                        if shell_state.history.push(cmd_string).is_err() {
                            break; // History buffer is full
                        }
                    }
                }
            }
        }
    }
    // Ignore errors - history file might not exist on first run
}

/// Save command history to filesystem
fn save_shell_history() {
    let shell_state = SHELL_STATE.lock();
    let mut content = String::<4096>::new();
    
    for cmd in &shell_state.history {
        if content.push_str(cmd).is_ok() {
            let _ = content.push('\n');
        }
    }
    
    // Write to filesystem (ignore errors)
    let _ = filesystem::write_file(HISTORY_FILE_PATH, &content);
}

/// Show welcome message
fn show_welcome() {
    console_println!("=====================================");
    console_println!("          Welcome to elinOS!         ");
    console_println!("=====================================");
    console_println!("  RISC-V64 Operating System written in Rust");
    console_println!();
    console_println!("  Type 'help' for available commands");
    console_println!("  Type 'version' for system information");
    console_println!("  Type 'shutdown' to exit");
    console_println!();
}

/// Enhanced shell loop with history and navigation
pub fn enhanced_shell_loop() -> ! {
    loop {
        // Show prompt
        console_print!("elinOS> ");
        
        // Also draw prompt to framebuffer if graphics are available
        let _ = graphics::draw_shell_prompt();
        
        // Read command with enhanced features
        if let Ok(command) = read_enhanced_command() {
            if !command.is_empty() {
                // Add to history before processing
                add_to_history(&command);
                
                // Process command
                if let Err(e) = process_enhanced_command(&command) {
                    if e == "exit_shell" {
                        console_println!("Goodbye!");
                        break;
                    } else {
                        console_println!("Error: {}", e);
                    }
                }
            }
        }
        
        console_println!();
    }
    
    // This should never be reached due to shutdown/reboot commands
    panic!("Shell loop exited unexpectedly");
}

/// Read command with history navigation and editing support
fn read_enhanced_command() -> Result<String<MAX_COMMAND_LEN>, &'static str> {
    let mut shell_state = SHELL_STATE.lock();
    shell_state.command_buffer.clear();
    shell_state.history_index = None;
    shell_state.current_input.clear();
    
    loop {
        let ch = read_char();
        
        match ch {
            b'\r' | b'\n' => {
                console_println!();
                let command_str = core::str::from_utf8(&shell_state.command_buffer)
                    .map_err(|_| "Invalid UTF-8 in command")?;
                return String::try_from(command_str.trim())
                    .map_err(|_| "Command too long");
            }
            b'\x08' | b'\x7f' => { // Backspace or DEL
                if !shell_state.command_buffer.is_empty() {
                    shell_state.command_buffer.pop();
                    console_print!("\x08 \x08"); // Move back, print space, move back
                    
                    // Update current_input if not navigating history
                    if shell_state.history_index.is_none() {
                        // Clone the buffer to avoid borrow checker issues
                        let buffer_copy = shell_state.command_buffer.clone();
                        if let Ok(current_str) = core::str::from_utf8(&buffer_copy) {
                            shell_state.current_input.clear();
                            let _ = shell_state.current_input.push_str(current_str);
                        }
                    }
                }
            }
            b'\x1b' => { // ESC - start of escape sequence
                if let Ok(Some(sequence)) = read_escape_sequence() {
                    match sequence {
                        EscapeSequence::UpArrow => {
                            navigate_history_up(&mut shell_state)?;
                        }
                        EscapeSequence::DownArrow => {
                            navigate_history_down(&mut shell_state)?;
                        }
                        _ => {
                            // Ignore other escape sequences for now
                        }
                    }
                }
            }
            b' '..=b'~' => { // Printable ASCII
                if shell_state.command_buffer.len() < MAX_COMMAND_LEN - 1 {
                    if shell_state.command_buffer.push(ch).is_ok() {
                        console_print!(
                            "{}",
                            core::str::from_utf8(&[ch]).unwrap_or("?")
                        );
                        
                                                 // Update current_input if not navigating history
                         if shell_state.history_index.is_none() {
                             // Clone the buffer to avoid borrow checker issues
                             let buffer_copy = shell_state.command_buffer.clone();
                             if let Ok(current_str) = core::str::from_utf8(&buffer_copy) {
                                 shell_state.current_input.clear();
                                 let _ = shell_state.current_input.push_str(current_str);
                             }
                         } else {
                             // User started typing while in history mode - exit history mode
                             shell_state.history_index = None;
                         }
                    }
                }
            }
            _ => {
                // Ignore other characters
            }
        }
    }
}

/// Read escape sequence for arrow keys
fn read_escape_sequence() -> Result<Option<EscapeSequence>, &'static str> {
    let ch1 = read_char();
    if ch1 != b'[' {
        return Ok(None); // Not a standard escape sequence
    }
    
    let ch2 = read_char();
    match ch2 {
        b'A' => Ok(Some(EscapeSequence::UpArrow)),
        b'B' => Ok(Some(EscapeSequence::DownArrow)),
        b'C' => Ok(Some(EscapeSequence::RightArrow)),
        b'D' => Ok(Some(EscapeSequence::LeftArrow)),
        _ => Ok(None), // Unknown escape sequence
    }
}

/// Navigate up in command history
fn navigate_history_up(shell_state: &mut ShellState) -> Result<(), &'static str> {
    if shell_state.history.is_empty() {
        return Ok(());
    }
    
    // Save current input if not already navigating history
    if shell_state.history_index.is_none() {
        if let Ok(current_str) = core::str::from_utf8(&shell_state.command_buffer) {
            shell_state.current_input.clear();
            let _ = shell_state.current_input.push_str(current_str);
        }
    }
    
    // Navigate to previous command
    let new_index = match shell_state.history_index {
        None => shell_state.history.len() - 1, // Start from most recent
        Some(idx) => {
            if idx > 0 {
                idx - 1
            } else {
                return Ok(()); // Already at oldest command
            }
        }
    };
    
    shell_state.history_index = Some(new_index);
    load_history_command(shell_state, new_index)?;
    Ok(())
}

/// Navigate down in command history
fn navigate_history_down(shell_state: &mut ShellState) -> Result<(), &'static str> {
    if shell_state.history.is_empty() {
        return Ok(());
    }
    
    match shell_state.history_index {
        None => Ok(()), // Not in history mode
        Some(idx) => {
            if idx + 1 < shell_state.history.len() {
                // Move to next command in history
                let new_index = idx + 1;
                shell_state.history_index = Some(new_index);
                load_history_command(shell_state, new_index)?;
            } else {
                // Return to current input
                shell_state.history_index = None;
                restore_current_input(shell_state)?;
            }
            Ok(())
        }
    }
}

/// Load a command from history into the command buffer
fn load_history_command(shell_state: &mut ShellState, index: usize) -> Result<(), &'static str> {
    if let Some(cmd) = shell_state.history.get(index) {
        // Clear current line
        clear_current_line(&shell_state.command_buffer)?;
        
        // Load command into buffer
        shell_state.command_buffer.clear();
        for byte in cmd.as_bytes() {
            if shell_state.command_buffer.push(*byte).is_err() {
                break;
            }
        }
        
        // Display the command
        console_print!("{}", cmd);
    }
    Ok(())
}

/// Restore current input when exiting history navigation
fn restore_current_input(shell_state: &mut ShellState) -> Result<(), &'static str> {
    // Clear current line
    clear_current_line(&shell_state.command_buffer)?;
    
    // Load current input into buffer
    shell_state.command_buffer.clear();
    for byte in shell_state.current_input.as_bytes() {
        if shell_state.command_buffer.push(*byte).is_err() {
            break;
        }
    }
    
    // Display the current input
    console_print!("{}", &shell_state.current_input);
    Ok(())
}

/// Clear the current line on the terminal
fn clear_current_line(command_buffer: &[u8]) -> Result<(), &'static str> {
    // Move cursor to beginning of line and clear to end
    console_print!("\r");
    console_print!("elinOS> ");
    
    // Also redraw prompt to framebuffer if graphics are available
    let _ = graphics::draw_shell_prompt();
    
    // Clear rest of line by printing spaces
    for _ in 0..command_buffer.len() {
        console_print!(" ");
    }
    
    // Move cursor back to start of input area
    console_print!("\r");
    console_print!("elinOS> ");
    
    // Redraw prompt to framebuffer again
    let _ = graphics::draw_shell_prompt();
    
    Ok(())
}

/// Add command to history
fn add_to_history(command: &str) {
    let mut shell_state = SHELL_STATE.lock();
    
    // Don't add duplicate consecutive commands
    if let Some(last_cmd) = shell_state.history.last() {
        if last_cmd == command {
            return;
        }
    }
    
    // Don't add history command itself to history
    if command.trim() == "history" {
        return;
    }
    
    // Create history entry
    if let Ok(cmd_string) = String::try_from(command) {
        // Remove oldest entry if at capacity
        if shell_state.history.len() >= MAX_HISTORY_ENTRIES {
            shell_state.history.remove(0);
        }
        
        // Add new command
        if shell_state.history.push(cmd_string).is_ok() {
            // Save history to file (ignore errors)
            drop(shell_state); // Release lock before saving
            save_shell_history();
        }
    }
}

/// Process enhanced command with built-in shell commands
fn process_enhanced_command(command: &str) -> Result<(), &'static str> {
    if command.is_empty() {
        return Ok(());
    }
    
    let parts: Vec<&str, 16> = command.split_whitespace().collect();
    if parts.is_empty() {
        return Ok(());
    }
    
    let cmd = parts[0];
    
    match cmd {
        "help" => cmd_shell_help(),
        "history" => cmd_shell_history(),
        "exit" | "quit" => Err("exit_shell"),
        "shutdown" => {
            console_println!("Shutting down system...");
            commands::cmd_shutdown()
        }
        "reboot" => {
            console_println!("Rebooting system...");
            commands::cmd_reboot()
        }
        _ => {
            // Delegate to existing command processor
            commands::process_command(command)
        }
    }
}

/// Built-in shell help command
fn cmd_shell_help() -> Result<(), &'static str> {
    console_println!("Built-in Shell Commands:");
    console_println!("  help     - Show this help message");
    console_println!("  history  - Show command history");
    console_println!("  exit     - Exit the shell");
    console_println!("  quit     - Exit the shell");
    console_println!("  shutdown - Shutdown the system");
    console_println!("  reboot   - Reboot the system");
    console_println!();
    console_println!("Navigation:");
    console_println!("  Up/Down  - Navigate command history");
    console_println!("  Backspace- Edit current command");
    console_println!();
    console_println!("System Commands:");
    // Delegate to existing help for system commands
    commands::cmd_help()
}

/// History command - show command history
fn cmd_shell_history() -> Result<(), &'static str> {
    let shell_state = SHELL_STATE.lock();
    
    console_println!("Command History:");
    console_println!("────────────────");
    
    if shell_state.history.is_empty() {
        console_println!("  (no commands in history)");
    } else {
        for (i, cmd) in shell_state.history.iter().enumerate() {
            console_print!("  ");
            if i + 1 < 10 {
                console_print!("  ");
            } else if i + 1 < 100 {
                console_print!(" ");
            }
            
            // Simple number display
            let num = i + 1;
            if num < 10 {
                console_print!("{}", num);
            } else if num < 100 {
                console_print!("{}", num);
            } else {
                console_print!("99+");
            }
            console_print!("  ");
            console_println!("{}", cmd);
        }
    }
    
    console_println!();
    console_print!("Total commands: ");
    console_println!("{}", shell_state.history.len());
    Ok(())
}

/// Read a character from UART
fn read_char() -> u8 {
    let uart = UART.lock();
    uart.getc()
}

// Stack top symbol
#[link_section = ".bss"]
static mut _STACK_TOP: [u8; 4096 * 4] = [0; 4096 * 4];