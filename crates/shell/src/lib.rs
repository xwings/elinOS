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
}

/// Main shell structure
pub struct Shell<I: ShellInterface> {
    interface: I,
    config: ShellConfig,
    command_buffer: Vec<u8, MAX_COMMAND_LEN>,
}

impl<I: ShellInterface> Shell<I> {
    /// Create a new shell instance
    pub fn new(interface: I) -> Self {
        Self {
            interface,
            config: ShellConfig::default(),
            command_buffer: Vec::new(),
        }
    }
    
    /// Create shell with custom config
    pub fn with_config(interface: I, config: ShellConfig) -> Self {
        Self {
            interface,
            config,
            command_buffer: Vec::new(),
        }
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
                let command_str = core::str::from_utf8(&self.command_buffer)
                    .map_err(|_| "Invalid UTF-8 in command")?;
                
                // Process the command (this is safe - no trap handling)
                if let Err(e) = self.process_command(command_str.trim()) {
                    self.interface.print("Error: ")?;
                    self.interface.println(e)?;
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
        self.interface.println("  exit     - Exit the shell")?;
        self.interface.println("  quit     - Exit the shell")?;
        self.interface.println("  shutdown - Shutdown the system")?;
        self.interface.println("  reboot   - Reboot the system")?;
        self.interface.println("")?;
        self.interface.println("System Commands:")?;
        self.interface.println("  Use any system command - they will be passed to the kernel")?;
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
} 