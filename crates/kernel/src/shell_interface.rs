//! Kernel implementation of the shell interface
//! 
//! This module implements the ShellInterface trait to bridge
//! the independent shell with kernel functionality safely.

use shell::ShellInterface;
use crate::{console_print, console_println};

/// Kernel's implementation of the shell interface
pub struct KernelShellInterface;

impl KernelShellInterface {
    pub fn new() -> Self {
        Self
    }
}

impl ShellInterface for KernelShellInterface {
    fn print(&self, s: &str) -> Result<(), &'static str> {
        console_print!("{}", s);
        Ok(())
    }
    
    fn println(&self, s: &str) -> Result<(), &'static str> {
        console_println!("{}", s);
        Ok(())
    }
    
    fn read_char(&self) -> u8 {
        let uart = crate::UART.lock();
        uart.getc()
    }
    
    fn execute_system_command(&self, cmd: &str, args: &[&str]) -> Result<(), &'static str> {
        // Build command string like before
        let mut full_command = heapless::String::<1024>::new();
        full_command.push_str(cmd).map_err(|_| "Command too long")?;
        
        for arg in args {
            full_command.push(' ').map_err(|_| "Command too long")?;
            full_command.push_str(arg).map_err(|_| "Command too long")?;
        }
        
        // SAFE: Delegate to existing command processor - preserves all functionality
        crate::commands::process_command(&full_command)
    }
    
    fn request_shutdown(&self) -> Result<(), &'static str> {
        // SAFE: Use existing shutdown command - preserves exit mechanism
        crate::commands::cmd_shutdown()
    }
    
    fn request_reboot(&self) -> Result<(), &'static str> {
        // SAFE: Use existing reboot command - preserves exit mechanism  
        crate::commands::cmd_reboot()
    }
    
    fn read_file(&self, path: &str) -> Result<heapless::Vec<u8, 4096>, &'static str> {
        // Use kernel's filesystem to read file
        match crate::filesystem::read_file(path) {
            Ok(data) => {
                // Convert to heapless Vec with size limit
                let mut result = heapless::Vec::new();
                let max_len = core::cmp::min(data.len(), 4096);
                
                for &byte in data.iter().take(max_len) {
                    if result.push(byte).is_err() {
                        break;  // Buffer full
                    }
                }
                
                Ok(result)
            }
            Err(_) => Err("Failed to read file"),
        }
    }
    
    fn write_file(&self, path: &str, data: &[u8]) -> Result<(), &'static str> {
        // Convert to string for filesystem write
        let content = core::str::from_utf8(data)
            .map_err(|_| "Invalid UTF-8 in file data")?;
        
        // Use kernel's filesystem to write file
        match crate::filesystem::write_file(path, content) {
            Ok(()) => Ok(()),
            Err(_) => Err("Failed to write file"),
        }
    }
} 