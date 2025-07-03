//! ELF Common Library
//!
//! Shared ELF parsing and loading functionality between bootloader and kernel

use core::mem;

/// ELF64 header structure
#[repr(C)]
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

/// ELF64 program header structure
#[repr(C)]
pub struct Elf64Phdr {
    pub p_type: u32,
    pub p_flags: u32,
    pub p_offset: u64,
    pub p_vaddr: u64,
    pub p_paddr: u64,
    pub p_filesz: u64,
    pub p_memsz: u64,
    pub p_align: u64,
}

/// ELF constants
pub const ELFMAG: [u8; 4] = [0x7f, b'E', b'L', b'F'];
pub const ELF_MAGIC: [u8; 4] = ELFMAG; // Alias for compatibility
pub const ELFCLASS64: u8 = 2;
pub const ELFDATA2LSB: u8 = 1; // Little-endian
pub const EM_RISCV: u16 = 243; // RISC-V
pub const ET_EXEC: u16 = 2; // Executable file
pub const ET_DYN: u16 = 3;  // Shared object file
pub const PT_LOAD: u32 = 1;

// Program header flags
pub const PF_X: u32 = 1; // Execute
pub const PF_W: u32 = 2; // Write
pub const PF_R: u32 = 4; // Read

/// ELF validation and parsing utilities
pub struct ElfUtils;

/// Get segment permissions as string representation (standalone function for compatibility)
pub fn segment_permissions(flags: u32) -> &'static str {
    ElfUtils::segment_permissions(flags)
}

impl ElfUtils {
    /// Validate ELF header
    pub fn validate_elf_header(data: &[u8]) -> bool {
        if data.len() < mem::size_of::<Elf64Header>() {
            return false;
        }

        // Check magic number
        if &data[0..4] != &ELF_MAGIC {
            return false;
        }

        // Check 64-bit class
        if data[4] != ELFCLASS64 {
            return false;
        }

        true
    }

    /// Get ELF header from data
    pub fn get_header(data: &[u8]) -> Option<&Elf64Header> {
        if !Self::validate_elf_header(data) {
            return None;
        }

        unsafe {
            Some(&*(data.as_ptr() as *const Elf64Header))
        }
    }

    /// Get program header at index
    pub fn get_program_header(data: &[u8], header: &Elf64Header, index: usize) -> Option<Elf64Phdr> {
        if index >= header.e_phnum as usize {
            return None;
        }

        let ph_offset = header.e_phoff as usize + index * header.e_phentsize as usize;
        if ph_offset + mem::size_of::<Elf64Phdr>() > data.len() {
            return None;
        }

        unsafe {
            Some(core::ptr::read_unaligned(data.as_ptr().add(ph_offset) as *const Elf64Phdr))
        }
    }

    /// Get segment data from ELF file
    pub fn get_segment_data<'a>(data: &'a [u8], phdr: &Elf64Phdr) -> Option<&'a [u8]> {
        let offset = phdr.p_offset as usize;
        let size = phdr.p_filesz as usize;

        if offset + size > data.len() {
            return None;
        }

        Some(&data[offset..offset + size])
    }

    /// Check if program header is loadable
    pub fn is_loadable_segment(phdr: &Elf64Phdr) -> bool {
        phdr.p_type == PT_LOAD && phdr.p_memsz > 0
    }

    /// Get segment permissions as string representation
    pub fn segment_permissions(flags: u32) -> &'static str {
        let masked_flags = flags & (PF_R | PF_W | PF_X);
        
        match masked_flags {
            0 => "---",
            1 => "--X", // PF_X
            2 => "-W-", // PF_W
            3 => "-WX", // PF_W | PF_X
            4 => "R--", // PF_R
            5 => "R-X", // PF_R | PF_X
            6 => "RW-", // PF_R | PF_W
            7 => "RWX", // PF_R | PF_W | PF_X
            _ => "???",
        }
    }
}

/// Trait for ELF loading implementations
pub trait ElfLoader {
    type Error;

    /// Load a segment into memory
    fn load_segment(&self, phdr: &Elf64Phdr, data: &[u8]) -> Result<(), Self::Error>;

    /// Get entry point after loading
    fn get_entry_point(&self, header: &Elf64Header) -> u64 {
        header.e_entry
    }
}