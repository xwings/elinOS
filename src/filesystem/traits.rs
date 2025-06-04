// Common traits and types for filesystem implementations

use heapless::Vec;

/// Unified filesystem error types
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FilesystemError {
    NotInitialized,
    NotMounted,
    UnsupportedFilesystem,
    InvalidBootSector,
    InvalidSuperblock,
    FileNotFound,
    FilenameeTooLong,
    FilesystemFull,
    IoError,
    FileAlreadyExists,
    DirectoryNotFound,
    InvalidFAT,
    DeviceError,
    CorruptedFilesystem,
}

impl core::fmt::Display for FilesystemError {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        match self {
            FilesystemError::NotInitialized => write!(f, "Filesystem not initialized"),
            FilesystemError::NotMounted => write!(f, "Filesystem not mounted"),
            FilesystemError::UnsupportedFilesystem => write!(f, "Unsupported filesystem type"),
            FilesystemError::InvalidBootSector => write!(f, "Invalid FAT32 boot sector"),
            FilesystemError::InvalidSuperblock => write!(f, "Invalid ext4 superblock"),
            FilesystemError::FileNotFound => write!(f, "File not found"),
            FilesystemError::FilenameeTooLong => write!(f, "Filename too long"),
            FilesystemError::FilesystemFull => write!(f, "Filesystem full"),
            FilesystemError::IoError => write!(f, "I/O error"),
            FilesystemError::FileAlreadyExists => write!(f, "File already exists"),
            FilesystemError::DirectoryNotFound => write!(f, "Directory not found"),
            FilesystemError::InvalidFAT => write!(f, "Invalid FAT table"),
            FilesystemError::DeviceError => write!(f, "Device error"),
            FilesystemError::CorruptedFilesystem => write!(f, "Corrupted filesystem"),
        }
    }
}

pub type FilesystemResult<T> = Result<T, FilesystemError>;

/// Generic file entry structure
#[derive(Debug, Clone)]
pub struct FileEntry {
    pub name: heapless::String<256>,
    pub is_directory: bool,
    pub size: usize,
    pub inode: u64,  // Can be cluster (FAT32) or inode number (ext4)
}

impl FileEntry {
    pub fn new_file(name: &str, inode: u64, size: usize) -> FilesystemResult<Self> {
        let filename = heapless::String::try_from(name)
            .map_err(|_| FilesystemError::FilenameeTooLong)?;
            
        Ok(FileEntry {
            name: filename,
            is_directory: false,
            size,
            inode,
        })
    }
    
    pub fn new_directory(name: &str, inode: u64) -> FilesystemResult<Self> {
        let dirname = heapless::String::try_from(name)
            .map_err(|_| FilesystemError::FilenameeTooLong)?;
            
        Ok(FileEntry {
            name: dirname,
            is_directory: true,
            size: 0,
            inode,
        })
    }
}

/// Common filesystem trait that all filesystem implementations must implement
pub trait FileSystem {
    /// List all files in the filesystem
    fn list_files(&self) -> FilesystemResult<Vec<(heapless::String<64>, usize), 32>>;
    
    /// Read the contents of a file
    fn read_file(&self, filename: &str) -> FilesystemResult<Vec<u8, 4096>>;
    
    /// Check if a file exists
    fn file_exists(&self, filename: &str) -> bool;
    
    /// Get filesystem information (signature/magic, total blocks, block size)
    fn get_filesystem_info(&self) -> Option<(u16, u32, u16)>;
    
    /// Check if the filesystem is initialized
    fn is_initialized(&self) -> bool;
    
    /// Check if the filesystem is mounted
    fn is_mounted(&self) -> bool;
} 