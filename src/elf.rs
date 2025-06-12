// ELF Loader for elinOS
// Supports loading and parsing ELF64 binaries for RISC-V

use crate::{UART, console_println};
use crate::memory::mmu::{self, PTE_R, PTE_W, PTE_X, PTE_U};
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
        let phoff = header.e_phoff;
        let phentsize = header.e_phentsize;
        
        console_println!("🔄 Loading ELF binary with MMU support:");
        console_println!("  Entry point: 0x{:x}", entry_point);
        console_println!("  Program headers: {}", phnum);

        // Create user address space if MMU is enabled
        if mmu::is_mmu_enabled() {
            console_println!("🗺️  Creating user address space...");
            if let Err(e) = mmu::create_user_address_space() {
                console_println!("❌ Failed to create user address space: {}", e);
                return Err(ElfError::LoadError);
            }
        }

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
                
                console_println!("  Segment {}: 0x{:x} - 0x{:x} ({} bytes) flags: 0x{:x} ({})",
                    i, p_vaddr, p_vaddr + p_memsz, p_memsz, p_flags, segment_permissions(p_flags));

                let mut data_addr = None;
                let mut data_size = 0;

                // Copy segment data to allocated memory
                if p_filesz > 0 {
                    let file_offset = p_offset as usize;
                    let file_size = p_filesz as usize;
                    
                    if file_offset + file_size > data.len() {
                        return Err(ElfError::LoadError);
                    }

                    // Get the segment data from the ELF file
                    let segment_data = &data[file_offset..file_offset + file_size];
                    
                    // Allocate physical memory for the segment
                    if let Some(allocated_addr) = crate::memory::allocate_memory(p_memsz as usize) {
                        console_println!("📋 Allocated {} bytes at physical 0x{:x} for virtual 0x{:x}", 
                            p_memsz, allocated_addr, p_vaddr);
                        
                        // Copy the segment data to allocated memory
                        let dest_ptr = allocated_addr as *mut u8;
                        unsafe {
                            core::ptr::copy_nonoverlapping(
                                segment_data.as_ptr(),
                                dest_ptr,
                                file_size
                            );
                            
                            // Zero out the remaining memory if memsz > filesz
                            if p_memsz > p_filesz {
                                let zero_start = dest_ptr.add(file_size);
                                let zero_size = (p_memsz - p_filesz) as usize;
                                core::ptr::write_bytes(zero_start, 0, zero_size);
                            }
                        }
                        
                        // Map virtual address to physical address if MMU is enabled
                        if mmu::is_mmu_enabled() {
                            // Convert ELF flags to MMU flags
                            let mut mmu_flags = PTE_U; // User accessible
                            if p_flags & PF_R != 0 { mmu_flags |= PTE_R; }
                            if p_flags & PF_W != 0 { mmu_flags |= PTE_W; }
                            if p_flags & PF_X != 0 { mmu_flags |= PTE_X; }
                            
                            console_println!("🗺️  Mapping virtual 0x{:x} -> physical 0x{:x} (flags: 0x{:x})",
                                p_vaddr, allocated_addr, mmu_flags);
                            
                            if let Err(e) = mmu::map_elf_segment(
                                p_vaddr as usize, 
                                allocated_addr, 
                                p_memsz as usize, 
                                mmu_flags
                            ) {
                                console_println!("❌ Failed to map segment: {}", e);
                                return Err(ElfError::LoadError);
                            }
                        }
                        
                        data_addr = Some(allocated_addr);
                        data_size = p_memsz as usize;
                        
                        console_println!("✅ Copied {} bytes to 0x{:x}", file_size, allocated_addr);
                    } else {
                        console_println!("❌ Failed to allocate memory for segment");
                        return Err(ElfError::LoadError);
                    }
                }

                let segment = ElfSegment {
                    vaddr: p_vaddr,
                    memsz: p_memsz,
                    flags: p_flags,
                    data_addr,
                    data_size,
                };

                if segments.push(segment).is_err() {
                    return Err(ElfError::LoadError);
                }
            }
        }

        let loaded_elf = LoadedElf {
            entry_point,
            segments,
        };

        console_println!("✅ ELF loaded successfully with {} segments, entry at 0x{:x}", 
            loaded_elf.segments.len(), loaded_elf.entry_point);

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

    /// Execute a loaded ELF binary with MMU support
    pub fn execute_elf(&self, loaded_elf: &LoadedElf) -> Result<(), ElfError> {
        console_println!("🚀 Executing ELF at entry point 0x{:x}", loaded_elf.entry_point);
        
        if mmu::is_mmu_enabled() {
            console_println!("🗺️  MMU enabled - executing with virtual memory");
            
            // Execute at virtual address (the function will handle address space switching)
            console_println!("🏃 Jumping to virtual entry point: 0x{:x}", loaded_elf.entry_point);
            
            unsafe {
                execute_user_program_virtual(loaded_elf.entry_point as usize);
            }
            
            return Ok(());
        } else {
            // Legacy mode: find physical address
            console_println!("🔧 MMU disabled - using physical addresses");
            
            // Find the executable segment
            for segment in &loaded_elf.segments {
                if segment.flags & PF_X != 0 && segment.data_addr.is_some() {
                    let data_addr = segment.data_addr.unwrap();
                    
                    console_println!("📍 Found executable segment at 0x{:x}", data_addr);
                    console_println!("🎯 Virtual address: 0x{:x}, Physical address: 0x{:x}", 
                        segment.vaddr, data_addr);
                    
                    // Calculate the entry point offset within the segment
                    let entry_offset = (loaded_elf.entry_point - segment.vaddr) as usize;
                    let physical_entry = data_addr + entry_offset;
                    
                    console_println!("🏃 Jumping to physical entry point: 0x{:x}", physical_entry);
                    console_println!("   (Virtual entry: 0x{:x}, Segment base: 0x{:x}, Offset: 0x{:x})",
                        loaded_elf.entry_point, segment.vaddr, entry_offset);
                    
                    // Execute the program by jumping to the entry point
                    unsafe {
                        execute_user_program(physical_entry);
                    }
                    
                    return Ok(());
                }
            }
            
            console_println!("❌ No executable segment found");
            Err(ElfError::LoadError)
        }
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

/// Execute user program with temporary syscall support
unsafe fn execute_with_syscall_support(entry_point: usize) -> usize {
    use core::arch::asm;
    
    // Save the current trap handler
    let mut old_mtvec: usize;
    asm!("csrr {}, mtvec", out(reg) old_mtvec);
    
    // Set our temporary trap handler
    asm!("csrw mtvec, {}", in(reg) syscall_trap_handler as usize);
    
    console_println!("🛡️  Temporary syscall handler installed");
    
    // Execute the user function
    let user_func: extern "C" fn() -> usize = core::mem::transmute(entry_point);
    let result = user_func();
    
    // Restore the original trap handler
    asm!("csrw mtvec, {}", in(reg) old_mtvec);
    
    console_println!("🔄 Original trap handler restored");
    
    result
}

/// Temporary trap handler specifically for user program execution
#[no_mangle]
extern "C" fn syscall_trap_handler() {
    use core::arch::asm;
    
    let mut mcause: usize;
    let mut mepc: usize;
    let mut a0: usize; // syscall number  
    let mut a1: usize; // fd
    let mut a2: usize; // buffer ptr
    let mut a3: usize; // count
    
    unsafe {
        asm!(
            "csrr {}, mcause",
            "csrr {}, mepc",
            "mv {}, a0",
            "mv {}, a1", 
            "mv {}, a2",
            "mv {}, a3",
            out(reg) mcause,
            out(reg) mepc,
            out(reg) a0,
            out(reg) a1,
            out(reg) a2,
            out(reg) a3
        );
    }
    
    let exception_code = mcause & 0x7FFFFFFFFFFFFFFF;
    
    // Handle system calls (ecall)
    if exception_code == 8 {
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
                        "csrw mepc, {}",  // Skip ecall instruction
                        "mret",           // Return to user program
                        in(reg) message_len,
                        in(reg) mepc + 4,
                        options(noreturn)
                    );
                }
            }
        }
        
        // For unsupported syscalls, just return 0 and continue
        unsafe {
            asm!(
                "mv a0, zero",
                "csrw mepc, {}",
                "mret", 
                in(reg) mepc + 4,
                options(noreturn)
            );
        }
    } else {
        // Handle other exceptions
        console_println!("💥 Unhandled exception: code={}", exception_code);
        
        // Just return to avoid hanging the system
        unsafe {
            asm!(
                "csrw mepc, {}",
                "mret",
                in(reg) mepc + 4,
                options(noreturn)
            );
        }
    }
}



/// Execute user program at the given virtual address (MMU enabled)
unsafe fn execute_user_program_virtual(entry_point: usize) {
    use core::arch::asm;
    
    // Validate entry point alignment (RISC-V requires 4-byte alignment)
    if entry_point % 4 != 0 {
        console_println!("❌ Entry point 0x{:x} is not 4-byte aligned!", entry_point);
        return;
    }
    
    console_println!("🏃 About to execute at virtual address 0x{:x}", entry_point);
    
    console_println!("🔄 User space trap handling already set up by main trap handler");
    
    console_println!("🔄 Switching to user address space...");
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
    
    console_println!("🎯 Executing user program...");
    
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
/// This is a simplified implementation for kernel-mode execution
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
    console_println!("🔍 Examining instructions at entry point:");
    let instr_ptr = entry_point as *const u32;
    for i in 0..4 {
        let instr = core::ptr::read_volatile(instr_ptr.add(i));
        console_println!("   0x{:08x}: 0x{:08x}", entry_point + (i * 4), instr);
    }
    
    // Allocate a simple stack for the user program (4KB)
    if let Some(stack_addr) = crate::memory::allocate_memory(4096) {
        let stack_top = stack_addr + 4096;
        console_println!("📚 Allocated stack at 0x{:x}-0x{:x}", stack_addr, stack_top);
        
        console_println!("🎯 About to execute user program...");
        console_println!("   Entry point: 0x{:x}", entry_point);
        console_println!("   Stack pointer: 0x{:x}", stack_top);
        
        console_println!("🚀 Calling user program...");
        
        // Much simpler approach - just jump to the code directly
        // without changing the stack (use kernel stack for now)
        console_println!("🎯 Attempting direct function call...");
        
        let result: usize;
        
        // Create a function pointer and call it directly
        let user_func: extern "C" fn() -> usize = unsafe { 
            core::mem::transmute(entry_point) 
        };
        
        // With permanent syscall handler, just call the function directly
        console_println!("🔍 Using permanent syscall handler");
        result = user_func();
        
        console_println!("✅ User function returned successfully!");
        
        console_println!("🏁 User program returned with code: {}", result);
        
        // Deallocate the stack
        crate::memory::deallocate_memory(stack_addr, 4096);
    } else {
        console_println!("❌ Failed to allocate stack for user program");
    }
} 