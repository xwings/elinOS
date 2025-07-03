//! ELF Parser
//!
//! This module provides ELF64 binary parsing and validation functionality.

use elinos_common::console_println;
use super::constants::*;
use super::error::{ElfError, ElfResult};
use super::structures::Elf64Header;

/// ELF Parser for ELF64 binaries
pub struct ElfParser;

impl ElfParser {
    pub fn new() -> Self {
        ElfParser
    }

    /// Parse and validate ELF header
    pub fn parse_header(&self, data: &[u8]) -> ElfResult<&Elf64Header> {
        if data.len() < core::mem::size_of::<Elf64Header>() {
            return Err(ElfError::InvalidHeader);
        }

        let header = unsafe {
            &*(data.as_ptr() as *const Elf64Header)
        };

        // Check ELF magic
        if header.e_ident[0..4] != ELF_MAGIC {
            return Err(ElfError::InvalidMagic);
        }

        // Check class (64-bit)
        if header.e_ident[4] != ELFCLASS64 {
            return Err(ElfError::UnsupportedClass);
        }

        // Check endianness (little-endian)
        if header.e_ident[5] != ELFDATA2LSB {
            return Err(ElfError::UnsupportedEndian);
        }

        // Check machine (RISC-V)
        if header.e_machine != EM_RISCV {
            return Err(ElfError::UnsupportedMachine);
        }

        // Check type (executable or shared object)
        if header.e_type != ET_EXEC && header.e_type != ET_DYN {
            return Err(ElfError::UnsupportedType);
        }

        Ok(header)
    }

    /// Check if data contains a valid ELF binary
    pub fn is_elf(&self, data: &[u8]) -> bool {
        self.parse_header(data).is_ok()
    }

    /// Display ELF information for debugging
    pub fn display_elf_info(&self, data: &[u8]) -> ElfResult<()> {
        console_println!("=== ELF Binary Analysis ===");
        
        let header = self.parse_header(data)?;
        
        console_println!("ELF Header:");
        console_println!("  Magic: {:02x} {:02x} {:02x} {:02x}", 
            data[0], data[1], data[2], data[3]);
        console_println!("  Class: ELF{}", if header.e_ident[4] == ELFCLASS64 { 64 } else { 32 });
        console_println!("  Endianness: {}", if header.e_ident[5] == ELFDATA2LSB { "Little" } else { "Big" });
        console_println!("  Version: {}", header.e_ident[6..10].iter().fold(0, |acc, &b| acc << 8 | b as u32));
        
        // Copy packed fields to local variables to avoid unaligned access
        let machine = header.e_machine;
        let elf_type = header.e_type;
        let entry = header.e_entry;
        let phnum = header.e_phnum;
        let phoff = header.e_phoff;
        let shnum = header.e_shnum;
        let shoff = header.e_shoff;
        
        console_println!("  Machine: 0x{:x} ({})", machine, 
            if machine == EM_RISCV { "RISC-V" } else { "Unknown" });
        console_println!("  Type: 0x{:x} ({})", elf_type,
            match elf_type {
                1 => "Relocatable",
                2 => "Executable", 
                3 => "Shared Object",
                4 => "Core",
                _ => "Unknown"
            });
        console_println!("  Entry Point: 0x{:x}", entry);
        console_println!("  Program Headers: {} (offset: 0x{:x})", 
            phnum, phoff);
        console_println!("  Section Headers: {} (offset: 0x{:x})", 
            shnum, shoff);
        
        // Parse and display program headers
        self.display_program_headers(data, header)?;
        
        Ok(())
    }

    /// Display program headers for debugging
    fn display_program_headers(&self, data: &[u8], header: &Elf64Header) -> ElfResult<()> {
        console_println!("\nProgram Headers:");
        
        for i in 0..header.e_phnum {
            // Fix type conversion issues
            let i_usize = i as usize;
            let ph_entsize = header.e_phentsize as u64;
            let ph_offset = header.e_phoff + (i_usize as u64 * ph_entsize);
            
            if ph_offset + 56 > data.len() as u64 { // 56 bytes per program header
                return Err(ElfError::InvalidHeader);
            }
            
            let ph_offset_usize = ph_offset as usize;
            let ph_data = &data[ph_offset_usize..ph_offset_usize + 56];
            let p_type = u32::from_le_bytes([ph_data[0], ph_data[1], ph_data[2], ph_data[3]]);
            let p_flags = u32::from_le_bytes([ph_data[4], ph_data[5], ph_data[6], ph_data[7]]);
            let p_vaddr = u64::from_le_bytes([
                ph_data[16], ph_data[17], ph_data[18], ph_data[19],
                ph_data[20], ph_data[21], ph_data[22], ph_data[23]
            ]);
            let p_filesz = u64::from_le_bytes([
                ph_data[32], ph_data[33], ph_data[34], ph_data[35],
                ph_data[36], ph_data[37], ph_data[38], ph_data[39]
            ]);
            let p_memsz = u64::from_le_bytes([
                ph_data[40], ph_data[41], ph_data[42], ph_data[43],
                ph_data[44], ph_data[45], ph_data[46], ph_data[47]
            ]);
            
            let type_name = match p_type {
                0 => "NULL",
                1 => "LOAD",
                2 => "DYNAMIC", 
                3 => "INTERP",
                4 => "NOTE",
                5 => "SHLIB",
                6 => "PHDR",
                7 => "TLS",
                _ => "Unknown"
            };
            
            let perms = segment_permissions(p_flags);
            
            console_println!("  [{}] {} vaddr=0x{:x} filesz={} memsz={} [{}]",
                i, type_name, p_vaddr, p_filesz, p_memsz, perms);
        }
        
        Ok(())
    }
} 