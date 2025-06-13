//! Trap handling for RISC-V
//! 
//! This module provides exception and interrupt handling for the elinOS kernel.
//! It includes proper trap vector setup and detailed crash information dumping.

use core::arch::asm;
use spin::Mutex;
use crate::console_println;
use crate::console_print;

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
    
    console_println!("=====================================");
    console_println!("💥 KERNEL TRAP/CRASH DETECTED! 💥");
    console_println!("=====================================");
    console_println!();
    console_println!("📋 Trap Type: {}", if is_interrupt { "INTERRUPT" } else { "EXCEPTION" });
    console_println!("📋 Cause: {:?} (0x{:016x})", cause, ctx.scause);
    console_println!("📋 PC (sepc): 0x{:016x}", ctx.sepc);
    console_println!("📋 Trap Value (stval): 0x{:016x}", ctx.stval);
    console_println!("📋 Status (sstatus): 0x{:016x}", ctx.sstatus);
    console_println!();
    
    // Detailed register dump
    console_println!("📋 REGISTER DUMP:");
    console_println!("─────────────────────────────────────");
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
            console_println!();
        }
        console_print!("x{:2}({}): 0x{:016x}  ", i, reg_name, ctx.x[i]);
    }
    console_println!();
    console_println!();
    
    // Additional context based on trap type
    match cause {
        TrapCause::IllegalInstruction => {
            console_println!("🚨 ILLEGAL INSTRUCTION at PC: 0x{:016x}", ctx.sepc);
            console_println!("   This usually indicates:");
            console_println!("   - Corrupted code");
            console_println!("   - Jump to invalid address");
            console_println!("   - Unsupported instruction");
        }
        TrapCause::LoadAccessFault | TrapCause::StoreAccessFault => {
            console_println!("🚨 MEMORY ACCESS FAULT");
            console_println!("   Faulting address: 0x{:016x}", ctx.stval);
            console_println!("   PC: 0x{:016x}", ctx.sepc);
            console_println!("   This usually indicates:");
            console_println!("   - Access to unmapped memory");
            console_println!("   - Permission violation");
            console_println!("   - Hardware fault");
        }
        TrapCause::LoadAddressMisaligned | TrapCause::StoreAddressMisaligned => {
            console_println!("🚨 MISALIGNED MEMORY ACCESS");
            console_println!("   Faulting address: 0x{:016x}", ctx.stval);
            console_println!("   PC: 0x{:016x}", ctx.sepc);
        }
        TrapCause::InstructionAddressMisaligned => {
            console_println!("🚨 MISALIGNED INSTRUCTION FETCH");
            console_println!("   Faulting PC: 0x{:016x}", ctx.stval);
        }
        TrapCause::Breakpoint => {
            console_println!("🔍 BREAKPOINT HIT at PC: 0x{:016x}", ctx.sepc);
        }
        _ => {
            console_println!("ℹ️  Additional debugging info:");
            console_println!("   Raw scause: 0x{:016x}", ctx.scause);
            console_println!("   Raw stval: 0x{:016x}", ctx.stval);
        }
    }
    
    console_println!();
    console_println!("=====================================");
    console_println!("System halted. Reset required.");
    console_println!("=====================================");
}

/// Handle system calls by dispatching to the unified syscall module
fn handle_syscall(ctx: &mut TrapContext) {
    // Extract syscall arguments from registers
    let syscall_num = ctx.x[17] as usize; // a7
    let arg0 = ctx.x[10] as usize; // a0
    let arg1 = ctx.x[11] as usize; // a1
    let arg2 = ctx.x[12] as usize; // a2
    let arg3 = ctx.x[13] as usize; // a3
    let arg4 = ctx.x[14] as usize; // a4
    let arg5 = ctx.x[15] as usize; // a5
    
    console_println!("🎉 syscall: {} (a0={}, a1={}, a2={}, a3={})", 
        syscall_num, arg0, arg1, arg2, arg3);
    
    // Create syscall args structure
    let args = crate::syscall::SyscallArgs {
        syscall_number: syscall_num,
        arg0,
        arg1,
        arg2,
        arg3,
        arg4,
        arg5,
    };
    
    // Dispatch to unified syscall module
    let result = crate::syscall::handle_syscall(args);
    
    // Handle the result
    match result {
        crate::syscall::SysCallResult::Success(value) => {
            ctx.x[10] = value as u64; // Return value in a0
            console_println!("✅ Syscall {} completed successfully: {}", syscall_num, value);
        }
        crate::syscall::SysCallResult::Error(code) => {
            ctx.x[10] = (-code as i64) as u64; // Error code in a0 (negative)
            console_println!("❌ Syscall {} failed with error code: {}", syscall_num, code);
        }
    }
    
    // Check if a user program has exited (e.g., via sys_exit)
    if let Some(exit_code) = check_user_program_exit() {
        console_println!("🎯 User program exited with code {} - restarting shell", exit_code);
        
        // Instead of returning to user mode (which would crash), 
        // jump directly to shell_loop to restart the shell
        crate::shell_loop();
        
        // This should never be reached since shell_loop never returns
        return;
    }
    
    // Skip the ecall instruction (advance PC by 4 bytes) for all syscalls
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
                console_println!("⏰ Timer interrupt");
            }
            TrapCause::SupervisorExternalInterrupt => {
                console_println!("🔌 External interrupt");
            }
            _ => {
                console_println!("❓ Unknown interrupt: {:?}", cause);
            }
        }
    } else {
        // Handle exceptions
        console_println!("🔍 Exception occurred: cause={}, sepc=0x{:x}", ctx.scause, ctx.sepc);
        
        match cause {
            TrapCause::EnvironmentCallFromUMode => {
                console_println!("🔍 Handling user mode syscall");
                // Handle system calls from user mode - dispatch to unified syscall module
                handle_syscall(ctx);
            }
            TrapCause::EnvironmentCallFromSMode => {
                console_println!("🔍 Handling supervisor mode syscall");
                // Handle system calls from supervisor mode - dispatch to unified syscall module
                handle_syscall(ctx);
            }
            TrapCause::Breakpoint => {
                // Check if this breakpoint is from our exit stub
                if let Some(exit_code) = check_user_program_exit() {
                    console_println!("🎯 Exit stub breakpoint hit - returning to kernel with code {}", exit_code);
                    
                    // Set up return to kernel
                    ctx.x[10] = exit_code as u64; // a0 = exit code
                    
                    // Set supervisor mode
                    ctx.sstatus |= 0x00000100; // Set SPP bit for supervisor mode
                    
                    // Instead of trying to figure out the return address,
                    // let's just halt the system cleanly for now
                    console_println!("🎉 Program completed successfully with exit code: {}", exit_code);
                    console_println!("🏁 Returning to shell...");
                    
                    // For now, let's just return to a safe location
                    // We'll improve this later to properly return to the shell
                    ctx.sepc = 0x80200000; // Return to a safe kernel location
                    
                    console_println!("🔍 Setting sepc to safe kernel location: 0x{:x}", ctx.sepc);
                    
                    return;
                } else {
                    // Regular breakpoint - dump crash info
                    dump_crash_info(ctx);
                    
                    // Halt the system
                    loop {
                        unsafe {
                            asm!("wfi");
                        }
                    }
                }
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
    
    // Write back CSR values before returning
    console_println!("🔍 Writing back CSRs: sepc=0x{:x}, sstatus=0x{:x}", ctx.sepc, ctx.sstatus);
    unsafe {
        asm!(
            "csrw sepc, {}",
            "csrw sstatus, {}",
            in(reg) ctx.sepc,
            in(reg) ctx.sstatus,
        );
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

// Global flag to indicate when a user program has exited
pub static USER_PROGRAM_EXITED: Mutex<Option<i32>> = Mutex::new(None);

pub fn check_user_program_exit() -> Option<i32> {
    let mut exit_code = USER_PROGRAM_EXITED.lock();
    exit_code.take()
} 