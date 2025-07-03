//! ELF Loader
//!
//! This module provides ELF64 binary loading functionality, including memory allocation
//! and segment loading into physical memory.

use crate::{console_println, memory};
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
        
        console_println!("[i] Loading ELF binary with MMU support:");
        console_println!("   Entry point: 0x{:x}", entry_point);
        console_println!("   Program headers: {}", phnum);
        console_println!("   File size: {} bytes", data.len());
        
        if memory::mmu::is_mmu_enabled() {
            console_println!("[i] Using Software MMU - skipping hardware page table setup");
        }
        
        let mut segments = heapless::Vec::<ElfSegment, 8>::new();
        
        // Calculate the base address for program headers
        let ph_start = ph_offset as usize;
        let ph_size = (phnum as usize) * (phentsize as usize);
        
        if ph_start + ph_size > data.len() {
            return Err(ElfError::InvalidHeader);
        }
        
        let ph_count = phnum as usize;
        console_println!("[i] Starting to process {} program headers...", ph_count);
        
        for i in 0..ph_count {
            console_println!("[i] Processing program header {}/{}", i + 1, ph_count);
            
            let ph_offset_in_data = ph_start + i * (phentsize as usize);
            
            if ph_offset_in_data + 56 > data.len() {
                continue;
            }
            
            let ph_data = &data[ph_offset_in_data..ph_offset_in_data + 56];
            let p_type = u32::from_le_bytes([ph_data[0], ph_data[1], ph_data[2], ph_data[3]]);
            
            console_println!("[i] Program header {}: type=0x{:x}", i, p_type);
            
            if p_type == PT_LOAD {
                console_println!("[i] Found PT_LOAD segment {}", i);
                
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
                
                console_println!("[i] Segment details: vaddr=0x{:x}, memsz={}, flags=0x{:x}",
                    p_vaddr, p_memsz, p_flags);
                
                console_println!("   Segment {}: 0x{:x} - 0x{:x} ({} bytes) flags: 0x{:x} ({})",
                    i, p_vaddr, p_vaddr + p_memsz, p_memsz, p_flags,
                    segment_permissions(p_flags));
                
                if p_memsz == 0 {
                    continue;
                }
                
                console_println!("[i] Allocating memory for segment: {} bytes", p_memsz);
                
                let file_size = if p_offset < data.len() {
                    core::cmp::min(p_filesz as usize, data.len() - p_offset)
                } else {
                    0
                };
                
                console_println!("[i] Reading {} bytes from file offset 0x{:x}", file_size, p_offset);
                
                let segment_data = if p_offset >= data.len() {
                    console_println!("[!] File offset 0x{:x} is beyond file size {} - treating as BSS",
                        p_offset, data.len());
                    &[]
                } else if p_offset + file_size > data.len() {
                    let available = data.len() - p_offset;
                    console_println!("[!] Partial read: only {} bytes available from offset 0x{:x}",
                        available, p_offset);
                    &data[p_offset..data.len()]
                } else if file_size == 0 {
                    console_println!("[i] No file data to read (BSS segment)");
                    &[]
                } else {
                    &data[p_offset..p_offset + file_size]
                };
                
                console_println!("[i] Calling allocate_memory({})...", p_memsz);
                let allocated_addr = if let Some(addr) = memory::allocate_memory(p_memsz as usize) {
                    console_println!("[o] Memory allocated at 0x{:x}", addr);
                    
                    let dest_ptr = addr as *mut u8;
                    
                    unsafe {
                        // Zero the entire allocated memory
                        core::ptr::write_bytes(dest_ptr, 0, p_memsz as usize);
                        
                        // Copy file data if we have any
                        if !segment_data.is_empty() {
                            console_println!("[i] Copying {} bytes to memory", segment_data.len());
                            core::ptr::copy_nonoverlapping(
                                segment_data.as_ptr(),
                                dest_ptr,
                                segment_data.len()
                            );
                        } else {
                            console_println!("[i] No file data to copy - memory zeroed");
                        }
                    }
                    
                    if memory::mmu::is_mmu_enabled() {
                        console_println!("[i] Software MMU: Virtual 0x{:x} -> Physical 0x{:x} (will translate at runtime)",
                            p_vaddr, addr);
                    }
                    
                    if !segment_data.is_empty() {
                        console_println!("[o] Copied {} bytes to 0x{:x}", file_size, addr);
                    }
                    
                    addr
                } else {
                    console_println!("[x] Failed to allocate memory for segment");
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
        
        console_println!("[o] ELF loaded successfully with {} segments, entry at 0x{:x}",
            segments.len(), entry_point);
        
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