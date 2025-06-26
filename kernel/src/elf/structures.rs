//! ELF Data Structures
//!
//! This module contains the data structures used for ELF64 parsing and loading,
//! including headers, program headers, and loaded ELF representations.

/// ELF64 Header
#[repr(C, packed)]
#[derive(Debug, Copy, Clone)]
pub struct Elf64Header {
    pub e_ident: [u8; 16],
    pub e_type: u16,
    pub e_machine: u16,
    pub e_version: u32,
    pub e_entry: u64,
    pub e_phoff: u64,
    pub e_shoff: u64,
    pub e_flags: u32,
    pub e_ehsize: u16,
    pub e_phentsize: u16,
    pub e_phnum: u16,
    pub e_shentsize: u16,
    pub e_shnum: u16,
    pub e_shstrndx: u16,
}

/// ELF64 Program Header
#[repr(C, packed)]
#[derive(Debug, Copy, Clone)]
pub struct Elf64ProgramHeader {
    pub p_type: u32,
    pub p_flags: u32,
    pub p_offset: u64,
    pub p_vaddr: u64,
    pub p_paddr: u64,
    pub p_filesz: u64,
    pub p_memsz: u64,
    pub p_align: u64,
}

/// Represents a loaded ELF segment in memory
#[derive(Debug)]
pub struct ElfSegment {
    pub vaddr: u64,
    pub memsz: u64,
    pub flags: u32,
    pub data_addr: Option<usize>, // Physical address where data is loaded
    pub data_size: usize,
}

/// Represents a fully loaded ELF binary
#[derive(Debug)]
pub struct LoadedElf {
    pub entry_point: u64,
    pub segments: heapless::Vec<ElfSegment, 8>,
} 