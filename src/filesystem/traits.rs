// Common traits and types for filesystem implementations

use heapless::Vec;
use crate::virtio_blk::DiskError;
use crate::console_println;

/// Unified filesystem error types
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FilesystemError {
    NotInitialized,
    NotMounted,
    UnsupportedFilesystem,
    InvalidBootSector,
    InvalidSuperblock,
    FileNotFound,
    FilenameTooLong,
    FilesystemFull,
    IoError,
    FileAlreadyExists,
    DirectoryNotFound,
    InvalidFAT,
    DeviceError,
    CorruptedFilesystem,
    InvalidPath,
    NotADirectory,
    IsADirectory,
    DirectoryNotEmpty,
    PathNotFound,
    InvalidFileNameCharacter,
    Other(heapless::String<64>),
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
            FilesystemError::FilenameTooLong => write!(f, "Filename too long"),
            FilesystemError::FilesystemFull => write!(f, "Filesystem full"),
            FilesystemError::IoError => write!(f, "I/O error"),
            FilesystemError::FileAlreadyExists => write!(f, "File already exists"),
            FilesystemError::DirectoryNotFound => write!(f, "Directory not found"),
            FilesystemError::InvalidFAT => write!(f, "Invalid FAT table"),
            FilesystemError::DeviceError => write!(f, "Device error"),
            FilesystemError::CorruptedFilesystem => write!(f, "Corrupted filesystem"),
            FilesystemError::InvalidPath => write!(f, "Invalid path"),
            FilesystemError::NotADirectory => write!(f, "Not a directory"),
            FilesystemError::IsADirectory => write!(f, "Is a directory"),
            FilesystemError::DirectoryNotEmpty => write!(f, "Directory not empty"),
            FilesystemError::PathNotFound => write!(f, "Path not found"),
            FilesystemError::InvalidFileNameCharacter => write!(f, "Invalid file name character"),
            FilesystemError::Other(ref s) => write!(f, "Other error: {}", s),
        }
    }
}

impl From<DiskError> for FilesystemError {
    fn from(disk_error: DiskError) -> Self {
        // You can map specific DiskError variants to FilesystemError variants
        // if needed, or have a general mapping.
        console_println!("DiskError occurred: {:?}", disk_error);
        FilesystemError::DeviceError // General mapping
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
            .map_err(|_| FilesystemError::FilenameTooLong)?;
            
        Ok(FileEntry {
            name: filename,
            is_directory: false,
            size,
            inode,
        })
    }
    
    pub fn new_directory(name: &str, inode: u64) -> FilesystemResult<Self> {
        let dirname = heapless::String::try_from(name)
            .map_err(|_| FilesystemError::FilenameTooLong)?;
            
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

    // == Write Operations ==

    /// Create a new empty file
    fn create_file(&mut self, path: &str) -> FilesystemResult<FileEntry>;

    /// Create a new directory
    fn create_directory(&mut self, path: &str) -> FilesystemResult<FileEntry>;

    /// Write data to a file at a given offset.
    /// Should extend the file if offset + data.len() > file_size.
    fn write_file(&mut self, file: &FileEntry, offset: u64, data: &[u8]) -> FilesystemResult<usize>;

    /// Remove a file
    fn delete_file(&mut self, path: &str) -> FilesystemResult<()>;

    /// Remove an empty directory
    fn delete_directory(&mut self, path: &str) -> FilesystemResult<()>;
    
    /// Truncate or extend a file to a new size.
    /// If new_size > current_size, the file should be zero-extended.
    /// If new_size < current_size, data beyond new_size should be discarded.
    fn truncate_file(&mut self, file: &FileEntry, new_size: u64) -> FilesystemResult<()>;

    /// Synchronize any in-memory caches to the disk
    fn sync(&mut self) -> FilesystemResult<()>;
} 