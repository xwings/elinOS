// Device and I/O Management System Calls - Linux Compatible Numbers  
// Following Linux ARM64/RISC-V syscall numbers for compatibility

use super::{SysCallResult, SyscallArgs};
use crate::{console_println, console_print};
use spin::Mutex;
use heapless::{Vec, String};
use lazy_static::lazy_static;

// === TTY/PTY CONSTANTS ===

// Terminal I/O control constants
pub const TCGETS: usize = 0x5401;
pub const TCSETS: usize = 0x5402;
pub const TCSETSW: usize = 0x5403;
pub const TCSETSF: usize = 0x5404;
pub const TCGETA: usize = 0x5405;
pub const TCSETA: usize = 0x5406;
pub const TCSETAW: usize = 0x5407;
pub const TCSETAF: usize = 0x5408;
pub const TCSBRK: usize = 0x5409;
pub const TCXONC: usize = 0x540A;
pub const TCFLSH: usize = 0x540B;
pub const TIOCEXCL: usize = 0x540C;
pub const TIOCNXCL: usize = 0x540D;
pub const TIOCSCTTY: usize = 0x540E;
pub const TIOCGPGRP: usize = 0x540F;
pub const TIOCSPGRP: usize = 0x5410;
pub const TIOCOUTQ: usize = 0x5411;
pub const TIOCSTI: usize = 0x5412;
pub const TIOCGWINSZ: usize = 0x5413;
pub const TIOCSWINSZ: usize = 0x5414;
pub const TIOCMGET: usize = 0x5415;
pub const TIOCMBIS: usize = 0x5416;
pub const TIOCMBIC: usize = 0x5417;
pub const TIOCMSET: usize = 0x5418;
pub const TIOCGSOFTCAR: usize = 0x5419;
pub const TIOCSSOFTCAR: usize = 0x541A;
pub const FIONREAD: usize = 0x541B;
pub const TIOCINQ: usize = FIONREAD;
pub const TIOCLINUX: usize = 0x541C;
pub const TIOCCONS: usize = 0x541D;
pub const TIOCGSERIAL: usize = 0x541E;
pub const TIOCSSERIAL: usize = 0x541F;
pub const TIOCPKT: usize = 0x5420;

// Terminal flags
pub const IGNBRK: u32 = 0x0001;
pub const BRKINT: u32 = 0x0002;
pub const IGNPAR: u32 = 0x0004;
pub const PARMRK: u32 = 0x0008;
pub const INPCK: u32 = 0x0010;
pub const ISTRIP: u32 = 0x0020;
pub const INLCR: u32 = 0x0040;
pub const IGNCR: u32 = 0x0080;
pub const ICRNL: u32 = 0x0100;
pub const IUCLC: u32 = 0x0200;
pub const IXON: u32 = 0x0400;
pub const IXANY: u32 = 0x0800;
pub const IXOFF: u32 = 0x1000;
pub const IMAXBEL: u32 = 0x2000;
pub const IUTF8: u32 = 0x4000;

// Output flags
pub const OPOST: u32 = 0x0001;
pub const OLCUC: u32 = 0x0002;
pub const ONLCR: u32 = 0x0004;
pub const OCRNL: u32 = 0x0008;
pub const ONOCR: u32 = 0x0010;
pub const ONLRET: u32 = 0x0020;
pub const OFILL: u32 = 0x0040;
pub const OFDEL: u32 = 0x0080;

// Local flags
pub const ISIG: u32 = 0x0001;
pub const ICANON: u32 = 0x0002;
pub const ECHO: u32 = 0x0008;
pub const ECHOE: u32 = 0x0010;
pub const ECHOK: u32 = 0x0020;
pub const ECHONL: u32 = 0x0040;
pub const NOFLSH: u32 = 0x0080;
pub const TOSTOP: u32 = 0x0100;
pub const IEXTEN: u32 = 0x8000;

// Control characters
pub const VINTR: usize = 0;
pub const VQUIT: usize = 1;
pub const VERASE: usize = 2;
pub const VKILL: usize = 3;
pub const VEOF: usize = 4;
pub const VTIME: usize = 5;
pub const VMIN: usize = 6;
pub const VSWTC: usize = 7;
pub const VSTART: usize = 8;
pub const VSTOP: usize = 9;
pub const VSUSP: usize = 10;
pub const VEOL: usize = 11;
pub const VREPRINT: usize = 12;
pub const VDISCARD: usize = 13;
pub const VWERASE: usize = 14;
pub const VLNEXT: usize = 15;
pub const VEOL2: usize = 16;

// Terminal structure
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct Termios {
    pub c_iflag: u32,    // Input flags
    pub c_oflag: u32,    // Output flags
    pub c_cflag: u32,    // Control flags
    pub c_lflag: u32,    // Local flags
    pub c_line: u8,      // Line discipline
    pub c_cc: [u8; 32],  // Control characters
    pub c_ispeed: u32,   // Input speed
    pub c_ospeed: u32,   // Output speed
}

impl Termios {
    pub fn new() -> Self {
        let mut termios = Self {
            c_iflag: ICRNL | IXON,
            c_oflag: OPOST | ONLCR,
            c_cflag: 0,
            c_lflag: ISIG | ICANON | ECHO | ECHOE | ECHOK | IEXTEN,
            c_line: 0,
            c_cc: [0; 32],
            c_ispeed: 9600,
            c_ospeed: 9600,
        };
        
        // Set default control characters
        termios.c_cc[VINTR] = 3;    // Ctrl-C
        termios.c_cc[VQUIT] = 28;   // Ctrl-\
        termios.c_cc[VERASE] = 127; // DEL
        termios.c_cc[VKILL] = 21;   // Ctrl-U
        termios.c_cc[VEOF] = 4;     // Ctrl-D
        termios.c_cc[VSTART] = 17;  // Ctrl-Q
        termios.c_cc[VSTOP] = 19;   // Ctrl-S
        termios.c_cc[VSUSP] = 26;   // Ctrl-Z
        termios.c_cc[VMIN] = 1;
        termios.c_cc[VTIME] = 0;
        
        termios
    }
}

// Window size structure
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct Winsize {
    pub ws_row: u16,
    pub ws_col: u16,
    pub ws_xpixel: u16,
    pub ws_ypixel: u16,
}

impl Winsize {
    pub fn new() -> Self {
        Self {
            ws_row: 24,
            ws_col: 80,
            ws_xpixel: 0,
            ws_ypixel: 0,
        }
    }
}

// TTY device structure
#[derive(Debug)]
pub struct TtyDevice {
    pub termios: Termios,
    pub winsize: Winsize,
    pub pgrp: i32,
    pub input_buffer: Vec<u8, 1024>,
    pub output_buffer: Vec<u8, 1024>,
    pub is_controlling: bool,
}

impl TtyDevice {
    pub fn new() -> Self {
        Self {
            termios: Termios::new(),
            winsize: Winsize::new(),
            pgrp: 0,
            input_buffer: Vec::new(),
            output_buffer: Vec::new(),
            is_controlling: false,
        }
    }
    
    pub fn process_input(&mut self, byte: u8) -> Option<u8> {
        // Handle special characters if in canonical mode
        if self.termios.c_lflag & ICANON != 0 {
            match byte {
                b'\r' if self.termios.c_iflag & ICRNL != 0 => {
                    self.input_buffer.push(b'\n').ok();
                    if self.termios.c_lflag & ECHO != 0 {
                        return Some(b'\n');
                    }
                }
                b'\n' => {
                    self.input_buffer.push(b'\n').ok();
                    if self.termios.c_lflag & ECHO != 0 {
                        return Some(b'\n');
                    }
                }
                _ => {
                    self.input_buffer.push(byte).ok();
                    if self.termios.c_lflag & ECHO != 0 {
                        return Some(byte);
                    }
                }
            }
        } else {
            // Raw mode - pass through directly
            self.input_buffer.push(byte).ok();
            if self.termios.c_lflag & ECHO != 0 {
                return Some(byte);
            }
        }
        None
    }
    
    pub fn read_input(&mut self, buf: &mut [u8]) -> usize {
        let mut count = 0;
        let max_read = buf.len().min(self.input_buffer.len());
        
        for i in 0..max_read {
            if let Some(byte) = self.input_buffer.get(i) {
                buf[count] = *byte;
                count += 1;
                
                // In canonical mode, stop at newline
                if self.termios.c_lflag & ICANON != 0 && *byte == b'\n' {
                    break;
                }
            }
        }
        
        // Remove read bytes from buffer
        for _ in 0..count {
            if !self.input_buffer.is_empty() {
                self.input_buffer.remove(0);
            }
        }
        
        count
    }
    
    pub fn write_output(&mut self, data: &[u8]) -> usize {
        let mut written = 0;
        
        for &byte in data {
            if self.output_buffer.len() < self.output_buffer.capacity() {
                // Process output if OPOST is set
                if self.termios.c_oflag & OPOST != 0 {
                    match byte {
                        b'\n' if self.termios.c_oflag & ONLCR != 0 => {
                            self.output_buffer.push(b'\r').ok();
                            self.output_buffer.push(b'\n').ok();
                        }
                        _ => {
                            self.output_buffer.push(byte).ok();
                        }
                    }
                } else {
                    self.output_buffer.push(byte).ok();
                }
                written += 1;
            } else {
                break;
            }
        }
        
        written
    }
    
    pub fn flush_output(&mut self) -> Vec<u8, 1024> {
        let mut result = Vec::new();
        while let Some(byte) = self.output_buffer.pop() {
            result.push(byte).ok();
        }
        result.reverse();
        result
    }
}

// Global TTY devices
const MAX_TTYS: usize = 16;
pub static TTY_DEVICES: Mutex<Vec<TtyDevice, MAX_TTYS>> = Mutex::new(Vec::new());

lazy_static! {
    static ref TTY_INITIALIZED: Mutex<bool> = Mutex::new(false);
}

// Initialize TTY devices
pub fn init_tty_devices() {
    let mut initialized = TTY_INITIALIZED.lock();
    if !*initialized {
        let mut devices = TTY_DEVICES.lock();
        
        // Create console TTY (tty0)
        devices.push(TtyDevice::new()).ok();
        
        console_println!("[i] TTY devices initialized");
        *initialized = true;
    }
}

// Get TTY device by file descriptor
fn get_tty_for_fd(fd: i32) -> Option<usize> {
    // For now, map stdin/stdout/stderr to tty0
    if fd <= 2 {
        Some(0)
    } else {
        None
    }
}

// === LINUX COMPATIBLE DEVICE AND I/O MANAGEMENT SYSTEM CALL CONSTANTS ===
pub const SYS_DUP: usize = 23;         // Linux: dup
pub const SYS_DUP3: usize = 24;        // Linux: dup3
pub const SYS_FCNTL: usize = 25;       // Linux: fcntl
pub const SYS_INOTIFY_INIT1: usize = 26; // Linux: inotify_init1
pub const SYS_INOTIFY_ADD_WATCH: usize = 27; // Linux: inotify_add_watch
pub const SYS_INOTIFY_RM_WATCH: usize = 28;  // Linux: inotify_rm_watch
pub const SYS_IOCTL: usize = 29;       // Linux: ioctl
pub const SYS_IOPRIO_SET: usize = 30;  // Linux: ioprio_set
pub const SYS_IOPRIO_GET: usize = 31;  // Linux: ioprio_get
pub const SYS_FLOCK: usize = 32;       // Linux: flock
pub const SYS_MKNODAT: usize = 33;     // Linux: mknodat
pub const SYS_PIPE2: usize = 59;       // Linux: pipe2

// Legacy syscall aliases for backwards compatibility
pub const SYS_PIPE: usize = SYS_PIPE2; // Map pipe to pipe2
pub const SYS_DUP2: usize = SYS_DUP3;  // Map dup2 to dup3

// elinOS-specific device syscalls (keeping high numbers to avoid conflicts)
pub const SYS_GETDEVICES: usize = 950; // elinOS: get device info

// Linux compatible device management syscall handler
pub fn handle_device_syscall(args: &SyscallArgs) -> SysCallResult {
    match args.syscall_number {
        SYS_IOCTL => sys_ioctl(args.arg0_as_i32(), args.arg1, args.arg2),
        SYS_FCNTL => sys_fcntl(args.arg0_as_i32(), args.arg1_as_i32(), args.arg2),
        SYS_PIPE2 => sys_pipe2(args.arg0_as_mut_ptr::<i32>(), args.arg1_as_i32()),
        SYS_DUP => sys_dup(args.arg0_as_i32()),
        SYS_DUP3 => sys_dup3(args.arg0_as_i32(), args.arg1_as_i32(), args.arg2_as_i32()),
        SYS_FLOCK => sys_flock(args.arg0_as_i32(), args.arg1_as_i32()),
        SYS_MKNODAT => sys_mknodat(args.arg0_as_i32(), args.arg1_as_ptr::<u8>(), args.arg2 as u32, args.arg3 as u32),
        SYS_INOTIFY_INIT1 => sys_inotify_init1(args.arg0_as_i32()),
        SYS_INOTIFY_ADD_WATCH => sys_inotify_add_watch(args.arg0_as_i32(), args.arg1_as_ptr::<u8>(), args.arg2 as u32),
        SYS_INOTIFY_RM_WATCH => sys_inotify_rm_watch(args.arg0_as_i32(), args.arg1_as_i32()),
        SYS_IOPRIO_SET => sys_ioprio_set(args.arg0_as_i32(), args.arg1_as_i32(), args.arg2_as_i32()),
        SYS_IOPRIO_GET => sys_ioprio_get(args.arg0_as_i32(), args.arg1_as_i32()),
        SYS_GETDEVICES => sys_getdevices(),
        _ => SysCallResult::Error(crate::syscall::ENOSYS),
    }
}

// === SYSTEM CALL IMPLEMENTATIONS ===

fn sys_ioctl(fd: i32, request: usize, arg: usize) -> SysCallResult {
    console_println!("[i] SYS_IOCTL: fd={}, request=0x{:x}, arg=0x{:x}", fd, request, arg);
    
    // Initialize TTY devices if not already done
    init_tty_devices();
    
    let tty_index = match get_tty_for_fd(fd) {
        Some(index) => index,
        None => {
            console_println!("[x] No TTY device for fd {}", fd);
            return SysCallResult::Error(crate::syscall::ENOTTY);
        }
    };
    
    let mut devices = TTY_DEVICES.lock();
    if let Some(tty) = devices.get_mut(tty_index) {
        match request {
            TCGETS => {
                console_println!("[i] TCGETS: Getting terminal attributes");
                if arg == 0 {
                    return SysCallResult::Error(crate::syscall::EINVAL);
                }
                
                unsafe {
                    core::ptr::write(arg as *mut Termios, tty.termios);
                }
                SysCallResult::Success(0)
            }
            
            TCSETS | TCSETSW | TCSETSF => {
                console_println!("[i] TCSETS: Setting terminal attributes");
                if arg == 0 {
                    return SysCallResult::Error(crate::syscall::EINVAL);
                }
                
                let new_termios = unsafe {
                    core::ptr::read(arg as *const Termios)
                };
                
                // Handle flush flags
                if request == TCSETSF {
                    tty.input_buffer.clear();
                    tty.output_buffer.clear();
                }
                
                tty.termios = new_termios;
                console_println!("[i] Terminal attributes updated: lflag=0x{:x}, iflag=0x{:x}", 
                    tty.termios.c_lflag, tty.termios.c_iflag);
                SysCallResult::Success(0)
            }
            
            TIOCGWINSZ => {
                console_println!("[i] TIOCGWINSZ: Getting window size");
                if arg == 0 {
                    return SysCallResult::Error(crate::syscall::EINVAL);
                }
                
                unsafe {
                    core::ptr::write(arg as *mut Winsize, tty.winsize);
                }
                console_println!("[i] Window size: {}x{}", tty.winsize.ws_row, tty.winsize.ws_col);
                SysCallResult::Success(0)
            }
            
            TIOCSWINSZ => {
                console_println!("[i] TIOCSWINSZ: Setting window size");
                if arg == 0 {
                    return SysCallResult::Error(crate::syscall::EINVAL);
                }
                
                let new_winsize = unsafe {
                    core::ptr::read(arg as *const Winsize)
                };
                
                tty.winsize = new_winsize;
                console_println!("[i] Window size set to: {}x{}", tty.winsize.ws_row, tty.winsize.ws_col);
                SysCallResult::Success(0)
            }
            
            TIOCGPGRP => {
                console_println!("[i] TIOCGPGRP: Getting process group");
                if arg == 0 {
                    return SysCallResult::Error(crate::syscall::EINVAL);
                }
                
                unsafe {
                    core::ptr::write(arg as *mut i32, tty.pgrp);
                }
                SysCallResult::Success(0)
            }
            
            TIOCSPGRP => {
                console_println!("[i] TIOCSPGRP: Setting process group");
                if arg == 0 {
                    return SysCallResult::Error(crate::syscall::EINVAL);
                }
                
                let new_pgrp = unsafe {
                    core::ptr::read(arg as *const i32)
                };
                
                tty.pgrp = new_pgrp;
                console_println!("[i] Process group set to: {}", tty.pgrp);
                SysCallResult::Success(0)
            }
            
            TIOCSCTTY => {
                console_println!("[i] TIOCSCTTY: Setting controlling terminal");
                tty.is_controlling = true;
                SysCallResult::Success(0)
            }
            
            FIONREAD => {
                console_println!("[i] FIONREAD: Getting input buffer size");
                if arg == 0 {
                    return SysCallResult::Error(crate::syscall::EINVAL);
                }
                
                let available = tty.input_buffer.len() as i32;
                unsafe {
                    core::ptr::write(arg as *mut i32, available);
                }
                SysCallResult::Success(0)
            }
            
            TCFLSH => {
                console_println!("[i] TCFLSH: Flushing terminal");
                match arg {
                    0 => tty.input_buffer.clear(),  // TCIFLUSH
                    1 => tty.output_buffer.clear(), // TCOFLUSH
                    2 => { // TCIOFLUSH
                        tty.input_buffer.clear();
                        tty.output_buffer.clear();
                    }
                    _ => return SysCallResult::Error(crate::syscall::EINVAL),
                }
                SysCallResult::Success(0)
            }
            
            _ => {
                console_println!("[x] Unsupported ioctl request: 0x{:x}", request);
                SysCallResult::Error(crate::syscall::ENOSYS)
            }
        }
    } else {
        console_println!("[x] TTY device {} not found", tty_index);
        SysCallResult::Error(crate::syscall::ENODEV)
    }
}

fn sys_fcntl(_fd: i32, _cmd: i32, _arg: usize) -> SysCallResult {
    // TODO: Implement file control
    SysCallResult::Error(crate::syscall::ENOSYS)
}

fn sys_pipe2(_pipefd: *mut i32, _flags: i32) -> SysCallResult {
    // TODO: Implement pipe creation with flags
    SysCallResult::Error(crate::syscall::ENOSYS)
}

fn sys_dup(_oldfd: i32) -> SysCallResult {
    // TODO: Implement file descriptor duplication
    SysCallResult::Error(crate::syscall::ENOSYS)
}

fn sys_dup3(_oldfd: i32, _newfd: i32, _flags: i32) -> SysCallResult {
    // TODO: Implement file descriptor duplication to specific fd with flags
    SysCallResult::Error(crate::syscall::ENOSYS)
}

fn sys_flock(_fd: i32, _operation: i32) -> SysCallResult {
    // TODO: Implement file locking
    SysCallResult::Error(crate::syscall::ENOSYS)
}

fn sys_mknodat(_dirfd: i32, _pathname: *const u8, _mode: u32, _dev: u32) -> SysCallResult {
    // TODO: Implement device node creation
    SysCallResult::Error(crate::syscall::ENOSYS)
}

fn sys_inotify_init1(_flags: i32) -> SysCallResult {
    // TODO: Implement inotify initialization
    SysCallResult::Error(crate::syscall::ENOSYS)
}

fn sys_inotify_add_watch(_fd: i32, _pathname: *const u8, _mask: u32) -> SysCallResult {
    // TODO: Implement inotify watch addition
    SysCallResult::Error(crate::syscall::ENOSYS)
}

fn sys_inotify_rm_watch(_fd: i32, _wd: i32) -> SysCallResult {
    // TODO: Implement inotify watch removal
    SysCallResult::Error(crate::syscall::ENOSYS)
}

fn sys_ioprio_set(_which: i32, _who: i32, _ioprio: i32) -> SysCallResult {
    // TODO: Implement I/O priority setting
    SysCallResult::Error(crate::syscall::ENOSYS)
}

fn sys_ioprio_get(_which: i32, _who: i32) -> SysCallResult {
    // TODO: Implement I/O priority getting
    SysCallResult::Error(crate::syscall::ENOSYS)
}

fn sys_getdevices() -> SysCallResult {
    init_tty_devices();
    
    let devices = TTY_DEVICES.lock();
    console_println!("[i] TTY devices: {}", devices.len());
    
    for (i, tty) in devices.iter().enumerate() {
        console_println!("[i] TTY{}: pgrp={}, controlling={}", i, tty.pgrp, tty.is_controlling);
    }
    
    SysCallResult::Success(devices.len() as isize)
} 