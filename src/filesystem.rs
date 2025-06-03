use spin::Mutex;
use heapless::Vec;
use crate::{console_println, simple_disk};
use heapless::String;
use core::{
    result::Result::{Ok, Err},
    option::Option::{Some, None},
    convert::TryFrom,
    mem::drop,
    iter::Iterator,
    clone::Clone,
};
use core::fmt::Write;

// === PROPER RUST ERROR TYPES ===
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FilesystemError {
    NotInitialized,
    NotMounted,
    InvalidBootSector,
    FileNotFound,
    FilenameeTooLong,
    FilesystemFull,
    IoError,
    FileAlreadyExists,
    DirectoryNotFound,
    InvalidFAT,
    DeviceError,
}

impl core::fmt::Display for FilesystemError {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        match self {
            FilesystemError::NotInitialized => write!(f, "Filesystem not initialized"),
            FilesystemError::NotMounted => write!(f, "Filesystem not mounted"),
            FilesystemError::InvalidBootSector => write!(f, "Invalid FAT32 boot sector"),
            FilesystemError::FileNotFound => write!(f, "File not found"),
            FilesystemError::FilenameeTooLong => write!(f, "Filename too long"),
            FilesystemError::FilesystemFull => write!(f, "Filesystem full"),
            FilesystemError::IoError => write!(f, "I/O error"),
            FilesystemError::FileAlreadyExists => write!(f, "File already exists"),
            FilesystemError::DirectoryNotFound => write!(f, "Directory not found"),
            FilesystemError::InvalidFAT => write!(f, "Invalid FAT table"),
            FilesystemError::DeviceError => write!(f, "Device error"),
        }
    }
}

// Type alias for cleaner Result types
pub type FilesystemResult<T> = Result<T, FilesystemError>;

// === FAT32 CONSTANTS ===
const SECTOR_SIZE: usize = 512;
const FAT32_SIGNATURE: u16 = 0xAA55;

// === FILE ENTRY STRUCTURE ===
#[derive(Debug, Clone)]
pub struct FileEntry {
    pub name: heapless::String<64>,
    pub is_directory: bool,
    pub size: usize,
    pub cluster: u16,
}

impl FileEntry {
    pub fn new_file(name: &str, cluster: u16, size: usize) -> FilesystemResult<Self> {
        let filename = heapless::String::try_from(name)
            .map_err(|_| FilesystemError::FilenameeTooLong)?;
            
        Ok(FileEntry {
            name: filename,
            is_directory: false,
            size,
            cluster,
        })
    }
    
    pub fn new_directory(name: &str, cluster: u16) -> FilesystemResult<Self> {
        let dirname = heapless::String::try_from(name)
            .map_err(|_| FilesystemError::FilenameeTooLong)?;
            
        Ok(FileEntry {
            name: dirname,
            is_directory: true,
            size: 0,
            cluster,
        })
    }
}

// === FAT32 BOOT SECTOR ===
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
struct Fat32BootSector {
    jump_boot: [u8; 3],
    oem_name: [u8; 8],
    bytes_per_sector: u16,
    sectors_per_cluster: u8,
    reserved_sectors: u16,
    num_fats: u8,
    root_entries: u16,      // 0 for FAT32
    total_sectors_16: u16,  // 0 for FAT32
    media: u8,
    sectors_per_fat_16: u16, // 0 for FAT32
    sectors_per_track: u16,
    num_heads: u16,
    hidden_sectors: u32,
    total_sectors_32: u32,
    sectors_per_fat_32: u32,
    flags: u16,
    version: u16,
    root_cluster: u32,
    info_sector: u16,
    backup_boot_sector: u16,
    _reserved: [u8; 12],
    drive_number: u8,
    _reserved1: u8,
    boot_signature: u8,
    volume_id: u32,
    volume_label: [u8; 11],
    filesystem_type: [u8; 8],
    _boot_code: [u8; 420],
    signature: u16,
}

// === FAT32 DIRECTORY ENTRY ===
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
struct Fat32DirEntry {
    name: [u8; 8],          // Filename (8.3 format)
    ext: [u8; 3],           // Extension
    attributes: u8,         // File attributes
    _reserved: u8,
    creation_time_tenth: u8,
    creation_time: u16,
    creation_date: u16,
    last_access_date: u16,
    first_cluster_hi: u16,  // High 16 bits of first cluster
    write_time: u16,
    write_date: u16,
    first_cluster_lo: u16,  // Low 16 bits of first cluster
    file_size: u32,
}

// === SIMPLE FAT32 FILESYSTEM ===
pub struct Fat32FileSystem {
    boot_sector: Option<Fat32BootSector>,
    files: Vec<FileEntry, 64>, // Cached file entries
    initialized: bool,
    mounted: bool,
    root_dir_sector: u64, // Dynamic root directory sector
}

impl Fat32FileSystem {
    pub const fn new() -> Self {
        Fat32FileSystem {
            boot_sector: None,
            files: Vec::new(),
            initialized: false,
            mounted: false,
            root_dir_sector: 0, // Will be calculated from boot sector
        }
    }
    
    pub fn init(&mut self) -> Result<(), FilesystemError> {
        console_println!("🗂️  Initializing FAT32 filesystem...");

        // Get IDE disk device
        let mut disk_device = simple_disk::SIMPLE_DISK.lock();
        
        if !disk_device.is_initialized() {
            return Err(FilesystemError::DeviceError);
        }

        // Read boot sector from disk
        let mut boot_buffer = [0u8; SECTOR_SIZE];
        disk_device.read_blocks(0, &mut boot_buffer)
            .map_err(|_| FilesystemError::IoError)?;
        
        // Release the disk lock before continuing
        drop(disk_device);
        
        // Parse boot sector
        let boot_sector: Fat32BootSector = unsafe {
            let bs_ptr = boot_buffer.as_ptr() as *const Fat32BootSector;
            *bs_ptr
        };

        // Verify FAT32 signature
        let signature = boot_sector.signature;
        let total_sectors = boot_sector.total_sectors_32;
        let bytes_per_sector = boot_sector.bytes_per_sector;
        let volume_label = boot_sector.volume_label;
        
        if signature != FAT32_SIGNATURE {
            console_println!("❌ Invalid FAT32 signature: 0x{:x} (expected 0x{:x})", 
                signature, FAT32_SIGNATURE);
            return Err(FilesystemError::InvalidBootSector);
        }

        console_println!("✅ Valid FAT32 filesystem detected!");
        console_println!("   📊 {} sectors total", total_sectors);
        console_println!("   💾 {} bytes per sector", bytes_per_sector);
        console_println!("   📁 Volume: {}", 
            core::str::from_utf8(&volume_label)
                .unwrap_or("<invalid>").trim_end_matches('\0'));

        // Calculate root directory sector from boot sector
        let sectors_per_cluster = boot_sector.sectors_per_cluster as u32;
        let reserved_sectors = boot_sector.reserved_sectors as u32;
        let num_fats = boot_sector.num_fats as u32;
        let sectors_per_fat = boot_sector.sectors_per_fat_32;
        let root_cluster = boot_sector.root_cluster;
        
        let fat_start = reserved_sectors;
        let data_start = fat_start + (num_fats * sectors_per_fat);
        let root_dir_sector = data_start + ((root_cluster - 2) * sectors_per_cluster);
        
        console_println!("   📂 Root cluster: {}", root_cluster);
        console_println!("   📍 Root directory sector: {} (calculated)", root_dir_sector);
        
        self.root_dir_sector = root_dir_sector as u64;
        self.boot_sector = Some(boot_sector);
        self.initialized = true;

        // Read root directory
        self.read_root_directory()?;
        self.mounted = true;

        console_println!("✅ FAT32 filesystem mounted");
        Ok(())
    }

    fn read_root_directory(&mut self) -> FilesystemResult<()> {
        console_println!("📁 Reading FAT32 root directory...");
        
        // Clear existing entries
        self.files.clear();
        
        let mut disk_device = simple_disk::SIMPLE_DISK.lock();
        if !disk_device.is_initialized() {
            return Err(FilesystemError::DeviceError);
        }
        
        let mut dir_buffer = [0u8; SECTOR_SIZE];
        
        match disk_device.read_blocks(self.root_dir_sector, &mut dir_buffer) {
            Ok(()) => {
                // Success - continue with parsing
            }
            Err(e) => {
                drop(disk_device);
                return Err(FilesystemError::IoError);
            }
        }
        
        drop(disk_device); // Release lock before parsing
        
        // Parse directory entries safely
        let mut offset = 0;
        let mut entries_found = 0;
        
        while offset + 32 <= SECTOR_SIZE && entries_found < 10 {
            let name_bytes = &dir_buffer[offset..offset+8];
            let ext_bytes = &dir_buffer[offset+8..offset+11];
            let attributes = dir_buffer[offset+11];
            
            // Check if entry is valid
            if name_bytes[0] == 0 {
                break; // End of directory
            }
            
            if name_bytes[0] == 0xE5 {
                offset += 32; // Deleted entry, skip
                continue;
            }
            
            // Skip Long File Name entries (attributes = 0x0F)
            if attributes == 0x0F {
                console_println!("🔍 Skipping LFN entry at offset {}", offset);
                offset += 32;
                continue;
            }
            
            // Skip volume label entries (attributes & 0x08)
            if attributes & 0x08 != 0 {
                console_println!("🔍 Skipping volume label entry at offset {}", offset);
                offset += 32;
                continue;
            }
            
            console_println!("🔍 Processing directory entry at offset {}: name={:?}, ext={:?}, attr=0x{:02x}", 
                offset, name_bytes, ext_bytes, attributes);
            
            // Extract cluster and size info
            let first_cluster_hi = u16::from_le_bytes([
                dir_buffer[offset+20], 
                dir_buffer[offset+21]
            ]);
            let first_cluster_lo = u16::from_le_bytes([
                dir_buffer[offset+26], 
                dir_buffer[offset+27]
            ]);
            let file_size = u32::from_le_bytes([
                dir_buffer[offset+28], 
                dir_buffer[offset+29], 
                dir_buffer[offset+30], 
                dir_buffer[offset+31]
            ]);
            
            let cluster = ((first_cluster_hi as u32) << 16) | (first_cluster_lo as u32);
            console_println!("🔍 Cluster info: hi={}, lo={}, combined={}, size={}", 
                first_cluster_hi, first_cluster_lo, cluster, file_size);
            
            // Build filename
            let mut filename = String::<64>::new();
            
            // Copy name part
            for &ch in name_bytes {
                if ch != b' ' && ch != 0 {
                    if filename.push(ch as char).is_err() {
                        break;
                    }
                }
            }
            
            // Add extension if present
            if ext_bytes[0] != b' ' && ext_bytes[0] != 0 {
                if filename.push('.').is_ok() {
                    for &ch in ext_bytes {
                        if ch != b' ' && ch != 0 {
                            if filename.push(ch as char).is_err() {
                                break;
                            }
                        }
                    }
                }
            }
            
            let filename_str = filename.as_str();
            console_println!("🔍 Parsed filename: '{}'", filename_str);
            
            if !filename_str.is_empty() {
                let is_dir = attributes & 0x10 != 0;
                
                let result = if is_dir {
                    FileEntry::new_directory(&filename_str, cluster as u16)
                } else {
                    FileEntry::new_file(&filename_str, cluster as u16, file_size as usize)
                };
                
                if let Ok(file_entry) = result {
                    console_println!("✅ Added file: '{}' (cluster={}, size={}, is_dir={})", 
                        file_entry.name.as_str(), file_entry.cluster, file_entry.size, is_dir);
                    if self.files.push(file_entry).is_ok() {
                        entries_found += 1;
                    } else {
                        console_println!("⚠️ File list full, stopping");
                        break; // Vec is full
                    }
                } else {
                    console_println!("❌ Failed to create file entry for '{}'", filename_str);
                }
            }
            
            offset += 32;
        }
        
        console_println!("📊 Found {} files in FAT32 root directory", entries_found);
        Ok(())
    }
    
    pub fn list_files(&self) -> Result<Vec<(heapless::String<64>, usize), 32>, FilesystemError> {
        if !self.is_mounted() {
            return Err(FilesystemError::NotMounted);
        }

        let mut result = Vec::new();
        for file in &self.files {
            result.push((file.name.clone(), file.size))
                .map_err(|_| FilesystemError::FilesystemFull)?;
        }
        Ok(result)
    }
    
    pub fn read_file(&self, filename: &str) -> Result<Vec<u8, 4096>, FilesystemError> {
        if !self.is_mounted() {
            return Err(FilesystemError::NotMounted);
        }

        // Find file
        let file_entry = self.files.iter()
            .find(|f| f.name.as_str() == filename && !f.is_directory)
            .ok_or(FilesystemError::FileNotFound)?;

        console_println!("📖 Reading FAT32 file '{}' from cluster {}, size {} bytes", 
            filename, file_entry.cluster, file_entry.size);

        // Safety check: ensure cluster is valid
        if file_entry.cluster == 0 {
            console_println!("❌ Invalid cluster 0 for file '{}'", filename);
            return Err(FilesystemError::IoError);
        }

        // Read file content from cluster
        let mut file_content = Vec::new();
        
        {
            let mut disk_device = simple_disk::SIMPLE_DISK.lock();
            
            if !disk_device.is_initialized() {
                return Err(FilesystemError::DeviceError);
            }

            // Calculate sector from cluster using boot sector data
            let boot_sector = self.boot_sector.ok_or(FilesystemError::NotInitialized)?;
            let sectors_per_cluster = boot_sector.sectors_per_cluster as u32;
            let reserved_sectors = boot_sector.reserved_sectors as u32;
            let num_fats = boot_sector.num_fats as u32;
            let sectors_per_fat = boot_sector.sectors_per_fat_32;
            
            let data_start = reserved_sectors + (num_fats * sectors_per_fat);
            let sector = data_start as u64 + ((file_entry.cluster as u32 - 2) * sectors_per_cluster) as u64;
            
            console_println!("📂 File cluster {} maps to sector {} (data_start={}, sectors_per_cluster={})", 
                file_entry.cluster, sector, data_start, sectors_per_cluster);
            
            // Safety check: ensure sector is reasonable
            if sector > 131072 {  // Max sectors in our 64MB disk
                console_println!("❌ Calculated sector {} is beyond disk capacity", sector);
                return Err(FilesystemError::IoError);
            }
            
            let mut sector_buf = [0u8; SECTOR_SIZE];
            match disk_device.read_blocks(sector, &mut sector_buf) {
                Ok(()) => {
                    console_println!("✅ Read file sector {} successfully", sector);
                    drop(disk_device); // Release lock
                    
                    // Safety check: limit file size to prevent overflow
                    let safe_file_size = file_entry.size.min(SECTOR_SIZE).min(4096);
                    console_println!("📏 Reading {} bytes (file_size={}, limited to {})", 
                        safe_file_size, file_entry.size, safe_file_size);
                    
                    // Copy the file content, up to the safe file size
                    for i in 0..safe_file_size {
                        let byte_val = sector_buf[i];
                        if file_content.push(byte_val).is_err() {
                            console_println!("⚠️ File content buffer full at {} bytes", i);
                            break;
                        }
                        
                        // Stop at null terminator for text files
                        if byte_val == 0 && i > 0 {
                            console_println!("📄 Found null terminator at position {}", i);
                            break;
                        }
                    }
                }
                Err(e) => {
                    drop(disk_device);
                    console_println!("❌ Failed to read file sector: {:?}", e);
                    return Err(FilesystemError::IoError);
                }
            }
        }

        console_println!("✅ File '{}' read successfully ({} bytes)", filename, file_content.len());
        Ok(file_content)
    }
    
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }

    pub fn is_mounted(&self) -> bool {
        self.mounted
    }
    
    pub fn get_filesystem_info(&self) -> Option<(u16, u32, u16)> {
        self.boot_sector.map(|bs| (bs.signature, bs.total_sectors_32, bs.bytes_per_sector))
    }
    
    pub fn file_exists(&self, filename: &str) -> bool {
        self.files.iter().any(|f| f.name.as_str() == filename && !f.is_directory)
    }
    
    pub fn create_file(&mut self, filename: &str, _content: &[u8]) -> Result<(), FilesystemError> {
        if !self.is_mounted() {
            return Err(FilesystemError::NotMounted);
        }

        // Check if file already exists
        if self.file_exists(filename) {
            return Err(FilesystemError::FileAlreadyExists);
        }

        // For now, just add to memory cache (real implementation would write to disk)
        let new_cluster = 10 + self.files.len() as u16; // Start clusters from 10
        let new_file = FileEntry::new_file(filename, new_cluster, _content.len())?;
        
        self.files.push(new_file).map_err(|_| FilesystemError::FilesystemFull)?;
        
        console_println!("✅ Created file: {} ({} bytes, cluster: {})", 
            filename, _content.len(), new_cluster);

        // TODO: Actually write to disk via IDE
        
        Ok(())
    }
    
    pub fn delete_file(&mut self, filename: &str) -> Result<(), FilesystemError> {
        if !self.is_mounted() {
            return Err(FilesystemError::NotMounted);
        }

        // Find and remove the file
        for (i, file) in self.files.iter().enumerate() {
            if file.name.as_str() == filename && !file.is_directory {
                console_println!("🗑️  Deleting file: {} (cluster: {})", filename, file.cluster);
                self.files.swap_remove(i);
                
                // TODO: Actually delete from disk via IDE
                
                return Ok(());
            }
        }
        
        Err(FilesystemError::FileNotFound)
    }
}

// Global filesystem instance
pub static FILESYSTEM: Mutex<Fat32FileSystem> = Mutex::new(Fat32FileSystem::new());

pub fn init_filesystem() -> Result<(), FilesystemError> {
    let mut fs = FILESYSTEM.lock();
    fs.init()
}

// Convenience functions for commands
pub fn list_files() -> Result<(), FilesystemError> {
    let fs = FILESYSTEM.lock();
    
    console_println!("📁 FAT32 Filesystem contents (IDE disk):");
    if let Some((signature, total_sectors, bytes_per_sector)) = fs.get_filesystem_info() {
        console_println!("Boot signature: 0x{:x}", signature);
        console_println!("Total sectors: {}", total_sectors);
        console_println!("Bytes per sector: {}", bytes_per_sector);
    }
    console_println!();
    
    for file in fs.files.iter() {
        let file_type = if file.is_directory { "DIR " } else { "FILE" };
        console_println!("  {} {:>8} bytes  {} (cluster: {})", 
            file_type, file.size, file.name.as_str(), file.cluster);
    }
    
    console_println!("\nTotal files: {} (FAT32 on IDE)", fs.files.len());
    Ok(())
}

pub fn read_file(filename: &str) -> Result<(), FilesystemError> {
    let fs = FILESYSTEM.lock();
    
    match fs.read_file(filename) {
        Ok(content) => {
            console_println!("📖 Reading file: {} (from FAT32 IDE disk)", filename);
            
            if let Ok(content_str) = core::str::from_utf8(&content) {
                console_println!("Content:");
                console_println!("{}", content_str);
            } else {
                console_println!("(Binary file - {} bytes)", content.len());
            }
            Ok(())
        }
        Err(e) => {
            console_println!("❌ Failed to read file: {}", e);
            Err(e)
        }
    }
}

pub fn check_filesystem() -> Result<(), FilesystemError> {
    let fs = FILESYSTEM.lock();
    
    console_println!("🔍 FAT32 Filesystem Check:");
    if let Some((signature, total_sectors, bytes_per_sector)) = fs.get_filesystem_info() {
        console_println!("  Boot Signature: 0x{:x} {}", 
            signature,
            if signature == FAT32_SIGNATURE { "✅ Valid FAT32" } else { "❌ Invalid" }
        );
        console_println!("  Mount Status: {} ✅ Mounted from IDE disk", 
            if fs.is_mounted() { "MOUNTED" } else { "UNMOUNTED" }
        );
        console_println!("  Total Sectors: {}", total_sectors);
        console_println!("  Bytes per Sector: {}", bytes_per_sector);
        console_println!("  Storage: IDE Disk Interface");
    }
    console_println!("  Files in Cache: {}", fs.files.len());
    
    Ok(())
} 