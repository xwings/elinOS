// Process Management System Calls - Linux Compatible Numbers
// Following Linux ARM64/RISC-V syscall numbers for compatibility

use crate::{UART, elf::{ElfLoader, ElfError}, console_println};
use core::fmt::Write;
use super::{SysCallResult, SyscallArgs};

// === LINUX COMPATIBLE PROCESS MANAGEMENT SYSTEM CALL CONSTANTS ===
pub const SYS_EXIT: usize = 93;        // Linux: exit
pub const SYS_EXIT_GROUP: usize = 94;  // Linux: exit_group
pub const SYS_WAITID: usize = 95;      // Linux: waitid
pub const SYS_SET_TID_ADDRESS: usize = 96; // Linux: set_tid_address
pub const SYS_UNSHARE: usize = 97;     // Linux: unshare
pub const SYS_FUTEX: usize = 98;       // Linux: futex
pub const SYS_SET_ROBUST_LIST: usize = 99;  // Linux: set_robust_list
pub const SYS_GET_ROBUST_LIST: usize = 100; // Linux: get_robust_list
pub const SYS_NANOSLEEP: usize = 101;  // Linux: nanosleep

pub const SYS_GETITIMER: usize = 102;  // Linux: getitimer
pub const SYS_SETITIMER: usize = 103;  // Linux: setitimer
pub const SYS_KEXEC_LOAD: usize = 104; // Linux: kexec_load
pub const SYS_INIT_MODULE: usize = 105; // Linux: init_module
pub const SYS_DELETE_MODULE: usize = 106; // Linux: delete_module

pub const SYS_KILL: usize = 129;       // Linux: kill
pub const SYS_TKILL: usize = 130;      // Linux: tkill
pub const SYS_TGKILL: usize = 131;     // Linux: tgkill

pub const SYS_RT_SIGSUSPEND: usize = 133;   // Linux: rt_sigsuspend
pub const SYS_RT_SIGACTION: usize = 134;    // Linux: rt_sigaction
pub const SYS_RT_SIGPROCMASK: usize = 135;  // Linux: rt_sigprocmask
pub const SYS_RT_SIGPENDING: usize = 136;   // Linux: rt_sigpending
pub const SYS_RT_SIGTIMEDWAIT: usize = 137; // Linux: rt_sigtimedwait
pub const SYS_RT_SIGQUEUEINFO: usize = 138; // Linux: rt_sigqueueinfo
pub const SYS_RT_SIGRETURN: usize = 139;    // Linux: rt_sigreturn

pub const SYS_SETPRIORITY: usize = 140; // Linux: setpriority
pub const SYS_GETPRIORITY: usize = 141; // Linux: getpriority
pub const SYS_REBOOT: usize = 142;      // Linux: reboot

pub const SYS_SETREGID: usize = 143;    // Linux: setregid
pub const SYS_SETGID: usize = 144;      // Linux: setgid
pub const SYS_SETREUID: usize = 145;    // Linux: setreuid
pub const SYS_SETUID: usize = 146;      // Linux: setuid
pub const SYS_SETRESUID: usize = 147;   // Linux: setresuid
pub const SYS_GETRESUID: usize = 148;   // Linux: getresuid
pub const SYS_SETRESGID: usize = 149;   // Linux: setresgid
pub const SYS_GETRESGID: usize = 150;   // Linux: getresgid
pub const SYS_SETFSUID: usize = 151;    // Linux: setfsuid
pub const SYS_SETFSGID: usize = 152;    // Linux: setfsgid
pub const SYS_TIMES: usize = 153;       // Linux: times
pub const SYS_SETPGID: usize = 154;     // Linux: setpgid
pub const SYS_GETPGID: usize = 155;     // Linux: getpgid
pub const SYS_GETSID: usize = 156;      // Linux: getsid
pub const SYS_SETSID: usize = 157;      // Linux: setsid
pub const SYS_GETGROUPS: usize = 158;   // Linux: getgroups
pub const SYS_SETGROUPS: usize = 159;   // Linux: setgroups

pub const SYS_GETPID: usize = 172;      // Linux: getpid
pub const SYS_GETPPID: usize = 173;     // Linux: getppid
pub const SYS_GETUID: usize = 174;      // Linux: getuid
pub const SYS_GETEUID: usize = 175;     // Linux: geteuid
pub const SYS_GETGID: usize = 176;      // Linux: getgid
pub const SYS_GETEGID: usize = 177;     // Linux: getegid
pub const SYS_GETTID: usize = 178;      // Linux: gettid

pub const SYS_CLONE: usize = 220;       // Linux: clone
pub const SYS_EXECVE: usize = 221;      // Linux: execve

// Legacy syscall aliases for backwards compatibility
pub const SYS_FORK: usize = SYS_CLONE;  // Map fork to clone
pub const SYS_WAIT: usize = SYS_WAITID; // Map wait to waitid
pub const SYS_WAITPID: usize = SYS_WAITID; // Map waitpid to waitid
pub const SYS_SIGNAL: usize = SYS_RT_SIGACTION; // Map signal to rt_sigaction

// ELF loading syscalls - elinOS specific (keeping high numbers to avoid conflicts)
pub const SYS_LOAD_ELF: usize = 900;    // elinOS: load ELF binary
pub const SYS_EXEC_ELF: usize = 901;    // elinOS: execute ELF binary
pub const SYS_ELF_INFO: usize = 902;    // elinOS: ELF binary info

// Linux compatible process management syscall handler
pub fn handle_process_syscall(args: &SyscallArgs) -> SysCallResult {
    match args.syscall_num {
        SYS_EXIT => sys_exit(args.arg0_as_i32()),
        SYS_EXIT_GROUP => sys_exit_group(args.arg0_as_i32()),
        SYS_CLONE => sys_clone(args.arg0, args.arg1, args.arg2, args.arg3, args.arg4),
        SYS_EXECVE => sys_execve(args.arg0_as_ptr::<u8>(), args.arg1_as_ptr::<*const u8>(), args.arg2_as_ptr::<*const u8>()),
        SYS_WAITID => sys_waitid(args.arg0_as_i32(), args.arg1_as_i32(), args.arg2_as_mut_ptr::<i32>(), args.arg3_as_i32()),
        SYS_GETPID => sys_getpid(),
        SYS_GETPPID => sys_getppid(),
        SYS_GETUID => sys_getuid(),
        SYS_GETGID => sys_getgid(),
        SYS_GETTID => sys_gettid(),
        SYS_KILL => sys_kill(args.arg0_as_i32(), args.arg1_as_i32()),
        SYS_TKILL => sys_tkill(args.arg0_as_i32(), args.arg1_as_i32()),
        SYS_TGKILL => sys_tgkill(args.arg0_as_i32(), args.arg1_as_i32(), args.arg2_as_i32()),
        SYS_RT_SIGACTION => sys_rt_sigaction(args.arg0_as_i32(), args.arg1, args.arg2),
        
        // elinOS-specific ELF syscalls
        SYS_LOAD_ELF => sys_load_elf(args.arg0_as_ptr::<u8>(), args.arg1),
        SYS_EXEC_ELF => sys_exec_elf(args.arg0_as_ptr::<u8>(), args.arg1),
        SYS_ELF_INFO => sys_elf_info(args.arg0_as_ptr::<u8>(), args.arg1),
        
        _ => SysCallResult::Error("Unknown process management system call"),
    }
}

// === SYSTEM CALL IMPLEMENTATIONS ===

fn sys_exit(status: i32) -> SysCallResult {
    console_println!("Process exited with status: {}", status);
    // In a real OS, this would terminate the process
    // For now, we just return success
    SysCallResult::Success(status as isize)
}

fn sys_exit_group(status: i32) -> SysCallResult {
    console_println!("Process group exited with status: {}", status);
    // In a real OS, this would terminate the entire process group
    SysCallResult::Success(status as isize)
}

fn sys_clone(_flags: usize, _stack: usize, _parent_tid: usize, _child_tid: usize, _tls: usize) -> SysCallResult {
    // TODO: Implement process cloning/forking
    SysCallResult::Error("clone not implemented")
}

fn sys_execve(_filename: *const u8, _argv: *const *const u8, _envp: *const *const u8) -> SysCallResult {
    // TODO: Implement program execution
    // This could use the ELF loader once we have file system support
    SysCallResult::Error("execve not implemented - use load_elf or exec_elf for ELF binaries")
}

fn sys_waitid(_which: i32, _pid: i32, _status: *mut i32, _options: i32) -> SysCallResult {
    // TODO: Implement wait for child process
    SysCallResult::Error("waitid not implemented")
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

fn sys_getuid() -> SysCallResult {
    // TODO: Return actual user ID
    // For now, return root (0)
    SysCallResult::Success(0)
}

fn sys_getgid() -> SysCallResult {
    // TODO: Return actual group ID
    // For now, return root (0)
    SysCallResult::Success(0)
}

fn sys_gettid() -> SysCallResult {
    // TODO: Return actual thread ID
    // For now, return same as PID
    SysCallResult::Success(1)
}

fn sys_kill(_pid: i32, _sig: i32) -> SysCallResult {
    // TODO: Implement signal sending to process
    SysCallResult::Error("kill not implemented")
}

fn sys_tkill(_tid: i32, _sig: i32) -> SysCallResult {
    // TODO: Implement signal sending to thread
    SysCallResult::Error("tkill not implemented")
}

fn sys_tgkill(_tgid: i32, _tid: i32, _sig: i32) -> SysCallResult {
    // TODO: Implement signal sending to thread in thread group
    SysCallResult::Error("tgkill not implemented")
}

fn sys_rt_sigaction(_signum: i32, _act: usize, _oldact: usize) -> SysCallResult {
    // TODO: Implement signal handler registration
    SysCallResult::Error("rt_sigaction not implemented")
}

// === ELF LOADING SYSTEM CALLS ===

pub fn sys_load_elf(data_ptr: *const u8, size: usize) -> SysCallResult {
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
            console_println!("ELF loaded successfully with {} segments", loaded_elf.segments.len());
            console_println!("Entry point: 0x{:x}", loaded_elf.entry_point);
            
            // Display segment information
            for (i, segment) in loaded_elf.segments.iter().enumerate() {
                let perms = crate::elf::segment_permissions(segment.flags);
                console_println!("  Segment {}: 0x{:x} ({} bytes) [{}]", 
                    i, segment.vaddr, segment.memsz, perms);
            }
            
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

pub fn sys_exec_elf(data_ptr: *const u8, size: usize) -> SysCallResult {
    if data_ptr.is_null() || size == 0 {
        return SysCallResult::Error("Invalid ELF data pointer or size");
    }

    // Create slice from raw pointer (unsafe but necessary for kernel)
    let elf_data = unsafe {
        core::slice::from_raw_parts(data_ptr, size)
    };

    let loader = crate::elf::ElfLoader::new();
    
    // Load the ELF binary
    match loader.load_elf(elf_data) {
        Ok(loaded_elf) => {
            console_println!("✅ ELF loaded, attempting execution...");
            
            // Execute the loaded ELF
            match loader.execute_elf(&loaded_elf) {
                Ok(()) => {
                    console_println!("🎉 ELF execution completed successfully!");
                    SysCallResult::Success(loaded_elf.entry_point as isize)
                }
                Err(err) => {
                    let error_msg = match err {
                        crate::elf::ElfError::LoadError => "Failed to execute ELF binary",
                        _ => "ELF execution error",
                    };
                    SysCallResult::Error(error_msg)
                }
            }
        }
        Err(err) => {
            let error_msg = match err {
                crate::elf::ElfError::InvalidMagic => "Invalid ELF magic number",
                crate::elf::ElfError::UnsupportedClass => "Unsupported ELF class (need ELF64)",
                crate::elf::ElfError::UnsupportedEndian => "Unsupported endianness (need little-endian)",
                crate::elf::ElfError::UnsupportedMachine => "Unsupported machine type (need RISC-V)",
                crate::elf::ElfError::UnsupportedType => "Unsupported ELF type (need executable or shared object)",
                crate::elf::ElfError::InvalidHeader => "Invalid or corrupted ELF header",
                crate::elf::ElfError::LoadError => "Error loading ELF segments",
            };
            SysCallResult::Error(error_msg)
        }
    }
}

pub fn sys_elf_info(data_ptr: *const u8, size: usize) -> SysCallResult {
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