#![no_std]

//! # elinOS Shell
//! 
//! Independent shell implementation for elinOS.
//! Communicates with kernel through safe function call interface.

use heapless::{String, Vec};

pub mod commands;
pub mod parser;

/// Maximum length for command buffer
pub const MAX_COMMAND_LEN: usize = 1024;

/// Maximum path length
pub const MAX_PATH_LEN: usize = 256;

/// Maximum number of history entries
pub const MAX_HISTORY_ENTRIES: usize = 100;

/// History file path
pub const HISTORY_FILE_PATH: &str = "/.shell_history";

/// Shell configuration
pub struct ShellConfig {
    pub prompt: &'static str,
    pub max_command_len: usize,
}

impl Default for ShellConfig {
    fn default() -> Self {
        Self {
            prompt: "elinOS> ",
            max_command_len: MAX_COMMAND_LEN,
        }
    }
}

/// Shell interface trait - implemented by kernel
pub trait ShellInterface {
    /// Print a string to console
    fn print(&self, s: &str) -> Result<(), &'static str>;
    
    /// Print a string to console with newline
    fn println(&self, s: &str) -> Result<(), &'static str>;
    
    /// Read a character from input
    fn read_char(&self) -> u8;
    
    /// Execute a system command
    fn execute_system_command(&self, cmd: &str, args: &[&str]) -> Result<(), &'static str>;
    
    /// Request system shutdown
    fn request_shutdown(&self) -> Result<(), &'static str>;
    
    /// Request system reboot  
    fn request_reboot(&self) -> Result<(), &'static str>;
    
    /// Read a file from filesystem
    fn read_file(&self, path: &str) -> Result<Vec<u8, 4096>, &'static str>;
    
    /// Write a file to filesystem
    fn write_file(&self, path: &str, data: &[u8]) -> Result<(), &'static str>;
}

/// Main shell structure
pub struct Shell<I: ShellInterface> {
    interface: I,
    config: ShellConfig,
    command_buffer: Vec<u8, MAX_COMMAND_LEN>,
    history: Vec<String<MAX_COMMAND_LEN>, MAX_HISTORY_ENTRIES>,
}

impl<I: ShellInterface> Shell<I> {
    /// Create a new shell instance
    pub fn new(interface: I) -> Self {
        let mut shell = Self {
            interface,
            config: ShellConfig::default(),
            command_buffer: Vec::new(),
            history: Vec::new(),
        };
        
        // Load history from filesystem
        if let Err(_e) = shell.load_history() {
            // History file might not exist on first run - that's normal
        }
        
        shell
    }
    
    /// Create shell with custom config
    pub fn with_config(interface: I, config: ShellConfig) -> Self {
        let mut shell = Self {
            interface,
            config,
            command_buffer: Vec::new(),
            history: Vec::new(),
        };
        
        // Load history from filesystem
        if let Err(_e) = shell.load_history() {
            // History file might not exist on first run - that's normal
        }
        
        shell
    }
    
    /// Show welcome message
    pub fn show_welcome(&self) -> Result<(), &'static str> {
        self.interface.println("=====================================")?;
        self.interface.println("          Welcome to elinOS!         ")?;
        self.interface.println("=====================================")?;
        self.interface.println("  RISC-V64 Operating System written in Rust")?;
        self.interface.println("")?;
        self.interface.println("  Type 'help' for available commands")?;
        self.interface.println("  Type 'version' for system information")?;
        self.interface.println("  Type 'shutdown' to exit")?;
        self.interface.println("")?;
        Ok(())
    }
    
    /// Main shell loop - SAFE: doesn't handle traps, just UI
    pub fn run_loop(&mut self) -> Result<(), &'static str> {
        loop {
            // Show prompt
            self.interface.print(self.config.prompt)?;
            
            // Read command
            self.read_command()?;
            
            // Process command
            if !self.command_buffer.is_empty() {
                // Convert command buffer to string and clone for processing
                let command_str = core::str::from_utf8(&self.command_buffer)
                    .map_err(|_| "Invalid UTF-8 in command")?;
                let command_copy = String::<MAX_COMMAND_LEN>::try_from(command_str.trim())
                    .map_err(|_| "Command too long")?;
                
                // Process the command (this is safe - no trap handling)
                let result = self.process_command(&command_copy);
                
                // Add to history if not empty and command processed successfully
                if !command_copy.is_empty() {
                    self.add_to_history(&command_copy);
                }
                
                if let Err(e) = result {
                    // Don't show error for clean exit
                    if e != "exit_shell" {
                        self.interface.print("Error: ")?;
                        self.interface.println(e)?;
                    } else {
                        return Err(e);  // Propagate exit request
                    }
                }
            }
            
            self.interface.println("")?;
        }
    }
    
    /// Read a command from input (safe character handling)
    fn read_command(&mut self) -> Result<(), &'static str> {
        self.command_buffer.clear();
        
        loop {
            let ch = self.interface.read_char();
            
            match ch {
                b'\r' | b'\n' => {
                    self.interface.println("")?;
                    break;
                }
                b'\x08' | b'\x7f' => {  // Backspace or DEL
                    if !self.command_buffer.is_empty() {
                        self.command_buffer.pop();
                        self.interface.print("\x08 \x08")?;  // Move back, print space, move back
                    }
                }
                b' '..=b'~' => {  // Printable ASCII
                    if self.command_buffer.len() < self.config.max_command_len - 1 {
                        if self.command_buffer.push(ch).is_ok() {
                            self.interface.print(core::str::from_utf8(&[ch]).unwrap_or("?"))?;
                        }
                    }
                }
                _ => {
                    // Ignore other characters
                }
            }
        }
        
        Ok(())
    }
    
    /// Process a command (safe - delegates to kernel for system operations)
    fn process_command(&self, command: &str) -> Result<(), &'static str> {
        if command.is_empty() {
            return Ok(());
        }
        
        let parts: Vec<&str, 16> = command.split_whitespace().collect();
        if parts.is_empty() {
            return Ok(());
        }
        
        let cmd = parts[0];
        let args = &parts[1..];
        
        match cmd {
            "help" => self.cmd_help(),
            "history" => self.cmd_history(),
            "exit" | "quit" => self.cmd_exit(),
            "shutdown" => self.cmd_shutdown(),
            "reboot" => self.cmd_reboot(),
            _ => {
                // Delegate system commands to kernel interface
                self.interface.execute_system_command(cmd, args)
            }
        }
    }
    
    /// Built-in help command
    fn cmd_help(&self) -> Result<(), &'static str> {
        self.interface.println("Built-in Shell Commands:")?;
        self.interface.println("  help     - Show this help message")?;
        self.interface.println("  history  - Show command history")?;
        self.interface.println("  exit     - Exit the shell")?;
        self.interface.println("  quit     - Exit the shell")?;
        self.interface.println("  shutdown - Shutdown the system")?;
        self.interface.println("  reboot   - Reboot the system")?;
        self.interface.println("")?;
        self.interface.println("System Commands:")?;
        self.interface.println("  Use any system command - they will be passed to the kernel")?;
        Ok(())
    }
    
    /// History command - show command history
    fn cmd_history(&self) -> Result<(), &'static str> {
        self.interface.println("Command History:")?;
        self.interface.println("────────────────")?;
        
        if self.history.is_empty() {
            self.interface.println("  (no commands in history)")?;
        } else {
            for (i, cmd) in self.history.iter().enumerate() {
                // Show recent commands with line numbers (simple formatting)
                self.interface.print("  ")?;
                if i < 9 {
                    self.interface.print("  ")?;
                } else if i < 99 {
                    self.interface.print(" ")?;
                }
                // Simple number display
                let num = i + 1;
                if num < 10 {
                    let digit_str = [(b'0' + num as u8)];
                    self.interface.print(core::str::from_utf8(&digit_str).unwrap_or("?"))?;
                } else if num < 100 {
                    let digits = [(b'0' + (num / 10) as u8), (b'0' + (num % 10) as u8)];
                    self.interface.print(core::str::from_utf8(&digits).unwrap_or("??"))?;
                } else {
                    self.interface.print("99+")?;
                }
                self.interface.print("  ")?;
                self.interface.println(cmd)?;
            }
        }
        
        self.interface.println("")?;
        self.interface.print("Total commands: ")?;
        // Simple number display for count
        let count = self.history.len();
        if count == 0 {
            self.interface.println("0")?;
        } else if count < 10 {
            let digit_str = [(b'0' + count as u8)];
            self.interface.println(core::str::from_utf8(&digit_str).unwrap_or("?"))?;
        } else if count < 100 {
            let digits = [(b'0' + (count / 10) as u8), (b'0' + (count % 10) as u8)];
            self.interface.println(core::str::from_utf8(&digits).unwrap_or("??"))?;
        } else {
            self.interface.println("99+")?;
        }
        Ok(())
    }
    
    /// Exit command - SAFE: just returns error to break loop
    fn cmd_exit(&self) -> Result<(), &'static str> {
        self.interface.println("Exiting shell...")?;
        Err("exit_shell")  // Special error to indicate clean exit
    }
    
    /// Shutdown command - SAFE: delegates to kernel
    fn cmd_shutdown(&self) -> Result<(), &'static str> {
        self.interface.println("Shutting down system...")?;
        self.interface.request_shutdown()
    }
    
    /// Reboot command - SAFE: delegates to kernel  
    fn cmd_reboot(&self) -> Result<(), &'static str> {
        self.interface.println("Rebooting system...")?;
        self.interface.request_reboot()
    }
    
    /// Add command to history
    fn add_to_history(&mut self, command: &str) {
        // Don't add duplicate consecutive commands
        if let Some(last_cmd) = self.history.last() {
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
            if self.history.len() >= MAX_HISTORY_ENTRIES {
                self.history.remove(0);
            }
            
            // Add new command
            if self.history.push(cmd_string).is_ok() {
                // Save history to file (ignore errors for now)
                let _ = self.save_history();
            }
        }
    }
    
    /// Load history from filesystem
    fn load_history(&mut self) -> Result<(), &'static str> {
        match self.interface.read_file(HISTORY_FILE_PATH) {
            Ok(data) => {
                // Parse history file
                let content = core::str::from_utf8(&data)
                    .map_err(|_| "Invalid UTF-8 in history file")?;
                
                self.history.clear();
                for line in content.lines() {
                    let trimmed = line.trim();
                    if !trimmed.is_empty() {
                        if let Ok(cmd_string) = String::try_from(trimmed) {
                            if self.history.push(cmd_string).is_err() {
                                break;  // History buffer is full
                            }
                        }
                    }
                }
                
                Ok(())
            }
            Err(_) => {
                // History file doesn't exist yet - that's okay
                Ok(())
            }
        }
    }
    
    /// Save history to filesystem
    fn save_history(&self) -> Result<(), &'static str> {
        // Build history file content
        let mut content = String::<4096>::new();
        
        for cmd in &self.history {
            content.push_str(cmd).map_err(|_| "History content too large")?;
            content.push('\n').map_err(|_| "History content too large")?;
        }
        
        // Write to filesystem
        self.interface.write_file(HISTORY_FILE_PATH, content.as_bytes())
    }
} 