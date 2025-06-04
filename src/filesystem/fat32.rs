// FAT32 Filesystem Implementation

use super::traits::{FileSystem, FileEntry, FilesystemError, FilesystemResult};
use crate::{console_println, virtio_blk};
use heapless::{Vec, String};
use core::mem::drop;

/// FAT32 constants
const SECTOR_SIZE: usize = 512;
const FAT32_SIGNATURE: u16 = 0xAA55;

/// FAT32 Boot Sector structure
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
struct Fat32BootSector {
    jump_boot: [u8; 3],
    oem_name: [u8; 8],
    bytes_per_sector: u16,
    sectors_per_cluster: u8,
    reserved_sectors: u16,
    num_fats: u8,
    root_entries: u16,
    total_sectors_16: u16,
    media: u8,
    sectors_per_fat_16: u16,
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

/// FAT32 Directory Entry structure
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
struct Fat32DirEntry {
    name: [u8; 8],
    ext: [u8; 3],
    attributes: u8,
    _reserved: u8,
    creation_time_tenth: u8,
    creation_time: u16,
    creation_date: u16,
    last_access_date: u16,
    first_cluster_hi: u16,
    write_time: u16,
    write_date: u16,
    first_cluster_lo: u16,
    file_size: u32,
}

/// FAT32 Filesystem Implementation
pub struct Fat32FileSystem {
    boot_sector: Option<Fat32BootSector>,
    files: Vec<FileEntry, 64>,
    initialized: bool,
    mounted: bool,
    root_dir_sector: u64,
}

impl Fat32FileSystem {
    pub fn new() -> Self {
        Fat32FileSystem {
            boot_sector: None,
            files: Vec::new(),
            initialized: false,
            mounted: false,
            root_dir_sector: 0,
        }
    }
    
    /// Initialize the FAT32 filesystem
    pub fn init(&mut self) -> FilesystemResult<()> {
        console_println!("🗂️  Initializing FAT32 filesystem...");

        let mut disk_device = virtio_blk::VIRTIO_BLK.lock();
        
        if !disk_device.is_initialized() {
            return Err(FilesystemError::DeviceError);
        }

        let mut boot_buffer = [0u8; SECTOR_SIZE];
        disk_device.read_blocks(0, &mut boot_buffer)
            .map_err(|_| FilesystemError::IoError)?;
        
        drop(disk_device);
        
        let boot_sector: Fat32BootSector = unsafe {
            let bs_ptr = boot_buffer.as_ptr() as *const Fat32BootSector;
            *bs_ptr
        };

        if boot_sector.signature != FAT32_SIGNATURE {
            return Err(FilesystemError::InvalidBootSector);
        }

        console_println!("✅ Valid FAT32 filesystem detected!");
        let total_sectors = boot_sector.total_sectors_32;
        let bytes_per_sector = boot_sector.bytes_per_sector;
        console_println!("   📊 {} sectors total", total_sectors);
        console_println!("   💾 {} bytes per sector", bytes_per_sector);

        // Calculate root directory sector
        let sectors_per_cluster = boot_sector.sectors_per_cluster as u32;
        let reserved_sectors = boot_sector.reserved_sectors as u32;
        let num_fats = boot_sector.num_fats as u32;
        let sectors_per_fat = boot_sector.sectors_per_fat_32;
        let root_cluster = boot_sector.root_cluster;
        
        let fat_start = reserved_sectors;
        let data_start = fat_start + (num_fats * sectors_per_fat);
        let root_dir_sector = data_start + ((root_cluster - 2) * sectors_per_cluster);
        
        console_println!("   📂 Root cluster: {}", root_cluster);
        console_println!("   📍 Root directory sector: {}", root_dir_sector);
        
        self.root_dir_sector = root_dir_sector as u64;
        self.boot_sector = Some(boot_sector);
        self.initialized = true;

        self.read_root_directory()?;
        self.mounted = true;

        console_println!("✅ FAT32 filesystem mounted");
        Ok(())
    }
    
    /// Read and parse the root directory
    fn read_root_directory(&mut self) -> FilesystemResult<()> {
        console_println!("📂 Reading FAT32 root directory...");

        let mut disk_device = virtio_blk::VIRTIO_BLK.lock();
        let mut dir_buffer = [0u8; SECTOR_SIZE];
        
        disk_device.read_blocks(self.root_dir_sector, &mut dir_buffer)
            .map_err(|_| FilesystemError::IoError)?;
        
        drop(disk_device);

        let mut offset = 0;
        let mut entries_found = 0;
        
        while offset + 32 <= SECTOR_SIZE && entries_found < 10 {
            let name_bytes = &dir_buffer[offset..offset+8];
            let ext_bytes = &dir_buffer[offset+8..offset+11];
            let attributes = dir_buffer[offset+11];
            
            if name_bytes[0] == 0 { break; }
            if name_bytes[0] == 0xE5 { offset += 32; continue; }
            if attributes == 0x0F { offset += 32; continue; }
            if attributes & 0x08 != 0 { offset += 32; continue; }
            
            let first_cluster_hi = u16::from_le_bytes([dir_buffer[offset+20], dir_buffer[offset+21]]);
            let first_cluster_lo = u16::from_le_bytes([dir_buffer[offset+26], dir_buffer[offset+27]]);
            let file_size = u32::from_le_bytes([
                dir_buffer[offset+28], dir_buffer[offset+29], 
                dir_buffer[offset+30], dir_buffer[offset+31]
            ]);
            
            let cluster = ((first_cluster_hi as u32) << 16) | (first_cluster_lo as u32);
            
            // Create filename
            let mut filename = String::<256>::new();
            
            // Process 8.3 name
            for &byte in name_bytes.iter().take_while(|&&b| b != b' ' && b != 0) {
                if (filename.push(byte as char)).is_err() { break; }
            }
            
            // Add extension if present
            if ext_bytes[0] != b' ' && ext_bytes[0] != 0 {
                if filename.push('.').is_ok() {
                    for &byte in ext_bytes.iter().take_while(|&&b| b != b' ' && b != 0) {
                        if (filename.push(byte as char)).is_err() { break; }
                    }
                }
            }
            
            let is_directory = (attributes & 0x10) != 0;
            
            let file_entry = if is_directory {
                FileEntry::new_directory(&filename, cluster as u64)?
            } else {
                FileEntry::new_file(&filename, cluster as u64, file_size as usize)?
            };
            
            console_println!("📄 Found {}: {} (cluster: {}, size: {})", 
                if is_directory { "DIR " } else { "FILE" },
                filename, cluster, file_size);
            
            if self.files.push(file_entry).is_err() {
                console_println!("⚠️ File cache full");
                break;
            }
            
            entries_found += 1;
            offset += 32;
        }
        
        console_println!("✅ Found {} entries in FAT32 root directory", entries_found);
        Ok(())
    }
    
    /// Read file content from a cluster
    fn read_file_content(&self, file: &FileEntry) -> FilesystemResult<Vec<u8, 4096>> {
        let mut file_content = Vec::new();
        
        let mut disk_device = virtio_blk::VIRTIO_BLK.lock();
        
        if !disk_device.is_initialized() {
            return Err(FilesystemError::DeviceError);
        }

        let boot_sector = self.boot_sector.ok_or(FilesystemError::NotInitialized)?;
        let sectors_per_cluster = boot_sector.sectors_per_cluster as u32;
        let reserved_sectors = boot_sector.reserved_sectors as u32;
        let num_fats = boot_sector.num_fats as u32;
        let sectors_per_fat = boot_sector.sectors_per_fat_32;
        
        let data_start = reserved_sectors + (num_fats * sectors_per_fat);
        let sector = data_start as u64 + ((file.inode as u32 - 2) * sectors_per_cluster) as u64;
        
        let mut sector_buf = [0u8; SECTOR_SIZE];
        disk_device.read_blocks(sector, &mut sector_buf)
            .map_err(|_| FilesystemError::IoError)?;
        
        drop(disk_device);
        
        let safe_size = file.size.min(SECTOR_SIZE).min(4096);
        for i in 0..safe_size {
            let byte_val = sector_buf[i];
            if file_content.push(byte_val).is_err() {
                break;
            }
            if byte_val == 0 && i > 0 {
                break;
            }
        }

        Ok(file_content)
    }
}

impl FileSystem for Fat32FileSystem {
    fn list_files(&self) -> FilesystemResult<Vec<(heapless::String<64>, usize), 32>> {
        let mut result = Vec::new();
        
        for file in &self.files {
            let name_short = heapless::String::try_from(file.name.as_str())
                .unwrap_or_else(|_| {
                    let mut s = heapless::String::new();
                    let _ = s.push_str("invalid");
                    s
                });
            let _ = result.push((name_short, file.size));
        }
        
        Ok(result)
    }
    
    fn read_file(&self, filename: &str) -> FilesystemResult<Vec<u8, 4096>> {
        for file in &self.files {
            if file.name.as_str() == filename && !file.is_directory {
                return self.read_file_content(file);
            }
        }
        Err(FilesystemError::FileNotFound)
    }
    
    fn file_exists(&self, filename: &str) -> bool {
        self.files.iter().any(|f| f.name.as_str() == filename)
    }
    
    fn get_filesystem_info(&self) -> Option<(u16, u32, u16)> {
        if let Some(boot_sector) = &self.boot_sector {
            Some((boot_sector.signature, boot_sector.total_sectors_32, boot_sector.bytes_per_sector))
        } else {
            None
        }
    }
    
    fn is_initialized(&self) -> bool {
        self.initialized
    }
    
    fn is_mounted(&self) -> bool {
        self.mounted
    }
}

/// FAT32-specific test functions for debugging
impl Fat32FileSystem {
    /// Test function for debugging FAT32 filesystem operations
    /// This contains the low-level tests that were previously in main.rs
    pub fn run_debug_tests() -> FilesystemResult<()> {
        use crate::{console_println, virtio_blk};
        
        console_println!("🧪 Running FAT32 debug tests...");
        
        // Test 1: Read boot sector
        console_println!("🔍 Test 1: Reading boot sector...");
        {
            let mut disk_device = virtio_blk::VIRTIO_BLK.lock();
            let mut buffer = [0u8; 512];
            match disk_device.read_blocks(0, &mut buffer) {
                Ok(()) => {
                    console_println!("✅ Boot sector read successful");
                    
                    // Parse key FAT32 fields
                    let sectors_per_cluster = buffer[13];
                    let reserved_sectors = u16::from_le_bytes([buffer[14], buffer[15]]);
                    let num_fats = buffer[16];
                    let sectors_per_fat = u32::from_le_bytes([buffer[36], buffer[37], buffer[38], buffer[39]]);
                    let root_cluster = u32::from_le_bytes([buffer[44], buffer[45], buffer[46], buffer[47]]);
                    
                    console_println!("📊 FAT32 Boot Sector Analysis:");
                    console_println!("  Sectors per cluster: {}", sectors_per_cluster);
                    console_println!("  Reserved sectors: {}", reserved_sectors);
                    console_println!("  Number of FATs: {}", num_fats);
                    console_println!("  Sectors per FAT: {}", sectors_per_fat);
                    console_println!("  Root cluster: {}", root_cluster);
                    
                    // Calculate filesystem layout
                    let fat_start = reserved_sectors as u32;
                    let data_start = fat_start + (num_fats as u32 * sectors_per_fat);
                    let root_sector = data_start + ((root_cluster - 2) * sectors_per_cluster as u32);
                    
                    console_println!("  FAT starts at sector: {}", fat_start);
                    console_println!("  Data starts at sector: {}", data_start);
                    console_println!("  Root directory sector: {}", root_sector);
                    
                    // Test 2: Read root directory
                    console_println!("🔍 Test 2: Reading root directory...");
                    match disk_device.read_blocks(root_sector as u64, &mut buffer) {
                        Ok(()) => {
                            console_println!("✅ Root directory read successful");
                            console_println!("🔍 First 32 bytes: {:02x?}", &buffer[0..32]);
                            
                            // Look for directory entries
                            if buffer[0] != 0 && buffer[0] != 0xE5 {
                                console_println!("🎉 Found directory entry!");
                                let name_bytes = &buffer[0..8];
                                let ext_bytes = &buffer[8..11];
                                console_println!("  Name: {:?}", name_bytes);
                                console_println!("  Ext: {:?}", ext_bytes);
                                console_println!("  Attributes: 0x{:02x}", buffer[11]);
                            }
                        }
                        Err(e) => {
                            console_println!("❌ Failed to read root directory: {:?}", e);
                            return Err(FilesystemError::IoError);
                        }
                    }
                }
                Err(e) => {
                    console_println!("❌ Boot sector read failed: {:?}", e);
                    return Err(FilesystemError::IoError);
                }
            }
        }
        
        console_println!("✅ FAT32 debug tests complete");
        Ok(())
    }
} 