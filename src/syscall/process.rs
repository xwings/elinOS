// Process Management System Calls (121-170)
// Handles process operations like exit, fork, execve, kill, etc.

use crate::{UART, elf::{ElfLoader, ElfError}};
use core::fmt::Write;
use super::SysCallResult;

// === PROCESS MANAGEMENT SYSTEM CALL CONSTANTS (121-170) ===
pub const SYS_EXIT: usize = 121;
pub const SYS_FORK: usize = 122;
pub const SYS_EXECVE: usize = 123;
pub const SYS_WAIT: usize = 124;
pub const SYS_WAITPID: usize = 125;
pub const SYS_GETPID: usize = 126;
pub const SYS_GETPPID: usize = 127;
pub const SYS_KILL: usize = 128;
pub const SYS_SIGNAL: usize = 129;
// ELF loading syscalls
pub const SYS_LOAD_ELF: usize = 130;
pub const SYS_EXEC_ELF: usize = 131;
pub const SYS_ELF_INFO: usize = 132;
// Reserved for future process management: 133-170

// Handle process management system calls
pub fn handle_process_syscall(
    syscall_num: usize,
    arg0: usize,
    arg1: usize,
    arg2: usize,
    _arg3: usize,
) -> SysCallResult {
    match syscall_num {
        SYS_EXIT => sys_exit(arg0 as i32),
        SYS_FORK => sys_fork(),
        SYS_EXECVE => sys_execve(arg0 as *const u8, arg1 as *const *const u8, arg2 as *const *const u8),
        SYS_WAIT => sys_wait(arg0 as *mut i32),
        SYS_WAITPID => sys_waitpid(arg0 as i32, arg1 as *mut i32, arg2 as i32),
        SYS_GETPID => sys_getpid(),
        SYS_GETPPID => sys_getppid(),
        SYS_KILL => sys_kill(arg0 as i32, arg1 as i32),
        SYS_SIGNAL => sys_signal(arg0 as i32, arg1),
        SYS_LOAD_ELF => sys_load_elf(arg0 as *const u8, arg1),
        SYS_EXEC_ELF => sys_exec_elf(arg0 as *const u8, arg1),
        SYS_ELF_INFO => sys_elf_info(arg0 as *const u8, arg1),
        _ => SysCallResult::Error("Unknown process management system call"),
    }
}

// === SYSTEM CALL IMPLEMENTATIONS ===

fn sys_exit(status: i32) -> SysCallResult {
    let mut uart = UART.lock();
    let _ = writeln!(uart, "Process exited with status: {}", status);
    // In a real OS, this would terminate the process
    // For now, we just return success
    SysCallResult::Success(status as isize)
}

fn sys_fork() -> SysCallResult {
    // TODO: Implement process forking
    SysCallResult::Error("fork not implemented")
}

fn sys_execve(_filename: *const u8, _argv: *const *const u8, _envp: *const *const u8) -> SysCallResult {
    // TODO: Implement program execution
    // This could use the ELF loader once we have file system support
    SysCallResult::Error("execve not implemented - use load_elf or exec_elf for ELF binaries")
}

fn sys_wait(_status: *mut i32) -> SysCallResult {
    // TODO: Implement wait for child process
    SysCallResult::Error("wait not implemented")
}

fn sys_waitpid(_pid: i32, _status: *mut i32, _options: i32) -> SysCallResult {
    // TODO: Implement wait for specific child process
    SysCallResult::Error("waitpid not implemented")
}

fn sys_getpid() -> SysCallResult {
    // TODO: Return actual process ID
    // For now, return a fake PID
    SysCallResult::Success(1)
}

fn sys_getppid() -> SysCallResult {
    // TODO: Return actual parent process ID
    // For now, return a fake PPID
    SysCallResult::Success(0)
}

fn sys_kill(_pid: i32, _sig: i32) -> SysCallResult {
    // TODO: Implement signal sending to process
    SysCallResult::Error("kill not implemented")
}

fn sys_signal(_signum: i32, _handler: usize) -> SysCallResult {
    // TODO: Implement signal handler registration
    SysCallResult::Error("signal not implemented")
}

// === ELF LOADING SYSTEM CALLS ===

fn sys_load_elf(data_ptr: *const u8, size: usize) -> SysCallResult {
    if data_ptr.is_null() || size == 0 {
        return SysCallResult::Error("Invalid ELF data pointer or size");
    }

    // Create slice from raw pointer (unsafe but necessary for kernel)
    let elf_data = unsafe {
        core::slice::from_raw_parts(data_ptr, size)
    };

    let loader = ElfLoader::new();
    
    match loader.load_elf(elf_data) {
        Ok(loaded_elf) => {
            let mut uart = UART.lock();
            let _ = writeln!(uart, "ELF loaded successfully with {} segments", loaded_elf.segments.len());
            let _ = writeln!(uart, "Entry point: 0x{:x}", loaded_elf.entry_point);
            
            // Display segment information
            for (i, segment) in loaded_elf.segments.iter().enumerate() {
                let perms = crate::elf::segment_permissions(segment.flags);
                let _ = writeln!(uart, "  Segment {}: 0x{:x} ({} bytes) [{}]", 
                    i, segment.vaddr, segment.memsz, perms);
            }
            drop(uart);
            
            // Return entry point as success value
            SysCallResult::Success(loaded_elf.entry_point as isize)
        }
        Err(err) => {
            let error_msg = match err {
                ElfError::InvalidMagic => "Invalid ELF magic number",
                ElfError::UnsupportedClass => "Unsupported ELF class (need ELF64)",
                ElfError::UnsupportedEndian => "Unsupported endianness (need little-endian)",
                ElfError::UnsupportedMachine => "Unsupported machine type (need RISC-V)",
                ElfError::UnsupportedType => "Unsupported ELF type (need executable or shared object)",
                ElfError::InvalidHeader => "Invalid or corrupted ELF header",
                ElfError::LoadError => "Error loading ELF segments",
            };
            SysCallResult::Error(error_msg)
        }
    }
}

fn sys_exec_elf(data_ptr: *const u8, size: usize) -> SysCallResult {
    // First load the ELF
    match sys_load_elf(data_ptr, size) {
        SysCallResult::Success(entry_point) => {
            let mut uart = UART.lock();
            let _ = writeln!(uart, "Would execute ELF at entry point: 0x{:x}", entry_point);
            let _ = writeln!(uart, "NOTE: Actual execution requires virtual memory and process isolation");
            drop(uart);
            
            // TODO: In a real OS, we would:
            // 1. Create a new process context
            // 2. Set up virtual memory mappings
            // 3. Copy segments to the new address space
            // 4. Set up stack and heap
            // 5. Jump to the entry point
            
            SysCallResult::Success(entry_point)
        }
        error => error,
    }
}

fn sys_elf_info(data_ptr: *const u8, size: usize) -> SysCallResult {
    if data_ptr.is_null() || size == 0 {
        return SysCallResult::Error("Invalid ELF data pointer or size");
    }

    let elf_data = unsafe {
        core::slice::from_raw_parts(data_ptr, size)
    };

    let loader = ElfLoader::new();
    
    match loader.display_elf_info(elf_data) {
        Ok(()) => SysCallResult::Success(0),
        Err(err) => {
            let error_msg = match err {
                ElfError::InvalidMagic => "Invalid ELF magic number",
                ElfError::UnsupportedClass => "Unsupported ELF class",
                ElfError::UnsupportedEndian => "Unsupported endianness",
                ElfError::UnsupportedMachine => "Unsupported machine type",
                ElfError::UnsupportedType => "Unsupported ELF type",
                ElfError::InvalidHeader => "Invalid ELF header",
                ElfError::LoadError => "ELF load error",
            };
            SysCallResult::Error(error_msg)
        }
    }
} 