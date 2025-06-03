// ELF Loader for elinOS
// Supports loading and parsing ELF64 binaries for RISC-V

use crate::{UART, console_println};
use core::fmt::Write;

// ELF Magic number
const ELF_MAGIC: [u8; 4] = [0x7f, b'E', b'L', b'F'];

// ELF Classes
const ELFCLASS64: u8 = 2;

// ELF Data encoding
const ELFDATA2LSB: u8 = 1; // Little-endian

// ELF Machine types
const EM_RISCV: u16 = 243; // RISC-V

// ELF Types
const ET_EXEC: u16 = 2; // Executable file
const ET_DYN: u16 = 3;  // Shared object file

// Program header types
const PT_LOAD: u32 = 1; // Loadable segment

// Program header flags
const PF_X: u32 = 1; // Execute
const PF_W: u32 = 2; // Write
const PF_R: u32 = 4; // Read

// ELF64 Header
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

// ELF64 Program Header
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

// ELF Loading Result
#[derive(Debug)]
pub enum ElfError {
    InvalidMagic,
    UnsupportedClass,
    UnsupportedEndian,
    UnsupportedMachine,
    UnsupportedType,
    InvalidHeader,
    LoadError,
}

#[derive(Debug)]
pub struct LoadedElf {
    pub entry_point: u64,
    pub segments: heapless::Vec<ElfSegment, 8>,
}

#[derive(Debug)]
pub struct ElfSegment {
    pub vaddr: u64,
    pub memsz: u64,
    pub flags: u32,
}

pub struct ElfLoader;

impl ElfLoader {
    pub fn new() -> Self {
        ElfLoader
    }

    /// Parse and validate ELF header
    pub fn parse_header(&self, data: &[u8]) -> Result<&Elf64Header, ElfError> {
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

    /// Load ELF binary into memory
    pub fn load_elf(&self, data: &[u8]) -> Result<LoadedElf, ElfError> {
        let header = self.parse_header(data)?;
        
        // Copy packed fields to local variables to avoid alignment issues
        let entry_point = header.e_entry;
        let phnum = header.e_phnum;
        let phoff = header.e_phoff;
        let phentsize = header.e_phentsize;
        
        let mut uart = UART.lock();
        let _ = writeln!(uart, "Loading ELF binary:");
        let _ = writeln!(uart, "  Entry point: 0x{:x}", entry_point);
        let _ = writeln!(uart, "  Program headers: {}", phnum);
        drop(uart);

        // Parse program headers
        let ph_offset = phoff as usize;
        let ph_size = phentsize as usize;
        let ph_count = phnum as usize;

        if ph_offset + (ph_size * ph_count) > data.len() {
            return Err(ElfError::InvalidHeader);
        }

        let mut segments = heapless::Vec::new();

        for i in 0..ph_count {
            let ph_data = &data[ph_offset + (i * ph_size)..ph_offset + ((i + 1) * ph_size)];
            let ph = unsafe {
                &*(ph_data.as_ptr() as *const Elf64ProgramHeader)
            };

            if ph.p_type == PT_LOAD {
                // Copy packed fields to local variables
                let p_vaddr = ph.p_vaddr;
                let p_memsz = ph.p_memsz;
                let p_flags = ph.p_flags;
                let p_filesz = ph.p_filesz;
                let p_offset = ph.p_offset;
                
                let mut uart = UART.lock();
                let _ = writeln!(uart, "  Segment {}: 0x{:x} - 0x{:x} ({} bytes) flags: 0x{:x}",
                    i, p_vaddr, p_vaddr + p_memsz, p_memsz, p_flags);
                drop(uart);

                // In a real OS, we would map this segment into the process's virtual memory
                // For now, we'll just record the segment information
                let segment = ElfSegment {
                    vaddr: p_vaddr,
                    memsz: p_memsz,
                    flags: p_flags,
                };

                if segments.push(segment).is_err() {
                    return Err(ElfError::LoadError);
                }

                // TODO: Actually copy segment data to the target address
                // This would require proper memory management and virtual memory
                if p_filesz > 0 {
                    let file_offset = p_offset as usize;
                    let file_size = p_filesz as usize;
                    
                    if file_offset + file_size > data.len() {
                        return Err(ElfError::LoadError);
                    }

                    // In a real implementation, we would copy the data:
                    // let segment_data = &data[file_offset..file_offset + file_size];
                    // copy_to_virtual_address(ph.p_vaddr, segment_data);
                }
            }
        }

        let loaded_elf = LoadedElf {
            entry_point,
            segments,
        };

        let mut uart = UART.lock();
        let _ = writeln!(uart, "ELF loaded successfully, entry at 0x{:x}", loaded_elf.entry_point);
        drop(uart);

        Ok(loaded_elf)
    }

    /// Simple ELF information display
    pub fn display_elf_info(&self, data: &[u8]) -> Result<(), ElfError> {
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

    fn display_program_headers(&self, data: &[u8], header: &Elf64Header) -> Result<(), ElfError> {
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

    /// Check if data contains a valid ELF binary
    pub fn is_elf(&self, data: &[u8]) -> bool {
        self.parse_header(data).is_ok()
    }
}

// Helper function to get segment permissions as string
pub fn segment_permissions(flags: u32) -> &'static str {
    match flags & (PF_R | PF_W | PF_X) {
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