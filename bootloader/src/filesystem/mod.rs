// Unified Filesystem Module for elinOS
// Supports multiple filesystem types with automatic detection

pub mod ext2;
pub mod traits;

use spin::Mutex;
use elinos_common::console_println;
use heapless::Vec;

pub use traits::{FileSystem, FileEntry, FilesystemError, FilesystemResult};
use ext2::Ext2FileSystem;

/// Filesystem type detection
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FilesystemType {
    Unknown,
    Ext2,
}

impl core::fmt::Display for FilesystemType {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        match self {
            FilesystemType::Unknown => write!(f, "Unknown"),
            FilesystemType::Ext2 => write!(f, "ext2"),
        }
    }
}

/// Unified filesystem container
pub enum Filesystem {
    Ext2(Ext2FileSystem),
    None,
}

/// Main filesystem manager
pub struct UnifiedFileSystem {
    filesystem: Filesystem,
    fs_type: FilesystemType,
}

impl UnifiedFileSystem {
    pub const fn new() -> Self {
        UnifiedFileSystem {
            filesystem: Filesystem::None,
            fs_type: FilesystemType::Unknown,
        }
    }
    
    /// Initialize filesystem with automatic type detection
    pub fn init(&mut self) -> FilesystemResult<()> {
        console_println!("[i] Starting unified filesystem initialization...");
        
        // Detect filesystem type
        self.fs_type = detect_filesystem_type()?;
        
        match self.fs_type {
            FilesystemType::Ext2 => {
                // console_println!("[i] Mounting ext2 filesystem...");
                let mut ext2_fs = Ext2FileSystem::new();
                ext2_fs.init()?;
                self.filesystem = Filesystem::Ext2(ext2_fs);
                console_println!("[o] ext2 filesystem mounted successfully");
            }
            FilesystemType::Unknown => {
                console_println!("[x] No supported filesystem detected");
                return Err(FilesystemError::UnsupportedFilesystem);
            }
        }
        
        Ok(())
    }
    
    /// Get filesystem type
    pub fn get_filesystem_type(&self) -> FilesystemType {
        self.fs_type
    }
    
    /// Check if filesystem is initialized
    pub fn is_initialized(&self) -> bool {
        match &self.filesystem {
            Filesystem::Ext2(fs) => fs.is_initialized(),
            Filesystem::None => false,
        }
    }
    
    /// Check if filesystem is mounted
    pub fn is_mounted(&self) -> bool {
        match &self.filesystem {
            Filesystem::Ext2(fs) => fs.is_mounted(),
            Filesystem::None => false,
        }
    }
}

// Implement the FileSystem trait for UnifiedFileSystem
impl FileSystem for UnifiedFileSystem {
    fn list_files(&self) -> FilesystemResult<Vec<(heapless::String<64>, usize), 32>> {
        match &self.filesystem {
            Filesystem::Ext2(fs) => fs.list_files(),
            Filesystem::None => Err(FilesystemError::NotMounted),
        }
    }
    
    fn list_directory(&self, path: &str) -> FilesystemResult<Vec<(heapless::String<64>, usize, bool), 32>> {
        match &self.filesystem {
            Filesystem::Ext2(fs) => fs.list_directory(path),
            Filesystem::None => Err(FilesystemError::NotMounted),
        }
    }
    
    fn read_file(&self, filename: &str) -> FilesystemResult<heapless::Vec<u8, 32768>> {
        match &self.filesystem {
            Filesystem::Ext2(fs) => fs.read_file(filename),
            Filesystem::None => Err(FilesystemError::NotMounted),
        }
    }
    
    fn file_exists(&self, filename: &str) -> bool {
        match &self.filesystem {
            Filesystem::Ext2(fs) => fs.file_exists(filename),
            Filesystem::None => false,
        }
    }
    
    fn get_filesystem_info(&self) -> Option<(u16, u32, u16)> {
        match &self.filesystem {
            Filesystem::Ext2(fs) => fs.get_filesystem_info(),
            Filesystem::None => None,
        }
    }
    
    fn is_initialized(&self) -> bool {
        match &self.filesystem {
            Filesystem::Ext2(fs) => fs.is_initialized(),
            Filesystem::None => false,
        }
    }
    
    fn is_mounted(&self) -> bool {
        match &self.filesystem {
            Filesystem::Ext2(fs) => fs.is_mounted(),
            Filesystem::None => false,
        }
    }

    // TODO: Implement these methods for UnifiedFileSystem by dispatching to the active FS
    fn create_file(&mut self, path: &str) -> FilesystemResult<FileEntry> {
        match &mut self.filesystem {
            Filesystem::Ext2(fs) => fs.create_file(path),
            Filesystem::None => Err(FilesystemError::NotMounted),
        }
    }

    fn create_directory(&mut self, path: &str) -> FilesystemResult<FileEntry> {
        match &mut self.filesystem {
            Filesystem::Ext2(fs) => fs.create_directory(path),
            Filesystem::None => Err(FilesystemError::NotMounted),
        }
    }

    fn write_file(&mut self, file: &FileEntry, offset: u64, data: &[u8]) -> FilesystemResult<usize> {
        match &mut self.filesystem {
            Filesystem::Ext2(fs) => fs.write_file(file, offset, data),
            Filesystem::None => Err(FilesystemError::NotMounted),
        }
    }

    fn delete_file(&mut self, path: &str) -> FilesystemResult<()> {
        match &mut self.filesystem {
            Filesystem::Ext2(fs) => fs.delete_file(path),
            Filesystem::None => Err(FilesystemError::NotMounted),
        }
    }

    fn delete_directory(&mut self, path: &str) -> FilesystemResult<()> {
        match &mut self.filesystem {
            Filesystem::Ext2(fs) => fs.delete_directory(path),
            Filesystem::None => Err(FilesystemError::NotMounted),
        }
    }

    fn truncate_file(&mut self, file: &FileEntry, new_size: u64) -> FilesystemResult<()> {
        match &mut self.filesystem {
            Filesystem::Ext2(fs) => fs.truncate_file(file, new_size),
            Filesystem::None => Err(FilesystemError::NotMounted),
        }
    }

    fn sync(&mut self) -> FilesystemResult<()> {
        match &mut self.filesystem {
            Filesystem::Ext2(fs) => fs.sync(),
            Filesystem::None => Err(FilesystemError::NotMounted),
        }
    }

    fn read_file_to_buffer(&self, filename: &str, buffer: &mut [u8]) -> FilesystemResult<usize> {
        match &self.filesystem {
            Filesystem::Ext2(fs) => fs.read_file_to_buffer(filename, buffer),
            Filesystem::None => Err(FilesystemError::NotInitialized),
        }
    }

    fn get_file_size(&self, filename: &str) -> FilesystemResult<usize> {
        match &self.filesystem {
            Filesystem::Ext2(fs) => fs.get_file_size(filename),
            Filesystem::None => Err(FilesystemError::NotInitialized),
        }
    }
}

/// Detect filesystem type by reading specific disk locations
pub fn detect_filesystem_type() -> FilesystemResult<FilesystemType> {
    // console_println!("filesystem::detect_filesystem_type: Starting detection...");
            let mut disk_device = crate::virtio::VIRTIO_BLK.lock();

    if !disk_device.is_initialized() {
        // console_println!("filesystem::detect_filesystem_type: VirtIO disk not initialized.");
        return Err(FilesystemError::DeviceError);
    }

    // IMPORTANT: Warm up VirtIO driver with a simple read to ensure clean buffer state
    // This prevents VirtIO buffer corruption issues that occur when ext2 detection
    let mut warmup_buf = [0u8; 512];
    match disk_device.read_blocks(0, &mut warmup_buf) {
        Ok(_) => {
        }
        Err(e) => {
            console_println!("[!] VirtIO warmup failed: {:?}, continuing anyway", e);
            // Continue anyway - the warmup attempt may have still helped
        }
    }

    // Try ext2 detection (check Superblock Magic)
    // console_println!("filesystem::detect_filesystem_type: Attempting to read sectors for ext2 superblock check...");
    const EXT2_SUPERBLOCK_OFFSET: usize = 1024;
    const SECTOR_SIZE: usize = 512;
    let start_sector = EXT2_SUPERBLOCK_OFFSET / SECTOR_SIZE; // Should be sector 2
    let mut sb_buffer = [0u8; 1024];

    for i in 0..2 {
        let current_sector_to_read = (start_sector + i) as u64;
        // console_println!("filesystem::detect_filesystem_type: Reading ext2 SB sector {}", current_sector_to_read);
        let mut sector_buf = [0u8; SECTOR_SIZE];
        match disk_device.read_blocks(current_sector_to_read, &mut sector_buf) {
            Ok(_) => {
                // console_println!("filesystem::detect_filesystem_type: Successfully read ext2 SB sector {}", current_sector_to_read);
                sb_buffer[i * SECTOR_SIZE..(i + 1) * SECTOR_SIZE].copy_from_slice(&sector_buf);
            }
            Err(e) => {
                // console_println!("filesystem::detect_filesystem_type: Failed to read ext2 SB sector {}: {:?}", current_sector_to_read, e);
                // If we can't read these, it's unlikely ext2, or there's a general disk issue.
                return Ok(FilesystemType::Unknown); // Return Unknown, don't mask with IoError yet
            }
        }
    }

    // Parse ext2 superblock magic from sb_buffer
    // ext2 magic 0xEF53 is at offset 0x38 (56) within the 1024-byte superblock data
    if sb_buffer.len() >= 56 + 2 {
        let ext2_magic = u16::from_le_bytes([sb_buffer[56], sb_buffer[57]]);
        if ext2_magic == 0xEF53 {
            console_println!("[o] ext2 magic 0xEF53 found at offset 56");
            return Ok(FilesystemType::Ext2);
        }
        console_println!("[!] ext2 magic not found, read 0x{:04X} at offset 56", ext2_magic);
        
        // Debug: Show first few bytes of superblock area
        console_println!("[DEBUG] Superblock bytes 0-15: {:02X?}", &sb_buffer[0..16]);
        console_println!("[DEBUG] Superblock bytes 48-63: {:02X?}", &sb_buffer[48..64]);
        console_println!("[DEBUG] Superblock bytes 56-57: [{:02X}, {:02X}] = 0x{:04X}", 
            sb_buffer[56], sb_buffer[57], ext2_magic);
    } else {
        console_println!("[!] Superblock buffer too short for ext2 magic check");
    }

    // console_println!("filesystem::detect_filesystem_type: No known filesystem type identified.");
    Ok(FilesystemType::Unknown)
}

// === GLOBAL FILESYSTEM INSTANCE ===

pub static FILESYSTEM: Mutex<UnifiedFileSystem> = Mutex::new(UnifiedFileSystem::new());

// === PUBLIC API FUNCTIONS ===

/// Initialize the filesystem with automatic detection
pub fn init_filesystem() -> FilesystemResult<()> {
    let mut fs = FILESYSTEM.lock();
    fs.init()
}

/// List files in the filesystem
pub fn list_files() -> FilesystemResult<Vec<(heapless::String<64>, usize), 32>> {
    let fs = FILESYSTEM.lock();
    fs.list_files()
}

/// List files in a specific directory path
pub fn list_directory(path: &str) -> FilesystemResult<Vec<(heapless::String<64>, usize, bool), 32>> {
    let fs = FILESYSTEM.lock();
    fs.list_directory(path)
}

/// Read a file from the filesystem
pub fn read_file(filename: &str) -> FilesystemResult<heapless::Vec<u8, 32768>> {
    let fs = FILESYSTEM.lock();
    fs.read_file(filename)
}

/// Read an ELF file from the filesystem (supports larger files)
pub fn read_elf_file(filename: &str) -> Result<heapless::Vec<u8, 32768>, &'static str> {
    // Use the regular read_file with larger buffer
    match read_file(filename) {
        Ok(data) => Ok(data),
        Err(_) => Err("Failed to read ELF file"),
    }
}

/// Check if a file exists
pub fn file_exists(filename: &str) -> bool {
    let fs = FILESYSTEM.lock();
    fs.file_exists(filename)
}

/// Get file entry for an existing file (for internal use)
fn get_file_entry(fs: &UnifiedFileSystem, filename: &str) -> FilesystemResult<FileEntry> {
    match &fs.filesystem {
        Filesystem::Ext2(ext2_fs) => {
            // Use the public method from ext2 filesystem
            ext2_fs.get_file_entry(filename)
        }
        Filesystem::None => Err(FilesystemError::NotMounted),
    }
}

/// Write data to a file (create if it doesn't exist)
pub fn write_file(filename: &str, content: &str) -> FilesystemResult<()> {
    let mut fs = FILESYSTEM.lock();
    
    let file_entry = if fs.file_exists(filename) {
        // Try to get existing file entry efficiently
        match get_file_entry(&fs, filename) {
            Ok(entry) => entry,
            Err(_) => {
                // Fallback: delete and recreate
                fs.delete_file(filename)?;
                fs.create_file(filename)?
            }
        }
    } else {
        // Create new file
        fs.create_file(filename)?
    };
    
    // Write content to file
    let data = content.as_bytes();
    fs.write_file(&file_entry, 0, data)?;
    
    Ok(())
}

/// Check filesystem status and display information
pub fn check_filesystem() -> Result<(), FilesystemError> {
    let fs = FILESYSTEM.lock();
    
    console_println!("[i] Filesystem Check:");
    console_println!("   Type: {}", fs.get_filesystem_type());
    
    if let Some((signature, total_blocks, block_size)) = fs.get_filesystem_info() {
        console_println!("   Signature/Magic: 0x{:x} [o]", signature);
        console_println!("   Mount Status: {} [o]", 
            if fs.is_mounted() { "MOUNTED" } else { "UNMOUNTED" });
        console_println!("   Total Blocks/Sectors: {}", total_blocks);
        console_println!("   Block/Sector Size: {} bytes", block_size);
        console_println!("   Storage: VirtIO Block Device");
    }
    
    let file_count = match list_files() {
        Ok(files) => files.len(),
        Err(_) => 0,
    };
    console_println!("   Files in Cache: {}", file_count);
    
    Ok(())
} 