// Process Management System Calls - Linux Compatible Numbers
// Following Linux ARM64/RISC-V syscall numbers for compatibility

use crate::{elf::{ElfLoader, ElfError}, console_println};
use super::{SysCallResult, SyscallArgs};
use crate::trap::USER_PROGRAM_EXITED;
use super::{ENOSYS, EINVAL, ENOEXEC};
use heapless::Vec;
use spin::Mutex;
use lazy_static::lazy_static;

// === PROCESS MANAGEMENT STRUCTURES ===

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ProcessState {
    Running,
    Waiting,
    Zombie,   // Exited but parent hasn't collected exit status
    Unused,
}

#[derive(Debug, Clone)]
pub struct Process {
    pub pid: i32,
    pub ppid: i32,  // Parent process ID
    pub state: ProcessState,
    pub exit_code: Option<i32>,
    pub memory_base: Option<usize>,  // Base address of process memory
    pub memory_size: Option<usize>,  // Size of allocated memory
}

impl Process {
    pub fn new() -> Self {
        Self {
            pid: 0,
            ppid: 0,
            state: ProcessState::Unused,
            exit_code: None,
            memory_base: None,
            memory_size: None,
        }
    }
    
    pub fn new_with_pid(pid: i32, ppid: i32) -> Self {
        Self {
            pid,
            ppid,
            state: ProcessState::Running,
            exit_code: None,
            memory_base: None,
            memory_size: None,
        }
    }
}

// Simple process table - support up to 64 processes
const MAX_PROCESSES: usize = 64;

pub struct ProcessManager {
    processes: Vec<Process, MAX_PROCESSES>,
    next_pid: i32,
    current_pid: i32,
}

impl ProcessManager {
    pub fn new() -> Self {
        let mut pm = Self {
            processes: Vec::new(),
            next_pid: 1,
            current_pid: 1,  // Start with init process (shell)
        };
        
        // Create init process (the shell)
        let init_process = Process::new_with_pid(1, 0);
        pm.processes.push(init_process).ok();
        
        pm
    }
    
    pub fn allocate_pid(&mut self) -> i32 {
        let pid = self.next_pid;
        self.next_pid += 1;
        pid
    }
    
    pub fn create_process(&mut self, ppid: i32) -> Option<i32> {
        if self.processes.len() >= MAX_PROCESSES {
            return None;
        }
        
        let pid = self.allocate_pid();
        let process = Process::new_with_pid(pid, ppid);
        
        self.processes.push(process).ok()?;
        Some(pid)
    }
    
    pub fn get_process(&self, pid: i32) -> Option<&Process> {
        self.processes.iter().find(|p| p.pid == pid)
    }
    
    pub fn get_process_mut(&mut self, pid: i32) -> Option<&mut Process> {
        self.processes.iter_mut().find(|p| p.pid == pid)
    }
    
    pub fn exit_process(&mut self, pid: i32, exit_code: i32) {
        if let Some(process) = self.get_process_mut(pid) {
            process.state = ProcessState::Zombie;
            process.exit_code = Some(exit_code);
            console_println!("[i] Process {} exited with code {}", pid, exit_code);
        }
    }
    
    pub fn wait_for_child(&mut self, parent_pid: i32) -> Option<(i32, i32)> {
        // Find a zombie child process
        for process in self.processes.iter_mut() {
            if process.ppid == parent_pid && process.state == ProcessState::Zombie {
                let child_pid = process.pid;
                let exit_code = process.exit_code.unwrap_or(-1);
                
                // Remove the zombie process (reap it)
                process.state = ProcessState::Unused;
                
                return Some((child_pid, exit_code));
            }
        }
        None
    }
    
    pub fn get_current_pid(&self) -> i32 {
        self.current_pid
    }
    
    pub fn set_current_pid(&mut self, pid: i32) {
        self.current_pid = pid;
    }
}

// Global process manager
lazy_static! {
    pub static ref PROCESS_MANAGER: Mutex<ProcessManager> = Mutex::new(ProcessManager::new());
}

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

// Additional syscalls
pub const SYS_WAIT4: usize = 260;       // Linux: wait4

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
        SYS_WAITID => sys_waitid(args.arg0 as i32, args.arg1 as i32, args.arg2 as *mut i32, args.arg3 as i32),
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
    console_println!("[i] SYS_EXIT: Process exiting with code {}", exit_code);
    
    // Update process state in process manager
    {
        let mut pm = PROCESS_MANAGER.lock();
        let current_pid = pm.get_current_pid();
        pm.exit_process(current_pid, exit_code as i32);
        
        // If this is not the init process (PID 1), return to parent
        if current_pid != 1 {
            // Find the parent process and make it current
            if let Some(process) = pm.get_process(current_pid) {
                let parent_pid = process.ppid;
                pm.set_current_pid(parent_pid);
                console_println!("[i] Returning control to parent process {}", parent_pid);
            }
        }
    }
    
    // Set the global exit flag so the trap handler knows to jump to shell_loop
    // instead of returning to user mode
    {
        let mut exit_flag = USER_PROGRAM_EXITED.lock();
        *exit_flag = Some(exit_code as i32);
    }
    
    console_println!("[i] Process cleanup complete, returning to shell...");
    SysCallResult::Success(exit_code)
}

fn sys_exit_group(status: i32) -> SysCallResult {
    console_println!("[i] Process group exited with status: {}", status);
    // For now, treat this the same as regular exit
    sys_exit(status as isize)
}

fn sys_fork() -> SysCallResult {
    console_println!("[i] SYS_FORK: Creating child process");
    
    let mut pm = PROCESS_MANAGER.lock();
    let current_pid = pm.get_current_pid();
    
    // Create a new child process
    match pm.create_process(current_pid) {
        Some(child_pid) => {
            console_println!("[o] Fork successful: parent={}, child={}", current_pid, child_pid);
            
            // In a real fork, we would:
            // 1. Copy the parent's memory space to the child
            // 2. Set up child's execution context
            // 3. Return 0 to child, child_pid to parent
            
            // For now, we'll simulate this by returning the child PID to the parent
            // The child process will be created when execve is called
            SysCallResult::Success(child_pid as isize)
        }
        None => {
            console_println!("[x] Fork failed: too many processes");
            SysCallResult::Error(crate::syscall::ENOMEM)
        }
    }
}

fn sys_clone() -> SysCallResult {
    console_println!("[i] SYS_CLONE: Redirecting to fork");
    // For simplicity, treat clone as fork
    sys_fork()
}

fn sys_execve() -> SysCallResult {
    console_println!("[i] SYS_EXECVE: Replacing process image");
    
    // For now, we'll implement a simple version that works with our ELF loader
    // In a real implementation, we would:
    // 1. Parse the filename and arguments
    // 2. Load the new ELF binary
    // 3. Replace the current process's memory space
    // 4. Jump to the new program's entry point
    
    console_println!("[!] EXECVE: Current implementation uses direct ELF execution");
    console_println!("[!] Use the existing ELF execution system instead");
    
    // Return success for now - real implementation would not return
    SysCallResult::Success(0)
}

fn sys_waitid(_which: i32, _pid: i32, _status: *mut i32, _options: i32) -> SysCallResult {
    // TODO: Implement wait for child process
    SysCallResult::Error(ENOSYS)
}

fn sys_getpid() -> SysCallResult {
    let pm = PROCESS_MANAGER.lock();
    let current_pid = pm.get_current_pid();
    console_println!("[i] SYS_GETPID: returning PID {}", current_pid);
    SysCallResult::Success(current_pid as isize)
}

fn sys_getppid() -> SysCallResult {
    let pm = PROCESS_MANAGER.lock();
    let current_pid = pm.get_current_pid();
    
    if let Some(process) = pm.get_process(current_pid) {
        console_println!("[i] SYS_GETPPID: returning PPID {}", process.ppid);
        SysCallResult::Success(process.ppid as isize)
    } else {
        console_println!("[x] SYS_GETPPID: current process not found");
        SysCallResult::Success(0) // Return init as default parent
    }
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
    console_println!("[x] Kill not implemented");
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
            console_println!("[o] ELF loaded successfully with {} segments", loaded_elf.segments.len());
            console_println!("[i] Entry point: 0x{:x}", loaded_elf.entry_point);
            
            // Display segment information
            for (i, segment) in loaded_elf.segments.iter().enumerate() {
                let perms = crate::elf::segment_permissions(segment.flags);
                console_println!("[i] Segment {}: 0x{:x} ({} bytes) [{}]", 
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
                ElfError::ExecutionError => "Error executing ELF binary",
                ElfError::MemoryAllocationFailed => "Memory allocation failed",
                ElfError::InvalidEntryPoint => "Invalid entry point",
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
            console_println!("[o] ELF loaded, attempting execution...");
            
            // Execute the loaded ELF
            match crate::elf::execute_elf(&loaded_elf) {
                Ok(()) => {
                    console_println!("[o] ELF execution completed successfully!");
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
                crate::elf::ElfError::ExecutionError => "Error executing ELF binary",
                crate::elf::ElfError::MemoryAllocationFailed => "Memory allocation failed",
                crate::elf::ElfError::InvalidEntryPoint => "Invalid entry point",
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
                ElfError::ExecutionError => "Error executing ELF binary",
                ElfError::MemoryAllocationFailed => "Memory allocation failed",
                ElfError::InvalidEntryPoint => "Invalid entry point",
            };
            SysCallResult::Error(ENOEXEC)
        }
    }
}

fn sys_wait4(pid: i32, status: *mut i32, _options: i32, _rusage: *mut u8) -> SysCallResult {
    console_println!("[i] SYS_WAIT4: Waiting for child process (pid={})", pid);
    
    let mut pm = PROCESS_MANAGER.lock();
    let current_pid = pm.get_current_pid();
    
    // Wait for any child if pid == -1, or specific child if pid > 0
    match pm.wait_for_child(current_pid) {
        Some((child_pid, exit_code)) => {
            console_println!("[o] Child process {} exited with code {}", child_pid, exit_code);
            
            // Write exit status to the status pointer if provided
            if !status.is_null() {
                unsafe {
                    *status = exit_code;
                }
            }
            
            // Return the PID of the child that exited
            SysCallResult::Success(child_pid as isize)
        }
        None => {
            // No zombie children available
            console_println!("[i] No zombie children to wait for");
            SysCallResult::Error(crate::syscall::ECHILD) // No child processes
        }
    }
}

fn sys_setuid(_uid: u32) -> SysCallResult {
    console_println!("[x] Setuid not implemented");
    SysCallResult::Error(ENOSYS)
}

fn sys_setgid(_gid: u32) -> SysCallResult {
    console_println!("[x] Setgid not implemented");
    SysCallResult::Error(ENOSYS)
}

fn sys_geteuid() -> SysCallResult {
    console_println!("[x] Geteuid not implemented");
    SysCallResult::Success(0) // Return root
}

fn sys_getegid() -> SysCallResult {
    console_println!("[x] Getegid not implemented");
    SysCallResult::Success(0) // Return root
}

fn sys_setsid() -> SysCallResult {
    console_println!("[x] Setsid not implemented");
    SysCallResult::Error(ENOSYS)
}

fn sys_getpgid(_pid: i32) -> SysCallResult {
    console_println!("[x] Getpgid not implemented");
    SysCallResult::Success(1) // Return process group 1
}

fn sys_setpgid(_pid: i32, _pgid: i32) -> SysCallResult {
    console_println!("[x] Setpgid not implemented");
    SysCallResult::Error(ENOSYS)
}

fn sys_getpgrp() -> SysCallResult {
    console_println!("[x] Getpgrp not implemented");
    SysCallResult::Success(1) // Return process group 1
}

fn sys_sched_yield() -> SysCallResult {
    console_println!("[x] Sched_yield not implemented");
    SysCallResult::Success(0)
}

fn sys_nanosleep(_req: *const u8, _rem: *mut u8) -> SysCallResult {
    console_println!("[x] Nanosleep not implemented");
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