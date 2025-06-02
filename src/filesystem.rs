use core::fmt::Write;
use spin::Mutex;
use crate::UART;

// Block device abstraction for filesystem
pub trait BlockDevice {
    fn read_block(&self, block_num: u64, buffer: &mut [u8]) -> Result<(), &'static str>;
    fn write_block(&self, block_num: u64, buffer: &[u8]) -> Result<(), &'static str>;
    fn block_size(&self) -> usize {
        512 // Standard block size
    }
}

// Simple in-memory filesystem for testing
pub struct SimpleFS {
    files: heapless::FnvIndexMap<heapless::String<32>, heapless::Vec<u8, 1024>, 16>,
}

impl SimpleFS {
    pub fn new() -> Self {
        let mut fs = SimpleFS {
            files: heapless::FnvIndexMap::new(),
        };
        
        // Add some test files
        let _ = fs.create_file("hello.txt", b"Hello from elinKernel filesystem!\n");
        let _ = fs.create_file("test.txt", b"This is a test file.\nLine 2\nLine 3\n");
        let _ = fs.create_file("readme.md", b"# elinKernel\n\nA simple kernel in Rust.\n");
        
        // Add a sample ELF binary for testing (minimal RISC-V ELF header)
        let sample_elf: [u8; 120] = [
            // ELF header (64 bytes)
            0x7f, b'E', b'L', b'F',  // e_ident[0-3]: ELF magic
            2,                        // e_ident[4]: ELFCLASS64
            1,                        // e_ident[5]: ELFDATA2LSB
            1,                        // e_ident[6]: EV_CURRENT
            0,                        // e_ident[7]: ELFOSABI_NONE
            0, 0, 0, 0, 0, 0, 0, 0,   // e_ident[8-15]: padding
            
            2, 0,                     // e_type: ET_EXEC (executable)
            243, 0,                   // e_machine: EM_RISCV (243)
            1, 0, 0, 0,               // e_version: EV_CURRENT
            0x00, 0x10, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, // e_entry: 0x10000
            64, 0, 0, 0, 0, 0, 0, 0,  // e_phoff: program header offset
            0, 0, 0, 0, 0, 0, 0, 0,   // e_shoff: section header offset  
            0, 0, 0, 0,               // e_flags
            64, 0,                    // e_ehsize: header size
            56, 0,                    // e_phentsize: program header size
            1, 0,                     // e_phnum: program header count
            64, 0,                    // e_shentsize: section header size
            0, 0,                     // e_shnum: section header count
            0, 0,                     // e_shstrndx: string table index
            
            // Program header (56 bytes)
            1, 0, 0, 0,               // p_type: PT_LOAD (1)
            5, 0, 0, 0,               // p_flags: PF_R | PF_X (4 | 1)
            0, 0, 0, 0, 0, 0, 0, 0,   // p_offset: 0
            0x00, 0x10, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, // p_vaddr: 0x10000
            0x00, 0x10, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, // p_paddr: 0x10000
            120, 0, 0, 0, 0, 0, 0, 0, // p_filesz: 120 bytes
            120, 0, 0, 0, 0, 0, 0, 0, // p_memsz: 120 bytes
            16, 0, 0, 0, 0, 0, 0, 0,  // p_align: 16
        ];
        
        let _ = fs.create_file("hello.elf", &sample_elf);
        
        fs
    }

    pub fn create_file(&mut self, name: &str, content: &[u8]) -> Result<(), &'static str> {
        let mut filename = heapless::String::new();
        if filename.push_str(name).is_err() {
            return Err("Filename too long");
        }
        
        let mut file_content = heapless::Vec::new();
        
        for &byte in content {
            if file_content.push(byte).is_err() {
                return Err("File too large");
            }
        }
        
        if self.files.insert(filename, file_content).is_err() {
            return Err("Too many files");
        }
        
        Ok(())
    }

    pub fn read_file(&self, name: &str) -> Option<&[u8]> {
        // Find the file by comparing string contents
        for (filename, content) in &self.files {
            if filename.as_str() == name {
                return Some(content.as_slice());
            }
        }
        None
    }

    pub fn list_files(&self) -> impl Iterator<Item = (&str, usize)> {
        self.files.iter().map(|(name, content)| (name.as_str(), content.len()))
    }

    pub fn delete_file(&mut self, name: &str) -> Result<(), &'static str> {
        // Find and remove the file by comparing string contents
        let mut found_key = None;
        for (filename, _) in &self.files {
            if filename.as_str() == name {
                found_key = Some(filename.clone());
                break;
            }
        }
        
        if let Some(key) = found_key {
            self.files.remove(&key);
            Ok(())
        } else {
            Err("File not found")
        }
    }

    pub fn file_exists(&self, name: &str) -> bool {
        for (filename, _) in &self.files {
            if filename.as_str() == name {
                return true;
            }
        }
        false
    }
}

// Global filesystem instance
pub static FILESYSTEM: Mutex<SimpleFS> = Mutex::new(SimpleFS {
    files: heapless::FnvIndexMap::new(),
});

pub fn init_filesystem() {
    let mut uart = UART.lock();
    let _ = writeln!(uart, "\nInitializing filesystem...");
    drop(uart);
    
    let mut fs = FILESYSTEM.lock();
    *fs = SimpleFS::new();
    
    let mut uart = UART.lock();
    let _ = writeln!(uart, "Simple in-memory filesystem initialized with test files");
}

// Filesystem commands for the shell
pub fn cmd_ls() {
    let fs = FILESYSTEM.lock();
    let mut uart = UART.lock();
    
    let _ = writeln!(uart, "Files:");
    for (name, size) in fs.list_files() {
        let _ = writeln!(uart, "  {} ({} bytes)", name, size);
    }
}

pub fn cmd_cat(filename: &str) {
    let fs = FILESYSTEM.lock();
    let mut uart = UART.lock();
    
    if let Some(content) = fs.read_file(filename) {
        let _ = writeln!(uart, "Contents of {}:", filename);
        // Print content as string (assuming it's text)
        for &byte in content {
            uart.putchar(byte);
        }
        let _ = writeln!(uart, "\n--- End of file ---");
    } else {
        let _ = writeln!(uart, "File '{}' not found", filename);
    }
}

pub fn cmd_touch(filename: &str) {
    let mut fs = FILESYSTEM.lock();
    let mut uart = UART.lock();
    
    if fs.file_exists(filename) {
        let _ = writeln!(uart, "File '{}' already exists", filename);
    } else {
        match fs.create_file(filename, b"") {
            Ok(()) => {
                let _ = writeln!(uart, "Created file '{}'", filename);
            },
            Err(e) => {
                let _ = writeln!(uart, "Failed to create file '{}': {}", filename, e);
            }
        }
    }
}

pub fn cmd_rm(filename: &str) {
    let mut fs = FILESYSTEM.lock();
    let mut uart = UART.lock();
    
    match fs.delete_file(filename) {
        Ok(()) => {
            let _ = writeln!(uart, "Deleted file '{}'", filename);
        },
        Err(e) => {
            let _ = writeln!(uart, "Failed to delete file '{}': {}", filename, e);
        }
    }
} 