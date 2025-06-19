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
} 