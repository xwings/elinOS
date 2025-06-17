// Process Management System Calls - Linux Compatible Numbers
// Following Linux ARM64/RISC-V syscall numbers for compatibility

use crate::{UART, elf::{ElfLoader, ElfError}, console_println};
use core::fmt::Write;
use super::{SysCallResult, SyscallArgs};
use crate::trap::USER_PROGRAM_EXITED;
use super::{ENOSYS, EINVAL, ENOEXEC};

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
pub fn handle_process_syscall(syscall_num: usize, args: &SyscallArgs) -> SysCallResult {
    match syscall_num {
        SYS_EXIT => sys_exit(args.arg0 as isize),
        SYS_EXIT_GROUP => sys_exit_group(args.arg0 as i32),
        SYS_GETPID => sys_getpid(),
        SYS_GETPPID => sys_getppid(),
        SYS_FORK => sys_fork(),
        SYS_CLONE => sys_clone(),
        SYS_EXECVE => sys_execve(),
        SYS_WAIT4 => sys_wait4(args.arg0 as i32, args.arg1 as *mut i32, args.arg2 as i32, args.arg3 as *mut u8),
        SYS_KILL => sys_kill(args.arg0 as i32, args.arg1 as i32),
        SYS_GETUID => sys_getuid(),
        SYS_GETGID => sys_getgid(),
        SYS_SETUID => sys_setuid(args.arg0 as u32),
        SYS_SETGID => sys_setgid(args.arg0 as u32),
        SYS_GETEUID => sys_geteuid(),
        SYS_GETEGID => sys_getegid(),
        SYS_SETSID => sys_setsid(),
        SYS_GETPGID => sys_getpgid(args.arg0 as i32),
        SYS_SETPGID => sys_setpgid(args.arg0 as i32, args.arg1 as i32),
        SYS_GETPGRP => sys_getpgrp(),
        SYS_SCHED_YIELD => sys_sched_yield(),
        SYS_NANOSLEEP => sys_nanosleep(args.arg0 as *const u8, args.arg1 as *mut u8),
        SYS_ALARM => sys_alarm(args.arg0 as u32),
        SYS_PAUSE => sys_pause(),
        SYS_PRCTL => sys_prctl(args.arg0 as i32, args.arg1 as u64, args.arg2 as u64, args.arg3 as u64, args.arg4 as u64),
        _ => SysCallResult::Error(ENOSYS), // Function not implemented
    }
}

// === SYSTEM CALL IMPLEMENTATIONS ===

pub fn sys_exit(exit_code: isize) -> SysCallResult {
    // console_println!("ℹ️ SYS_EXIT: Program exiting with code {}", exit_code);
    // console_println!("✅ Program completed successfully with exit code: {}", exit_code);
    // console_println!("ℹ️ Returning to shell...");
    
    // Set the global exit flag so the trap handler knows to jump to shell_loop
    // instead of returning to user mode
    {
        let mut exit_flag = USER_PROGRAM_EXITED.lock();
        *exit_flag = Some(exit_code as i32);
    }
    
    // Return success - the trap handler will handle the actual transition to shell_loop
    SysCallResult::Success(exit_code)
}

fn sys_exit_group(status: i32) -> SysCallResult {
    console_println!("ℹ️ Process group exited with status: {}", status);
    // For now, treat this the same as regular exit
    sys_exit(status as isize)
}

fn sys_fork() -> SysCallResult {
    console_println!("❌ Fork not implemented");
    SysCallResult::Error(crate::syscall::ENOSYS) // Not implemented
}

fn sys_clone() -> SysCallResult {
    console_println!("❌ Clone not implemented");
    SysCallResult::Error(crate::syscall::ENOSYS) // Not implemented
}

fn sys_execve() -> SysCallResult {
    console_println!("❌ Execve not implemented");
    SysCallResult::Error(crate::syscall::ENOSYS) // Not implemented
}

fn sys_waitid(_which: i32, _pid: i32, _status: *mut i32, _options: i32) -> SysCallResult {
    // TODO: Implement wait for child process
    SysCallResult::Error(ENOSYS)
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
    console_println!("❌ Kill not implemented");
    SysCallResult::Error(ENOSYS)
}

fn sys_tkill(_tid: i32, _sig: i32) -> SysCallResult {
    // TODO: Implement signal sending to thread
    SysCallResult::Error(ENOSYS)
}

fn sys_tgkill(_tgid: i32, _tid: i32, _sig: i32) -> SysCallResult {
    // TODO: Implement signal sending to thread in thread group
    SysCallResult::Error(ENOSYS)
}

fn sys_rt_sigaction(_signum: i32, _act: usize, _oldact: usize) -> SysCallResult {
    // TODO: Implement signal handler registration
    SysCallResult::Error(ENOSYS)
}

// === ELF LOADING SYSTEM CALLS ===

pub fn sys_load_elf(data_ptr: *const u8, size: usize) -> SysCallResult {
    if data_ptr.is_null() || size == 0 {
        return SysCallResult::Error(EINVAL);
    }

    // Create slice from raw pointer (unsafe but necessary for kernel)
    let elf_data = unsafe {
        core::slice::from_raw_parts(data_ptr, size)
    };

    let loader = ElfLoader::new();
    
    match loader.load_elf(elf_data) {
        Ok(loaded_elf) => {
            console_println!("✅ ELF loaded successfully with {} segments", loaded_elf.segments.len());
            console_println!("ℹ️ Entry point: 0x{:x}", loaded_elf.entry_point);
            
            // Display segment information
            for (i, segment) in loaded_elf.segments.iter().enumerate() {
                let perms = crate::elf::segment_permissions(segment.flags);
                console_println!("ℹ️ Segment {}: 0x{:x} ({} bytes) [{}]", 
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
            SysCallResult::Error(ENOEXEC)
        }
    }
}

pub fn sys_exec_elf(data_ptr: *const u8, size: usize) -> SysCallResult {
    if data_ptr.is_null() || size == 0 {
        return SysCallResult::Error(EINVAL);
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
                    console_println!("✅ ELF execution completed successfully!");
                    SysCallResult::Success(loaded_elf.entry_point as isize)
                }
                Err(err) => {
                    let error_msg = match err {
                        crate::elf::ElfError::LoadError => "Failed to execute ELF binary",
                        _ => "ELF execution error",
                    };
                    SysCallResult::Error(ENOEXEC)
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
            SysCallResult::Error(ENOEXEC)
        }
    }
}

pub fn sys_elf_info(data_ptr: *const u8, size: usize) -> SysCallResult {
    if data_ptr.is_null() || size == 0 {
        return SysCallResult::Error(EINVAL);
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
            SysCallResult::Error(ENOEXEC)
        }
    }
}

fn sys_wait4(_pid: i32, _status: *mut i32, _options: i32, _rusage: *mut u8) -> SysCallResult {
    console_println!("❌ Wait4 not implemented");
    SysCallResult::Error(ENOSYS)
}

fn sys_setuid(_uid: u32) -> SysCallResult {
    console_println!("❌ Setuid not implemented");
    SysCallResult::Error(ENOSYS)
}

fn sys_setgid(_gid: u32) -> SysCallResult {
    console_println!("❌ Setgid not implemented");
    SysCallResult::Error(ENOSYS)
}

fn sys_geteuid() -> SysCallResult {
    console_println!("❌ Geteuid not implemented");
    SysCallResult::Success(0) // Return root
}

fn sys_getegid() -> SysCallResult {
    console_println!("❌ Getegid not implemented");
    SysCallResult::Success(0) // Return root
}

fn sys_setsid() -> SysCallResult {
    console_println!("❌ Setsid not implemented");
    SysCallResult::Error(ENOSYS)
}

fn sys_getpgid(_pid: i32) -> SysCallResult {
    console_println!("❌ Getpgid not implemented");
    SysCallResult::Success(1) // Return process group 1
}

fn sys_setpgid(_pid: i32, _pgid: i32) -> SysCallResult {
    console_println!("❌ Setpgid not implemented");
    SysCallResult::Error(ENOSYS)
}

fn sys_getpgrp() -> SysCallResult {
    console_println!("❌ Getpgrp not implemented");
    SysCallResult::Success(1) // Return process group 1
}

fn sys_sched_yield() -> SysCallResult {
    console_println!("❌ Sched_yield not implemented");
    SysCallResult::Success(0)
}

fn sys_nanosleep(_req: *const u8, _rem: *mut u8) -> SysCallResult {
    console_println!("❌ Nanosleep not implemented");
    SysCallResult::Error(ENOSYS)
}

fn sys_alarm(_seconds: u32) -> SysCallResult {
    console_println!("Alarm not implemented");
    SysCallResult::Success(0)
}

fn sys_pause() -> SysCallResult {
    console_println!("Pause not implemented");
    SysCallResult::Error(ENOSYS)
}

fn sys_prctl(_option: i32, _arg2: u64, _arg3: u64, _arg4: u64, _arg5: u64) -> SysCallResult {
    console_println!("Prctl not implemented");
    SysCallResult::Error(ENOSYS)
} 