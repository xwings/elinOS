//! Filesystem implementations for elinOS
//! 
//! This module provides filesystem support for both bootloader and kernel

use spin::Mutex;
use heapless::{Vec, String};

pub mod traits;
pub mod ext2;

pub use traits::*;
pub use ext2::*;

/// Global filesystem instance
pub static FILESYSTEM: Mutex<Ext2FileSystem> = Mutex::new(Ext2FileSystem::new());

/// Initialize the global filesystem
pub fn init_filesystem() -> FilesystemResult<()> {
    let mut fs = FILESYSTEM.lock();
    fs.init()
}

/// Read a file using the global filesystem
pub fn read_file(filename: &str) -> FilesystemResult<Vec<u8, 32768>> {
    let fs = FILESYSTEM.lock();
    fs.read_file(filename)
}

/// Write a file using the global filesystem
pub fn write_file(filename: &str, content: &str) -> FilesystemResult<()> {
    let mut fs = FILESYSTEM.lock();
    
    // Try to get existing file entry, or create a new one
    let file_entry = match fs.get_file_entry(filename) {
        Ok(entry) => entry,
        Err(FilesystemError::FileNotFound) => {
            // Create new file if it doesn't exist
            fs.create_file(filename)?
        },
        Err(e) => return Err(e),
    };
    
    // Write content to file at offset 0 (overwrite)
    fs.write_file(&file_entry, 0, content.as_bytes())?;
    
    Ok(())
}

/// Read an ELF file with larger buffer
pub fn read_elf_file(filename: &str) -> FilesystemResult<Vec<u8, 65536>> {
    let fs = FILESYSTEM.lock();
    
    // First get the regular sized buffer
    let content = fs.read_file(filename)?;
    
    // Convert to larger buffer size
    let mut large_buffer = Vec::<u8, 65536>::new();
    for byte in content.iter() {
        if large_buffer.push(*byte).is_err() {
            break; // Buffer full
        }
    }
    
    Ok(large_buffer)
}

/// List directory contents
pub fn list_directory(path: &str) -> FilesystemResult<Vec<(String<64>, usize, bool), 32>> {
    let fs = FILESYSTEM.lock();
    fs.list_directory(path)
}

/// Check filesystem status
pub fn check_filesystem() -> FilesystemResult<()> {
    let fs = FILESYSTEM.lock();
    if fs.is_mounted() {
        Ok(())
    } else {
        Err(FilesystemError::NotMounted)
    }
}