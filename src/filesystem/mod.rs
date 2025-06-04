// Unified Filesystem Module for elinOS
// Supports multiple filesystem types with automatic detection

pub mod fat32;
pub mod ext4;
pub mod traits;

use spin::Mutex;
use crate::console_println;
use heapless::Vec;

pub use traits::{FileSystem, FileEntry, FilesystemError, FilesystemResult};
use fat32::Fat32FileSystem;
use ext4::Ext4FileSystem;

/// Filesystem type detection
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FilesystemType {
    Unknown,
    Fat32,
    Ext4,
}

impl core::fmt::Display for FilesystemType {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        match self {
            FilesystemType::Unknown => write!(f, "Unknown"),
            FilesystemType::Fat32 => write!(f, "FAT32"),
            FilesystemType::Ext4 => write!(f, "ext4"),
        }
    }
}

/// Unified filesystem container
pub enum Filesystem {
    Fat32(Fat32FileSystem),
    Ext4(Ext4FileSystem),
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
        console_println!("🔍 Starting unified filesystem initialization...");
        
        // Detect filesystem type
        self.fs_type = detect_filesystem_type()?;
        
        match self.fs_type {
            FilesystemType::Fat32 => {
                console_println!("🗂️  Mounting FAT32 filesystem...");
                let mut fat32_fs = Fat32FileSystem::new();
                fat32_fs.init()?;
                self.filesystem = Filesystem::Fat32(fat32_fs);
                console_println!("✅ FAT32 filesystem mounted successfully");
            }
            FilesystemType::Ext4 => {
                console_println!("🗂️  Mounting ext4 filesystem...");
                let mut ext4_fs = Ext4FileSystem::new();
                ext4_fs.init()?;
                self.filesystem = Filesystem::Ext4(ext4_fs);
                console_println!("✅ ext4 filesystem mounted successfully");
            }
            FilesystemType::Unknown => {
                console_println!("❌ No supported filesystem detected");
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
            Filesystem::Fat32(fs) => fs.is_initialized(),
            Filesystem::Ext4(fs) => fs.is_initialized(),
            Filesystem::None => false,
        }
    }
    
    /// Check if filesystem is mounted
    pub fn is_mounted(&self) -> bool {
        match &self.filesystem {
            Filesystem::Fat32(fs) => fs.is_mounted(),
            Filesystem::Ext4(fs) => fs.is_mounted(),
            Filesystem::None => false,
        }
    }
}

// Implement the FileSystem trait for UnifiedFileSystem
impl FileSystem for UnifiedFileSystem {
    fn list_files(&self) -> FilesystemResult<Vec<(heapless::String<64>, usize), 32>> {
        match &self.filesystem {
            Filesystem::Fat32(fs) => fs.list_files(),
            Filesystem::Ext4(fs) => fs.list_files(),
            Filesystem::None => Err(FilesystemError::NotMounted),
        }
    }
    
    fn read_file(&self, filename: &str) -> FilesystemResult<Vec<u8, 4096>> {
        match &self.filesystem {
            Filesystem::Fat32(fs) => fs.read_file(filename),
            Filesystem::Ext4(fs) => fs.read_file(filename),
            Filesystem::None => Err(FilesystemError::NotMounted),
        }
    }
    
    fn file_exists(&self, filename: &str) -> bool {
        match &self.filesystem {
            Filesystem::Fat32(fs) => fs.file_exists(filename),
            Filesystem::Ext4(fs) => fs.file_exists(filename),
            Filesystem::None => false,
        }
    }
    
    fn get_filesystem_info(&self) -> Option<(u16, u32, u16)> {
        match &self.filesystem {
            Filesystem::Fat32(fs) => fs.get_filesystem_info(),
            Filesystem::Ext4(fs) => fs.get_filesystem_info(),
            Filesystem::None => None,
        }
    }
    
    fn is_initialized(&self) -> bool {
        match &self.filesystem {
            Filesystem::Fat32(fs) => fs.is_initialized(),
            Filesystem::Ext4(fs) => fs.is_initialized(),
            Filesystem::None => false,
        }
    }
    
    fn is_mounted(&self) -> bool {
        match &self.filesystem {
            Filesystem::Fat32(fs) => fs.is_mounted(),
            Filesystem::Ext4(fs) => fs.is_mounted(),
            Filesystem::None => false,
        }
    }
}

/// Detect filesystem type by examining disk magic numbers
pub fn detect_filesystem_type() -> FilesystemResult<FilesystemType> {
    console_println!("🔍 Detecting filesystem type...");
    
    let mut disk_device = crate::virtio_blk::VIRTIO_BLK.lock();
    
    if !disk_device.is_initialized() {
        return Err(FilesystemError::DeviceError);
    }

    // First, check for FAT32 in boot sector (sector 0)
    let mut boot_buffer = [0u8; 512];
    disk_device.read_blocks(0, &mut boot_buffer)
        .map_err(|_| FilesystemError::IoError)?;
    
    let boot_signature = u16::from_le_bytes([boot_buffer[510], boot_buffer[511]]);
    
    if boot_signature == 0xAA55 {
        console_println!("✅ Boot signature found: 0x{:04x}", boot_signature);
        
        // Check for FAT32 filesystem type string
        let fs_type = &boot_buffer[82..90];
        if fs_type.starts_with(b"FAT32") {
            console_println!("✅ Confirmed FAT32 filesystem");
            drop(disk_device);
            return Ok(FilesystemType::Fat32);
        }
    }
    
    // Check for ext4 superblock (at offset 1024 bytes = sector 2)
    let mut superblock_buffer = [0u8; 1024];  // Read 2 sectors
    
    for i in 0..2 {
        let mut sector_buf = [0u8; 512];
        disk_device.read_blocks((2 + i) as u64, &mut sector_buf)
            .map_err(|_| FilesystemError::IoError)?;
        superblock_buffer[i * 512..(i + 1) * 512].copy_from_slice(&sector_buf);
    }
    
    drop(disk_device);
    
    // Check ext4 magic at offset 56 within the superblock
    let magic_offset = 56;
    let ext4_magic = u16::from_le_bytes([
        superblock_buffer[magic_offset], 
        superblock_buffer[magic_offset + 1]
    ]);
    
    if ext4_magic == 0xEF53 {
        console_println!("✅ ext4 magic number found: 0x{:04x}", ext4_magic);
        return Ok(FilesystemType::Ext4);
    }
    
    console_println!("❌ No recognized filesystem found");
    console_println!("   Boot signature: 0x{:04x}", boot_signature);
    console_println!("   ext4 magic: 0x{:04x} (expected: 0xEF53)", ext4_magic);
    
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

/// Read a file from the filesystem
pub fn read_file(filename: &str) -> FilesystemResult<Vec<u8, 4096>> {
    let fs = FILESYSTEM.lock();
    fs.read_file(filename)
}

/// Check if a file exists
pub fn file_exists(filename: &str) -> bool {
    let fs = FILESYSTEM.lock();
    fs.file_exists(filename)
}

/// Check filesystem status and display information
pub fn check_filesystem() -> Result<(), FilesystemError> {
    let fs = FILESYSTEM.lock();
    
    console_println!("🔍 Filesystem Check:");
    console_println!("  Type: {}", fs.get_filesystem_type());
    
    if let Some((signature, total_blocks, block_size)) = fs.get_filesystem_info() {
        console_println!("  Signature/Magic: 0x{:x} ✅", signature);
        console_println!("  Mount Status: {} ✅", 
            if fs.is_mounted() { "MOUNTED" } else { "UNMOUNTED" });
        console_println!("  Total Blocks/Sectors: {}", total_blocks);
        console_println!("  Block/Sector Size: {} bytes", block_size);
        console_println!("  Storage: VirtIO Block Device");
    }
    
    let file_count = match list_files() {
        Ok(files) => files.len(),
        Err(_) => 0,
    };
    console_println!("  Files in Cache: {}", file_count);
    
    Ok(())
} 