//! ELF Loader
//!
//! This module provides ELF64 binary loading functionality, including memory allocation
//! and segment loading into physical memory.

use crate::memory;
use super::constants::*;
use super::error::{ElfError, ElfResult};
use super::structures::{Elf64Header, LoadedElf, ElfSegment};
use super::parser::ElfParser;

/// ELF Loader for loading ELF64 binaries into memory
pub struct ElfLoader {
    parser: ElfParser,
}

impl ElfLoader {
    pub fn new() -> Self {
        ElfLoader {
            parser: ElfParser::new(),
        }
    }

    /// Load ELF binary into memory
    pub fn load_elf(&self, data: &[u8]) -> ElfResult<LoadedElf> {
        let header = self.parser.parse_header(data)?;
        
        // Copy packed fields to local variables to avoid alignment issues
        let entry_point = header.e_entry;
        let phnum = header.e_phnum;
        let ph_offset = header.e_phoff;
        let phentsize = header.e_phentsize;
        
        if memory::mmu::is_mmu_enabled() {
            // Using Software MMU - skipping hardware page table setup
        }
        
        let mut segments = heapless::Vec::<ElfSegment, 8>::new();
        
        // Calculate the base address for program headers
        let ph_start = ph_offset as usize;
        let ph_size = (phnum as usize) * (phentsize as usize);
        
        if ph_start + ph_size > data.len() {
            return Err(ElfError::InvalidHeader);
        }
        
        let ph_count = phnum as usize;
        
        for i in 0..ph_count {
            
            let ph_offset_in_data = ph_start + i * (phentsize as usize);
            
            if ph_offset_in_data + 56 > data.len() {
                continue;
            }
            
            let ph_data = &data[ph_offset_in_data..ph_offset_in_data + 56];
            let p_type = u32::from_le_bytes([ph_data[0], ph_data[1], ph_data[2], ph_data[3]]);
            
            
            if p_type == PT_LOAD {
                
                let p_flags = u32::from_le_bytes([ph_data[4], ph_data[5], ph_data[6], ph_data[7]]);
                let p_offset = u64::from_le_bytes([
                    ph_data[8], ph_data[9], ph_data[10], ph_data[11],
                    ph_data[12], ph_data[13], ph_data[14], ph_data[15]
                ]) as usize;
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
                
                
                if p_memsz == 0 {
                    continue;
                }
                
                
                let file_size = if p_offset < data.len() {
                    core::cmp::min(p_filesz as usize, data.len() - p_offset)
                } else {
                    0
                };
                
                
                let segment_data = if p_offset >= data.len() {
                    &[]
                } else if p_offset + file_size > data.len() {
                    &data[p_offset..data.len()]
                } else if file_size == 0 {
                    &[]
                } else {
                    &data[p_offset..p_offset + file_size]
                };
                
                let allocated_addr = if let Ok(addr) = memory::allocate_memory(p_memsz as usize, 8) {
                    
                    let dest_ptr = addr.as_ptr();
                    
                    unsafe {
                        // Zero the entire allocated memory
                        core::ptr::write_bytes(dest_ptr, 0, p_memsz as usize);
                        
                        // Copy file data if we have any
                        if !segment_data.is_empty() {
                            core::ptr::copy_nonoverlapping(
                                segment_data.as_ptr(),
                                dest_ptr,
                                segment_data.len()
                            );
                        }
                    }
                    
                    
                    addr.as_ptr() as usize
                } else {
                    return Err(ElfError::LoadError);
                };
                
                if let Err(_) = segments.push(ElfSegment {
                    vaddr: p_vaddr,
                    memsz: p_memsz,
                    data_addr: Some(allocated_addr),
                    data_size: file_size,
                    flags: p_flags,
                }) {
                    return Err(ElfError::LoadError);
                }
            }
        }
        
        
        Ok(LoadedElf {
            entry_point,
            segments,
        })
    }

    /// Parse and validate ELF header (delegate to parser)
    pub fn parse_header(&self, data: &[u8]) -> ElfResult<&Elf64Header> {
        self.parser.parse_header(data)
    }

    /// Check if data contains a valid ELF binary (delegate to parser)
    pub fn is_elf(&self, data: &[u8]) -> bool {
        self.parser.is_elf(data)
    }

    /// Display ELF information (delegate to parser)
    pub fn display_elf_info(&self, data: &[u8]) -> ElfResult<()> {
        self.parser.display_elf_info(data)
    }
} 