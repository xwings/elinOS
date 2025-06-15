// ELF Loader for elinOS
// Supports loading and parsing ELF64 binaries for RISC-V

use crate::{UART, console_println};
use crate::memory::mmu::{self, PTE_R, PTE_W, PTE_X, PTE_U};
use crate::memory;
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
    pub data_addr: Option<usize>, // Physical address where data is loaded
    pub data_size: usize,
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
        let ph_offset = header.e_phoff;
        let phentsize = header.e_phentsize;
        
        console_println!("ℹ️ Loading ELF binary with MMU support:");
        console_println!("   Entry point: 0x{:x}", entry_point);
        console_println!("   Program headers: {}", phnum);
        console_println!("   File size: {} bytes", data.len());
        
        if memory::mmu::is_mmu_enabled() {
            console_println!("ℹ️ Using Software MMU - skipping hardware page table setup");
        }
        
        let mut segments = heapless::Vec::<ElfSegment, 8>::new();
        
        // Calculate the base address for program headers
        let ph_start = ph_offset as usize;
        let ph_size = (phnum as usize) * (phentsize as usize);
        
        if ph_start + ph_size > data.len() {
            return Err(ElfError::InvalidHeader);
        }
        
        let ph_count = phnum as usize;
        console_println!("ℹ️ Starting to process {} program headers...", ph_count);
        
        for i in 0..ph_count {
            console_println!("ℹ️ Processing program header {}/{}", i + 1, ph_count);
            
            let ph_offset_in_data = ph_start + i * (phentsize as usize);
            
            if ph_offset_in_data + 56 > data.len() {
                continue;
            }
            
            let ph_data = &data[ph_offset_in_data..ph_offset_in_data + 56];
            let p_type = u32::from_le_bytes([ph_data[0], ph_data[1], ph_data[2], ph_data[3]]);
            
            console_println!("ℹ️ Program header {}: type=0x{:x}", i, p_type);
            
            if p_type == PT_LOAD {
                console_println!("ℹ️ Found PT_LOAD segment {}", i);
                
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
                
                console_println!("ℹ️ Segment details: vaddr=0x{:x}, memsz={}, flags=0x{:x}",
                    p_vaddr, p_memsz, p_flags);
                
                console_println!("   Segment {}: 0x{:x} - 0x{:x} ({} bytes) flags: 0x{:x} ({})",
                    i, p_vaddr, p_vaddr + p_memsz, p_memsz, p_flags,
                    segment_permissions(p_flags));
                
                if p_memsz == 0 {
                    continue;
                }
                
                console_println!("ℹ️ Allocating memory for segment: {} bytes", p_memsz);
                
                let file_size = if p_offset < data.len() {
                    core::cmp::min(p_filesz as usize, data.len() - p_offset)
                } else {
                    0
                };
                
                console_println!("ℹ️ Reading {} bytes from file offset 0x{:x}", file_size, p_offset);
                
                let segment_data = if p_offset >= data.len() {
                    console_println!("⚠️ File offset 0x{:x} is beyond file size {} - treating as BSS",
                        p_offset, data.len());
                    &[]
                } else if p_offset + file_size > data.len() {
                    let available = data.len() - p_offset;
                    console_println!("⚠️ Partial read: only {} bytes available from offset 0x{:x}",
                        available, p_offset);
                    &data[p_offset..data.len()]
                } else if file_size == 0 {
                    console_println!("ℹ️ No file data to read (BSS segment)");
                    &[]
                } else {
                    &data[p_offset..p_offset + file_size]
                };
                
                console_println!("ℹ️ Calling allocate_memory({})...", p_memsz);
                let allocated_addr = if let Some(addr) = memory::allocate_memory(p_memsz as usize) {
                    console_println!("✅ Memory allocated at 0x{:x}", addr);
                    
                    let dest_ptr = addr as *mut u8;
                    
                    unsafe {
                        // Zero the entire allocated memory
                        core::ptr::write_bytes(dest_ptr, 0, p_memsz as usize);
                        
                        // Copy file data if we have any
                        if !segment_data.is_empty() {
                            console_println!("ℹ️ Copying {} bytes to memory", segment_data.len());
                            core::ptr::copy_nonoverlapping(
                                segment_data.as_ptr(),
                                dest_ptr,
                                segment_data.len()
                            );
                        } else {
                            console_println!("ℹ️ No file data to copy - memory zeroed");
                        }
                    }
                    
                    if memory::mmu::is_mmu_enabled() {
                        console_println!("ℹ️ Software MMU: Virtual 0x{:x} -> Physical 0x{:x} (will translate at runtime)",
                            p_vaddr, addr);
                    }
                    
                    if !segment_data.is_empty() {
                        console_println!("✅ Copied {} bytes to 0x{:x}", file_size, addr);
                    }
                    
                    addr
                } else {
                    console_println!("❌ Failed to allocate memory for segment");
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
        
        console_println!("✅ ELF loaded successfully with {} segments, entry at 0x{:x}",
            segments.len(), entry_point);
        
        Ok(LoadedElf {
            entry_point,
            segments,
        })
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

    /// Execute a loaded ELF binary with MMU support
    pub fn execute_elf(&self, loaded_elf: &LoadedElf) -> Result<(), ElfError> {
        console_println!("ℹ️ Executing ELF at entry point 0x{:x}", loaded_elf.entry_point);
        
        // Always use software MMU for now (hardware MMU has issues)
        console_println!("ℹ️  Virtual Memory enabled - executing with software MMU");
        
        // Execute with software virtual memory translation
        console_println!("🏃 Executing at virtual entry point: 0x{:x}", loaded_elf.entry_point);
        
        unsafe {
            execute_user_program_with_software_mmu(loaded_elf.entry_point as usize, loaded_elf);
        }
        
        Ok(())
    }
}

// Helper function to get segment permissions as string
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

/// Execute user program with temporary syscall support
unsafe fn execute_with_syscall_support(entry_point: usize) -> usize {
    use core::arch::asm;
    
    //console_println!("ℹ️  Setting up REAL user mode execution with REAL syscalls!");
    
    // Allocate user stack
    let user_stack = match crate::memory::allocate_memory(8192) {
        Some(addr) => addr,
        None => {
            console_println!("❌ Failed to allocate user stack");
            return 0;
        }
    };
    let user_stack_top = user_stack + 8192;
    
    console_println!("📚 User stack allocated: 0x{:x} - 0x{:x}", user_stack, user_stack_top);
    
    // Create a small exit stub that will be called when the user program returns
    let exit_stub = match crate::memory::allocate_memory(32) {
        Some(addr) => addr,
        None => {
            console_println!("❌ Failed to allocate exit stub");
            crate::memory::deallocate_memory(user_stack, 8192);
            return 0;
        }
    };
    
    // Write exit stub code: li a7, 93; ecall; ebreak (breakpoint)
    // This is actually: addi a7, x0, 93
    // RISC-V I-type: imm[11:0] | rs1 | funct3 | rd | opcode
    // addi: opcode=0010011, funct3=000, rs1=x0(0), rd=a7(17), imm=93
    // 93 = 0x5d = 0000 0101 1101
    // Encoding: 0000 0101 1101 | 00000 | 000 | 10001 | 0010011
    //          = 0x05d00893
    let exit_stub_ptr = exit_stub as *mut u32;
    exit_stub_ptr.write_volatile(0x05d00893); // li a7, 93 (addi a7, x0, 93)
    exit_stub_ptr.add(1).write_volatile(0x00000073); // ecall
    exit_stub_ptr.add(2).write_volatile(0x00100073); // ebreak (breakpoint)
    exit_stub_ptr.add(3).write_volatile(0x00000013); // nop (padding)
    
    console_println!("ℹ️ Exit stub created at 0x{:x}", exit_stub);
    console_println!("ℹ️ Exit stub instructions:");
    console_println!("   0x{:x}: 0x{:08x} (li a7, 93)", exit_stub, exit_stub_ptr.read_volatile());
    console_println!("   0x{:x}: 0x{:08x} (ecall)", exit_stub + 4, exit_stub_ptr.add(1).read_volatile());
    console_println!("   0x{:x}: 0x{:08x} (ebreak)", exit_stub + 8, exit_stub_ptr.add(2).read_volatile());
    console_println!("   0x{:x}: 0x{:08x} (nop)", exit_stub + 12, exit_stub_ptr.add(3).read_volatile());
    
    console_println!("ℹ️ User mode context set up:");
    console_println!("   Entry point: 0x{:x}", entry_point);
    console_println!("   Stack pointer: 0x{:x}", user_stack_top);
    console_println!("   Return address: 0x{:x}", exit_stub);
    
    // Set up proper user mode status
    // Clear SPP bit (bit 8) to ensure we return to user mode
    // Set SPIE bit (bit 5) to enable interrupts in user mode
    let user_status = 0x00000020; // SPIE=1, SPP=0 (user mode)
    console_println!("   Status: 0x{:x}", user_status);
    
    console_println!("ℹ️ About to jump to user mode...");
    
    let result: usize;
    unsafe {
        asm!(
            // Set up supervisor exception program counter to user entry point
            "csrw sepc, {entry}",
            
            // Set up supervisor status for user mode
            "csrw sstatus, {status}",
            
            // Set up user stack pointer
            "mv sp, {stack}",
            
            // Set up return address for when user program exits
            "mv ra, {exit_stub}",
            
            // Jump to user mode
            "sret",
            
            // We should never reach this point normally
            // But if we do (via breakpoint handling), get the result from a0
            "mv {result}, a0",
            
            entry = in(reg) entry_point,
            status = in(reg) user_status,
            stack = in(reg) user_stack_top,
            exit_stub = in(reg) exit_stub,
            result = out(reg) result,
        );
    }
    
    console_println!("✅ Returned from user mode!");
    console_println!("🏁 User program returned: {}", result);
    result
}

/// Temporary trap handler specifically for user program execution
#[no_mangle]
extern "C" fn syscall_trap_handler() {
    use core::arch::asm;
    
    let mut scause: usize;
    let mut sepc: usize;
    let mut a0: usize; // syscall number  
    let mut a1: usize; // fd
    let mut a2: usize; // buffer ptr
    let mut a3: usize; // count
    
    unsafe {
        asm!(
            "csrr {}, scause",
            "csrr {}, sepc",
            "mv {}, a0",
            "mv {}, a1", 
            "mv {}, a2",
            "mv {}, a3",
            out(reg) scause,
            out(reg) sepc,
            out(reg) a0,
            out(reg) a1,
            out(reg) a2,
            out(reg) a3
        );
    }
    
    let exception_code = scause & 0x7FFFFFFFFFFFFFFF;
    
    // Handle system calls (ecall from user mode = 8, ecall from supervisor mode = 9)
    if exception_code == 8 || exception_code == 9 {
        console_println!("📞 System call: SYS_{} fd={} ptr=0x{:x} len={}", a0, a1, a2, a3);
        
        // Handle SYS_WRITE (64)
        if a0 == 64 && a1 == 1 { // SYS_WRITE to stdout
            let message_ptr = a2 as *const u8;
            let message_len = a3;
            
            console_println!("📝 Writing {} bytes to stdout", message_len);
            
            // Print the message
            if message_len > 0 && message_len < 1024 {
                let mut uart = crate::UART.lock();
                for i in 0..message_len {
                    let byte = unsafe { core::ptr::read_volatile(message_ptr.add(i)) };
                    uart.putchar(byte);
                }
                drop(uart);
                
                console_println!("✅ Successfully wrote {} bytes", message_len);
                
                // Return bytes written and continue
                unsafe {
                    asm!(
                        "mv a0, {}",      // Return bytes written
                        "csrw sepc, {}",  // Skip ecall instruction
                        "sret",           // Return to user program
                        in(reg) message_len,
                        in(reg) sepc + 4,
                    );
                }
            }
        }
        
        // For unsupported syscalls, just return 0 and continue
        unsafe {
            asm!(
                "mv a0, zero",
                "csrw sepc, {}",
                "sret", 
                in(reg) sepc + 4,
            );
        }
    } else {
        // Handle other exceptions
        console_println!("❌ Unhandled exception: code={}", exception_code);
        
        // Just return to avoid hanging the system
        unsafe {
            asm!(
                "csrw sepc, {}",
                "sret",
                in(reg) sepc + 4,
            );
        }
    }
}

/// Execute user program with software MMU virtual memory translation
unsafe fn execute_user_program_with_software_mmu(entry_point: usize, loaded_elf: &LoadedElf) {
    console_println!("ℹ️ Executing with Software Virtual Memory Manager...");
    
    // Always try to find the executable segment for virtual-to-physical mapping
    // Don't assume entry points >= 0x80000000 are physical addresses
    console_println!("📍 Looking for executable segment containing entry point 0x{:08x}", entry_point);
    
    // Find the executable segment to get the virtual-to-physical mapping
    for segment in &loaded_elf.segments {
        if segment.flags & PF_X != 0 && segment.data_addr.is_some() {
            let data_addr = segment.data_addr.unwrap();
            let segment_start = segment.vaddr as usize;
            let segment_end = segment_start + segment.data_size;
            
            console_println!("📍 Found executable segment:");
            console_println!("   Virtual range: 0x{:08x} - 0x{:08x}", segment_start, segment_end);
            console_println!("   Physical base: 0x{:08x}", data_addr);
            console_println!("   Size: {} bytes", segment.data_size);
            
            // Check if entry point is within this segment
            if entry_point >= segment_start && entry_point < segment_end {
                // Calculate the entry point offset within the segment
                let entry_offset = entry_point - segment_start;
                let physical_entry = data_addr + entry_offset;
                
                console_println!("ℹ️ Virtual entry point: 0x{:08x}", entry_point);
                console_println!("ℹ️ Physical entry point: 0x{:08x}", physical_entry);
                console_println!("ℹ️ Entry offset: 0x{:x}", entry_offset);
                
                // Execute the program using the translated physical address
                console_println!("ℹ️ Executing with software virtual memory translation...");
                execute_user_program(physical_entry);
                
                return;
            }
        }
    }
    
    console_println!("❌ Entry point 0x{:08x} not found in any executable segment", entry_point);
    console_println!("Available segments:");
    for (i, segment) in loaded_elf.segments.iter().enumerate() {
        let perms = crate::elf::segment_permissions(segment.flags);
        console_println!("  Segment {}: 0x{:08x} - 0x{:08x} [{}]", 
            i, segment.vaddr, segment.vaddr + segment.memsz, perms);
    }
}

/// Execute user program at the given virtual address (Hardware MMU enabled)
unsafe fn execute_user_program_virtual(entry_point: usize) {
    use core::arch::asm;
    
    // Validate entry point alignment (RISC-V requires 4-byte alignment)
    if entry_point % 4 != 0 {
        console_println!("❌ Entry point 0x{:x} is not 4-byte aligned!", entry_point);
        return;
    }
    
    console_println!("🏃 About to execute at virtual address 0x{:x}", entry_point);
    
    console_println!("ℹ️ User space trap handling already set up by main trap handler");
    
    console_println!("ℹ️ Switching to user address space...");
    if let Err(e) = crate::memory::mmu::switch_to_user_space() {
        console_println!("❌ Failed to switch to user space: {}", e);
        return;
    }
    console_println!("✅ Switched to user address space successfully!");
    
    // Set up user stack
    console_println!("📚 Setting up user stack...");
    let stack_size = 8192; // 8KB stack
    let stack_vaddr = 0x7FFFF000; // High address for stack
    
    // Allocate physical memory for stack
    let stack_paddr = match crate::memory::allocate_aligned_memory(stack_size, 4096) {
        Some(addr) => addr,
        None => {
            console_println!("❌ Failed to allocate user stack");
            return;
        }
    };
    
    // Map stack into user space
    if let Err(e) = crate::memory::mmu::map_elf_segment(
        stack_vaddr, 
        stack_paddr, 
        stack_size, 
        crate::memory::mmu::PTE_R | crate::memory::mmu::PTE_W | crate::memory::mmu::PTE_U
    ) {
        console_println!("❌ Failed to map user stack: {}", e);
        return;
    }
    
    let stack_top = stack_vaddr + stack_size - 8; // Leave some space at top
    console_println!("✅ User stack mapped at 0x{:x} - 0x{:x}, SP will be 0x{:x}", 
        stack_vaddr, stack_vaddr + stack_size, stack_top);
    
    console_println!("ℹ️ Executing user program...");
    
    // Execute the user function with proper stack setup
    let result: i32;
    unsafe {
        use core::arch::asm;
        asm!(
            "mv t0, sp",           // Save current kernel stack pointer
            "mv sp, {stack_top}",  // Set user stack pointer
            "jalr {entry}",        // Jump to user program
            "mv sp, t0",           // Restore kernel stack pointer
            "mv a0, a0",           // Result is already in a0
            stack_top = in(reg) stack_top,
            entry = in(reg) entry_point,
            out("a0") result,      // Explicit register for output
        );
    }
    
    // Switch back to kernel space for completion message
    let _ = crate::memory::mmu::switch_to_kernel_space();
    console_println!("✅ User program completed with result: {}", result);
}

/// Execute user program at the given physical address
/// This implementation sets up proper user mode execution with syscall support
unsafe fn execute_user_program(entry_point: usize) {
    use core::arch::asm;
    
    console_println!("🎬 Setting up execution environment...");
    
    // Validate entry point alignment (RISC-V requires 4-byte alignment)
    if entry_point % 4 != 0 {
        console_println!("❌ Entry point 0x{:x} is not 4-byte aligned!", entry_point);
        return;
    }
    
    // Check if entry point looks reasonable (within our allocated memory)
    if entry_point < 0x80000000 || entry_point > 0x90000000 {
        console_println!("❌ Entry point 0x{:x} looks suspicious!", entry_point);
        return;
    }
    
    // Examine the instructions at the entry point
    console_println!("ℹ️ Examining instructions at entry point:");
    let instr_ptr = entry_point as *const u32;
    for i in 0..4 {
        let instr = core::ptr::read_volatile(instr_ptr.add(i));
        console_println!("   0x{:08x}: 0x{:08x}", entry_point + (i * 4), instr);
    }
    
    // Allocate a simple stack for the user program (4KB)
    if let Some(stack_addr) = crate::memory::allocate_memory(4096) {
        let stack_top = stack_addr + 4096;
        console_println!("📚 Allocated stack at 0x{:x}-0x{:x}", stack_addr, stack_top);
        
        console_println!("ℹ️ About to execute user program...");
        console_println!("   Entry point: 0x{:x}", entry_point);
        console_println!("   Stack pointer: 0x{:x}", stack_top);
        
        console_println!("ℹ️ Executing in user mode with syscall support...");
        
        // Create a wrapper that calls the user program and then exits
        console_println!("ℹ️ Setting up user program wrapper...");
        
        // For now, let's try a simpler approach - execute in supervisor mode but with syscall interception
        let result = execute_with_syscall_support(entry_point);
        
        console_println!("✅ User program completed with result: {}", result);
        
        // Deallocate the stack
        crate::memory::deallocate_memory(stack_addr, 4096);
    } else {
        console_println!("❌ Failed to allocate stack for user program");
    }
} 