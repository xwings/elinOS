//! ELF Module for elinOS
//!
//! This module provides a modular ELF64 implementation for RISC-V, including:
//! - ELF parsing and validation
//! - Memory loading and allocation
//! - Program execution support
//! - System call handling for user programs

// Re-export core types for backward compatibility
pub use constants::*;
pub use error::{ElfError, ElfResult};
pub use structures::{Elf64Header, Elf64ProgramHeader, LoadedElf, ElfSegment};
pub use parser::ElfParser;
pub use loader::ElfLoader;

// Modules
pub mod constants;
pub mod error;
pub mod structures;
pub mod parser;
pub mod loader;

// TODO: These modules will be created in follow-up work
// pub mod executor;
// pub mod syscall;
// pub mod memory;

// For now, include the remaining functions from the original elf.rs
// These will be moved to their respective modules later

use crate::console_println;

/// Main ELF execution function - coordinates loading and execution
pub fn execute_elf(loaded_elf: &LoadedElf) -> ElfResult<()> {
    console_println!("[i] Executing ELF at entry point 0x{:x}", loaded_elf.entry_point);
    
    // Always use software MMU for now (hardware MMU has issues)
    console_println!("[i] Virtual Memory enabled - executing with software MMU");
    
    // Execute with software virtual memory translation
    console_println!("[i] Executing at virtual entry point: 0x{:x}", loaded_elf.entry_point);
    
    unsafe {
        execute_user_program_with_software_mmu(loaded_elf.entry_point as usize, loaded_elf);
    }
    
    Ok(())
}

// Temporary inclusion of execution functions
// TODO: Move these to executor.rs and syscall.rs modules

/// Execute user program with temporary syscall support
unsafe fn execute_with_syscall_support(entry_point: usize) -> usize {
    use core::arch::asm;
    
    // Allocate user stack
    let user_stack = match crate::memory::allocate_memory(8192, 8) {
        Ok(addr) => addr.as_ptr() as usize,
        Err(_) => {
            console_println!("[x] Failed to allocate user stack");
            return 0;
        }
    };
    let user_stack_top = user_stack + 8192;
    
    console_println!("[i] User stack allocated: 0x{:x} - 0x{:x}", user_stack, user_stack_top);
    
    // Create a small exit stub that will be called when the user program returns
    let exit_stub = match crate::memory::allocate_memory(32, 8) {
        Ok(addr) => addr.as_ptr() as usize,
        Err(_) => {
            console_println!("[x] Failed to allocate exit stub");
            // Note: deallocate_memory signature needs to be fixed too
            return 0;
        }
    };
    
    // Write exit stub code: li a7, 93; ecall; ebreak (breakpoint)
    let exit_stub_ptr = exit_stub as *mut u32;
    exit_stub_ptr.write_volatile(0x05d00893); // li a7, 93 (addi a7, x0, 93)
    exit_stub_ptr.add(1).write_volatile(0x00000073); // ecall
    exit_stub_ptr.add(2).write_volatile(0x00100073); // ebreak (breakpoint)
    exit_stub_ptr.add(3).write_volatile(0x00000013); // nop (padding)
    
    console_println!("[i] Exit stub created at 0x{:x}", exit_stub);
    
    // Set up proper user mode status
    let user_status = 0x00000020; // SPIE=1, SPP=0 (user mode)
    console_println!("   Status: 0x{:x}", user_status);
    
    console_println!("[i] About to jump to user mode...");
    
    let result: usize;
    unsafe {
        asm!(
            "csrw sepc, {entry}",
            "csrw sstatus, {status}",
            "mv sp, {stack}",
            "mv ra, {exit_stub}",
            "sret",
            "mv {result}, a0",
            entry = in(reg) entry_point,
            status = in(reg) user_status,
            stack = in(reg) user_stack_top,
            exit_stub = in(reg) exit_stub,
            result = out(reg) result,
        );
    }
    
    console_println!("[o] Returned from user mode. Result: {}", result);
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
            
            if message_len > 0 && message_len < 1024 {
                let mut uart = crate::UART.lock();
                for i in 0..message_len {
                    let byte = unsafe { core::ptr::read_volatile(message_ptr.add(i)) };
                    uart.putchar(byte);
                }
                drop(uart);
                
                unsafe {
                    asm!(
                        "mv a0, {}",
                        "csrw sepc, {}",
                        "sret",
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
        console_println!("[x] Unhandled exception: code={}", exception_code);
        
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
    // Find the executable segment to get the virtual-to-physical mapping
    for segment in &loaded_elf.segments {
        if segment.flags & PF_X != 0 && segment.data_addr.is_some() {
            let data_addr = segment.data_addr.unwrap();
            let segment_start = segment.vaddr as usize;
            let segment_end = segment_start + segment.data_size;
            
            if entry_point >= segment_start && entry_point < segment_end {
                let entry_offset = entry_point - segment_start;
                let physical_entry = data_addr + entry_offset;
                
                execute_user_program(physical_entry);
                return;
            }
        }
    }
    
    console_println!("[x] Entry point 0x{:08x} not found in any executable segment", entry_point);
    for (i, segment) in loaded_elf.segments.iter().enumerate() {
        let perms = segment_permissions(segment.flags);
        console_println!("      Segment {}: 0x{:08x} - 0x{:08x} [{}]", 
            i, segment.vaddr, segment.vaddr + segment.memsz, perms);
    }
}

/// Execute user program at the given physical address
unsafe fn execute_user_program(entry_point: usize) {
    
    console_println!("[i] Setting up execution environment...");
    
    // Validate entry point alignment (RISC-V requires 4-byte alignment)
    if entry_point % 4 != 0 {
        console_println!("[x] Entry point 0x{:x} is not 4-byte aligned!", entry_point);
        return;
    }
    
    // Check if entry point looks reasonable (within our allocated memory)
    if entry_point < 0x80000000 || entry_point > 0x90000000 {
        console_println!("[x] Entry point 0x{:x} looks suspicious!", entry_point);
        return;
    }
    
    // Examine the instructions at the entry point
    console_println!("[i] Examining instructions at entry point:");
    let instr_ptr = entry_point as *const u32;
    for i in 0..4 {
        let instr = core::ptr::read_volatile(instr_ptr.add(i));
        console_println!("   0x{:08x}: 0x{:08x}", entry_point + (i * 4), instr);
    }
    
    // Allocate a simple stack for the user program (4KB)
    if let Ok(stack_addr) = crate::memory::allocate_memory(4096, 8) {
        let stack_addr = stack_addr.as_ptr() as usize;
        let stack_top = stack_addr + 4096;
        console_println!("[i] Allocated stack at 0x{:x}-0x{:x}", stack_addr, stack_top);
        
        console_println!("[i] About to execute user program...");
        console_println!("   Entry point: 0x{:x}", entry_point);
        console_println!("   Stack pointer: 0x{:x}", stack_top);
        
        console_println!("[i] Executing in user mode with syscall support...");
        
        let result = execute_with_syscall_support(entry_point);
        
        console_println!("[o] User program completed with result: {}", result);
        
        // Deallocate the stack
        if let Some(ptr) = core::ptr::NonNull::new(stack_addr as *mut u8) {
            crate::memory::deallocate_memory(ptr, 4096);
        }
    } else {
        console_println!("[x] Failed to allocate stack for user program");
    }
} 