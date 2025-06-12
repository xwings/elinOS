//! Trap handling for RISC-V
//! 
//! This module provides exception and interrupt handling for the elinOS kernel.
//! It includes proper trap vector setup and detailed crash information dumping.

use core::arch::asm;
use core::fmt::Write;

/// RISC-V trap causes
#[derive(Debug, Clone, Copy)]
#[repr(u64)]
pub enum TrapCause {
    // Exceptions (scause MSB = 0)
    InstructionAddressMisaligned = 0,
    InstructionAccessFault = 1,
    IllegalInstruction = 2,
    Breakpoint = 3,
    LoadAddressMisaligned = 4,
    LoadAccessFault = 5,
    StoreAddressMisaligned = 6,
    StoreAccessFault = 7,
    EnvironmentCallFromUMode = 8,
    EnvironmentCallFromSMode = 9,
    EnvironmentCallFromMMode = 11,
    InstructionPageFault = 12,
    LoadPageFault = 13,
    StorePageFault = 15,
    
    // Interrupts (scause MSB = 1)
    SupervisorSoftwareInterrupt = 1 | (1 << 63),
    MachineSoftwareInterrupt = 3 | (1 << 63),
    SupervisorTimerInterrupt = 5 | (1 << 63),
    MachineTimerInterrupt = 7 | (1 << 63),
    SupervisorExternalInterrupt = 9 | (1 << 63),
    MachineExternalInterrupt = 11 | (1 << 63),
    
    Unknown = 0xFFFFFFFFFFFFFFFF,
}

impl From<u64> for TrapCause {
    fn from(value: u64) -> Self {
        match value {
            0 => TrapCause::InstructionAddressMisaligned,
            1 => TrapCause::InstructionAccessFault,
            2 => TrapCause::IllegalInstruction,
            3 => TrapCause::Breakpoint,
            4 => TrapCause::LoadAddressMisaligned,
            5 => TrapCause::LoadAccessFault,
            6 => TrapCause::StoreAddressMisaligned,
            7 => TrapCause::StoreAccessFault,
            8 => TrapCause::EnvironmentCallFromUMode,
            9 => TrapCause::EnvironmentCallFromSMode,
            11 => TrapCause::EnvironmentCallFromMMode,
            12 => TrapCause::InstructionPageFault,
            13 => TrapCause::LoadPageFault,
            15 => TrapCause::StorePageFault,
            v if v & (1 << 63) != 0 => match v & !((1u64) << 63) {
                1 => TrapCause::SupervisorSoftwareInterrupt,
                3 => TrapCause::MachineSoftwareInterrupt,
                5 => TrapCause::SupervisorTimerInterrupt,
                7 => TrapCause::MachineTimerInterrupt,
                9 => TrapCause::SupervisorExternalInterrupt,
                11 => TrapCause::MachineExternalInterrupt,
                _ => TrapCause::Unknown,
            },
            _ => TrapCause::Unknown,
        }
    }
}

/// Trap context - registers saved during trap
#[repr(C)]
#[derive(Debug)]
pub struct TrapContext {
    pub x: [u64; 32],  // General purpose registers x0-x31
    pub sstatus: u64,  // Supervisor status
    pub sepc: u64,     // Supervisor exception program counter
    pub stval: u64,    // Supervisor trap value
    pub scause: u64,   // Supervisor cause
}

impl TrapContext {
    pub fn new() -> Self {
        Self {
            x: [0; 32],
            sstatus: 0,
            sepc: 0,
            stval: 0,
            scause: 0,
        }
    }
}

/// Initialize trap handling
pub fn init_trap_handling() {
    unsafe {
        // Set trap vector to our handler
        asm!(
            "la t0, {trap_vector}",
            "csrw stvec, t0",
            trap_vector = sym trap_vector,
            options(nostack)
        );
        
        // Enable interrupts in sstatus
        asm!(
            "csrr t0, sstatus",
            "ori t0, t0, 2",  // Set SIE bit
            "csrw sstatus, t0",
            options(nostack)
        );
    }
}

/// Dump detailed crash information
pub fn dump_crash_info(ctx: &TrapContext) {
    let cause = TrapCause::from(ctx.scause);
    let is_interrupt = (ctx.scause & (1 << 63)) != 0;
    
    let mut uart = crate::UART.lock();
    let _ = writeln!(uart, "=====================================");
    let _ = writeln!(uart, "💥 KERNEL TRAP/CRASH DETECTED! 💥");
    let _ = writeln!(uart, "=====================================");
    
    let _ = writeln!(uart, "Trap Type: {}", if is_interrupt { "INTERRUPT" } else { "EXCEPTION" });
    let _ = writeln!(uart, "Cause: {:?} (0x{:016x})", cause, ctx.scause);
    let _ = writeln!(uart, "PC (sepc): 0x{:016x}", ctx.sepc);
    let _ = writeln!(uart, "Trap Value (stval): 0x{:016x}", ctx.stval);
    let _ = writeln!(uart, "Status (sstatus): 0x{:016x}", ctx.sstatus);
    let _ = writeln!(uart);
    
    // Dump registers
    let _ = writeln!(uart, "📋 REGISTER DUMP:");
    let _ = writeln!(uart, "─────────────────────────────────────");
    for i in 0..32 {
        let reg_name = match i {
            0 => "zero",
            1 => "ra  ",
            2 => "sp  ",
            3 => "gp  ",
            4 => "tp  ",
            5 => "t0  ",
            6 => "t1  ",
            7 => "t2  ",
            8 => "s0  ",
            9 => "s1  ",
            10 => "a0  ",
            11 => "a1  ",
            12 => "a2  ",
            13 => "a3  ",
            14 => "a4  ",
            15 => "a5  ",
            16 => "a6  ",
            17 => "a7  ",
            18 => "s2  ",
            19 => "s3  ",
            20 => "s4  ",
            21 => "s5  ",
            22 => "s6  ",
            23 => "s7  ",
            24 => "s8  ",
            25 => "s9  ",
            26 => "s10 ",
            27 => "s11 ",
            28 => "t3  ",
            29 => "t4  ",
            30 => "t5  ",
            31 => "t6  ",
            _ => "??? ",
        };
        
        if i % 4 == 0 && i > 0 {
            let _ = writeln!(uart);
        }
        let _ = write!(uart, "x{:2}({}): 0x{:016x}  ", i, reg_name, ctx.x[i]);
    }
    let _ = writeln!(uart);
    let _ = writeln!(uart);
    
    // Additional context based on trap type
    match cause {
        TrapCause::IllegalInstruction => {
            let _ = writeln!(uart, "🚨 ILLEGAL INSTRUCTION at PC: 0x{:016x}", ctx.sepc);
            let _ = writeln!(uart, "   This usually indicates:");
            let _ = writeln!(uart, "   - Corrupted code");
            let _ = writeln!(uart, "   - Jump to invalid address");
            let _ = writeln!(uart, "   - Unsupported instruction");
        }
        TrapCause::LoadAccessFault | TrapCause::StoreAccessFault => {
            let _ = writeln!(uart, "🚨 MEMORY ACCESS FAULT");
            let _ = writeln!(uart, "   Faulting address: 0x{:016x}", ctx.stval);
            let _ = writeln!(uart, "   PC: 0x{:016x}", ctx.sepc);
            let _ = writeln!(uart, "   This usually indicates:");
            let _ = writeln!(uart, "   - Access to unmapped memory");
            let _ = writeln!(uart, "   - Permission violation");
            let _ = writeln!(uart, "   - Hardware fault");
        }
        TrapCause::LoadAddressMisaligned | TrapCause::StoreAddressMisaligned => {
            let _ = writeln!(uart, "🚨 MISALIGNED MEMORY ACCESS");
            let _ = writeln!(uart, "   Faulting address: 0x{:016x}", ctx.stval);
            let _ = writeln!(uart, "   PC: 0x{:016x}", ctx.sepc);
        }
        TrapCause::InstructionAddressMisaligned => {
            let _ = writeln!(uart, "🚨 MISALIGNED INSTRUCTION FETCH");
            let _ = writeln!(uart, "   Faulting PC: 0x{:016x}", ctx.stval);
        }
        TrapCause::Breakpoint => {
            let _ = writeln!(uart, "🔍 BREAKPOINT HIT at PC: 0x{:016x}", ctx.sepc);
        }
        _ => {
            let _ = writeln!(uart, "ℹ️  Additional debugging info:");
            let _ = writeln!(uart, "   Raw scause: 0x{:016x}", ctx.scause);
            let _ = writeln!(uart, "   Raw stval: 0x{:016x}", ctx.stval);
        }
    }
    
    let _ = writeln!(uart);
    let _ = writeln!(uart, "=====================================");
    let _ = writeln!(uart, "System halted. Reset required.");
    let _ = writeln!(uart, "=====================================");
}

/// Handle user space system calls
fn handle_user_syscall(ctx: &mut TrapContext) {
    let syscall_num = ctx.x[17]; // a7 register contains syscall number
    let arg1 = ctx.x[10]; // a0
    let arg2 = ctx.x[11]; // a1  
    let arg3 = ctx.x[12]; // a2
    let arg4 = ctx.x[13]; // a3
    
    // Handle SYS_WRITE (64)
    if syscall_num == 64 && arg1 == 1 { // SYS_WRITE to stdout
        let message_ptr = arg2 as *const u8;
        let message_len = arg3 as usize;
        
        // Switch back to kernel space to access UART
        let _ = crate::memory::mmu::switch_to_kernel_space();
        
        // Print the message
        if message_len > 0 && message_len < 1024 {
            let mut uart = crate::UART.lock();
            for i in 0..message_len {
                let byte = unsafe { core::ptr::read_volatile(message_ptr.add(i)) };
                uart.putchar(byte);
            }
            drop(uart);
            
            // Switch back to user space
            let _ = crate::memory::mmu::switch_to_user_space();
            
            // Return bytes written
            ctx.x[10] = message_len as u64; // a0 = return value
        } else {
            // Switch back to user space
            let _ = crate::memory::mmu::switch_to_user_space();
            ctx.x[10] = 0; // Return 0 for invalid length
        }
    } else {
        // For unsupported syscalls, just return 0
        ctx.x[10] = 0;
    }
    
    // Skip the ecall instruction (advance PC by 4 bytes)
    ctx.sepc += 4;
}

/// Main trap handler (called from assembly)
#[no_mangle]
pub extern "C" fn trap_handler(ctx: &mut TrapContext) {
    // Read CSR values
    unsafe {
        asm!(
            "csrr {}, scause",
            "csrr {}, sepc", 
            "csrr {}, stval",
            "csrr {}, sstatus",
            out(reg) ctx.scause,
            out(reg) ctx.sepc,
            out(reg) ctx.stval,
            out(reg) ctx.sstatus,
        );
    }
    
    let cause = TrapCause::from(ctx.scause);
    let is_interrupt = (ctx.scause & (1 << 63)) != 0;
    
    if is_interrupt {
        // Handle interrupts
        let mut uart = crate::UART.lock();
        match cause {
            TrapCause::SupervisorTimerInterrupt => {
                // Handle timer interrupt
                let _ = writeln!(uart, "⏰ Timer interrupt");
                // Clear timer interrupt and set next timer
                // TODO: Implement proper timer handling
            }
            TrapCause::SupervisorExternalInterrupt => {
                let _ = writeln!(uart, "🔌 External interrupt");
                // TODO: Handle external interrupts (UART, etc.)
            }
            _ => {
                let _ = writeln!(uart, "❓ Unknown interrupt: {:?}", cause);
            }
        }
    } else {
        // Handle exceptions
        match cause {
            TrapCause::EnvironmentCallFromUMode => {
                // Handle system calls from user mode
                handle_user_syscall(ctx);
            }
            _ => {
                // Other exceptions are usually fatal
                dump_crash_info(ctx);
                
                // Halt the system
                loop {
                    unsafe {
                        asm!("wfi");
                    }
                }
            }
        }
    }
}

/// Assembly trap vector - saves context and calls trap_handler
#[unsafe(naked)]
#[no_mangle]
pub unsafe extern "C" fn trap_vector() {
    core::arch::naked_asm!(
        // Save all registers to stack
        "addi sp, sp, -256",  // Make room for TrapContext
        
        // Save x1-x31 (x0 is always 0)
        "sd x1, 8(sp)",
        "sd x2, 16(sp)",
        "sd x3, 24(sp)",
        "sd x4, 32(sp)",
        "sd x5, 40(sp)",
        "sd x6, 48(sp)",
        "sd x7, 56(sp)",
        "sd x8, 64(sp)",
        "sd x9, 72(sp)",
        "sd x10, 80(sp)",
        "sd x11, 88(sp)",
        "sd x12, 96(sp)",
        "sd x13, 104(sp)",
        "sd x14, 112(sp)",
        "sd x15, 120(sp)",
        "sd x16, 128(sp)",
        "sd x17, 136(sp)",
        "sd x18, 144(sp)",
        "sd x19, 152(sp)",
        "sd x20, 160(sp)",
        "sd x21, 168(sp)",
        "sd x22, 176(sp)",
        "sd x23, 184(sp)",
        "sd x24, 192(sp)",
        "sd x25, 200(sp)",
        "sd x26, 208(sp)",
        "sd x27, 216(sp)",
        "sd x28, 224(sp)",
        "sd x29, 232(sp)",
        "sd x30, 240(sp)",
        "sd x31, 248(sp)",
        
        // Call trap handler with context pointer
        "mv a0, sp",
        "call {trap_handler}",
        
        // Restore registers
        "ld x1, 8(sp)",
        "ld x2, 16(sp)",
        "ld x3, 24(sp)",
        "ld x4, 32(sp)",
        "ld x5, 40(sp)",
        "ld x6, 48(sp)",
        "ld x7, 56(sp)",
        "ld x8, 64(sp)",
        "ld x9, 72(sp)",
        "ld x10, 80(sp)",
        "ld x11, 88(sp)",
        "ld x12, 96(sp)",
        "ld x13, 104(sp)",
        "ld x14, 112(sp)",
        "ld x15, 120(sp)",
        "ld x16, 128(sp)",
        "ld x17, 136(sp)",
        "ld x18, 144(sp)",
        "ld x19, 152(sp)",
        "ld x20, 160(sp)",
        "ld x21, 168(sp)",
        "ld x22, 176(sp)",
        "ld x23, 184(sp)",
        "ld x24, 192(sp)",
        "ld x25, 200(sp)",
        "ld x26, 208(sp)",
        "ld x27, 216(sp)",
        "ld x28, 224(sp)",
        "ld x29, 232(sp)",
        "ld x30, 240(sp)",
        "ld x31, 248(sp)",
        
        "addi sp, sp, 256",
        "sret",
        
        trap_handler = sym trap_handler
    );
} 