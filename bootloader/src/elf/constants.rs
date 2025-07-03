//! ELF Constants and Type Definitions
//! 
//! This module contains all the constants, magic numbers, and type definitions
//! used in ELF64 parsing and loading for RISC-V.

// ELF Magic number
pub const ELF_MAGIC: [u8; 4] = [0x7f, b'E', b'L', b'F'];

// ELF Classes
pub const ELFCLASS64: u8 = 2;

// ELF Data encoding
pub const ELFDATA2LSB: u8 = 1; // Little-endian

// ELF Machine types
pub const EM_RISCV: u16 = 243; // RISC-V

// ELF Types
pub const ET_EXEC: u16 = 2; // Executable file
pub const ET_DYN: u16 = 3;  // Shared object file

// Program header types
pub const PT_LOAD: u32 = 1; // Loadable segment

// Program header flags
pub const PF_X: u32 = 1; // Execute
pub const PF_W: u32 = 2; // Write
pub const PF_R: u32 = 4; // Read

/// Get segment permissions as string representation
pub fn segment_permissions(flags: u32) -> &'static str {
    let masked_flags = flags & (PF_R | PF_W | PF_X);
    
    match masked_flags {
        0 => "---",
        PF_R => "R--",
        PF_W => "-W-",
        PF_X => "--X",
        PF_R | PF_W => "RW-",
        PF_R | PF_X => "R-X",
        PF_W | PF_X => "-WX",
        PF_R | PF_W | PF_X => "RWX",
        _ => "???",
    }
} 