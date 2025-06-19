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
    root_dir_cluster: u32,
    sectors_per_cluster: u8,
    bytes_per_sector: u16,
    first_fat_sector: u64,
    sectors_per_fat: u32,
    data_region_start_sector: u64,
    total_data_clusters: u32,
    fs_info_sector_num: Option<u16>,
}

impl Fat32FileSystem {
    pub fn new() -> Self {
        Fat32FileSystem {
            boot_sector: None,
            files: Vec::new(),
            initialized: false,
            mounted: false,
            root_dir_cluster: 0,
            sectors_per_cluster: 0,
            bytes_per_sector: 0,
            first_fat_sector: 0,
            sectors_per_fat: 0,
            data_region_start_sector: 0,
            total_data_clusters: 0,
            fs_info_sector_num: None,
        }
    }
    
    /// Initialize the FAT32 filesystem
    pub fn init(&mut self) -> FilesystemResult<()> {
        console_println!("[i] Initializing FAT32 filesystem...");

        let mut disk_device = virtio_blk::VIRTIO_BLK.lock();
        
        if !disk_device.is_initialized() {
            return Err(FilesystemError::DeviceError);
        }

        let mut boot_buffer = [0u8; SECTOR_SIZE];
        disk_device.read_blocks(0, &mut boot_buffer)
            .map_err(|e| {
                console_println!("Error reading boot sector: {:?}", e);
                FilesystemError::IoError
            })?;
        
        let boot_sector: Fat32BootSector = unsafe {
            let bs_ptr = boot_buffer.as_ptr() as *const Fat32BootSector;
            *bs_ptr
        };

        let signature_val = boot_sector.signature; // local copy
        let bytes_per_sector_val = boot_sector.bytes_per_sector; // local copy

        if signature_val != FAT32_SIGNATURE {
            console_println!("Invalid FAT32 signature: expected 0x{:X}, got 0x{:X}", FAT32_SIGNATURE, signature_val);
            return Err(FilesystemError::InvalidBootSector);
        }
        if bytes_per_sector_val == 0 || !bytes_per_sector_val.is_power_of_two() || bytes_per_sector_val < 512 {
            console_println!("Invalid bytes_per_sector: {}", bytes_per_sector_val);
            return Err(FilesystemError::InvalidBootSector);
        }
        if boot_sector.sectors_per_cluster == 0 || !boot_sector.sectors_per_cluster.is_power_of_two() {
            console_println!("Invalid sectors_per_cluster: {}", boot_sector.sectors_per_cluster);
            return Err(FilesystemError::InvalidBootSector);
        }

        console_println!("[o] Valid FAT32 filesystem detected!");
        let total_sectors = if boot_sector.total_sectors_16 == 0 { boot_sector.total_sectors_32 } else { boot_sector.total_sectors_16 as u32 };
        let bytes_per_sector = boot_sector.bytes_per_sector;
        console_println!("   [i] {} total sectors", total_sectors);
        console_println!("   💾 {} bytes per sector", bytes_per_sector);
        console_println!("   🧱 {} sectors per cluster", boot_sector.sectors_per_cluster);

        self.boot_sector = Some(boot_sector);
        self.bytes_per_sector = bytes_per_sector;
        self.sectors_per_cluster = boot_sector.sectors_per_cluster;
        self.root_dir_cluster = boot_sector.root_cluster;
        self.first_fat_sector = boot_sector.reserved_sectors as u64;
        self.sectors_per_fat = boot_sector.sectors_per_fat_32;
        
        let fat_sectors = boot_sector.num_fats as u32 * self.sectors_per_fat;
        self.data_region_start_sector = self.first_fat_sector + fat_sectors as u64;
        
        let total_data_sectors = total_sectors - (boot_sector.reserved_sectors as u32 + fat_sectors);
        self.total_data_clusters = total_data_sectors / boot_sector.sectors_per_cluster as u32;

        if boot_sector.info_sector != 0 && boot_sector.info_sector != 0xFFFF {
            self.fs_info_sector_num = Some(boot_sector.info_sector);
        }
        
        console_println!("   [i]  Root cluster: {}", self.root_dir_cluster);
        console_println!("   [i]  FAT starts at sector: {}", self.first_fat_sector);
        console_println!("   [i]  Sectors per FAT: {}", self.sectors_per_fat);
        console_println!("   💾 Data region starts at sector: {}", self.data_region_start_sector);
        console_println!("   🧱 Total data clusters: {}", self.total_data_clusters);
        if let Some(fs_info) = self.fs_info_sector_num {
            console_println!("   [i]  FSInfo sector at: {}", fs_info);
        }

        self.initialized = true;
        
        drop(disk_device); 
        self.read_directory(self.root_dir_cluster)?; 
        self.mounted = true;

        console_println!("[o] FAT32 filesystem mounted");
        Ok(())
    }
    
    /// Helper function to calculate the sector for a given cluster
    fn cluster_to_sector(&self, cluster: u32) -> u64 {
        self.data_region_start_sector + ((cluster - 2) as u64 * self.sectors_per_cluster as u64)
    }
    
    /// Reads a FAT entry for a given cluster number.
    fn read_fat_entry(&self, cluster: u32) -> FilesystemResult<u32> {
        let boot_sector = self.boot_sector.ok_or(FilesystemError::NotInitialized)?;
        if cluster >= self.total_data_clusters + 2 { // FAT entries also exist for reserved clusters 0 and 1
            console_println!("read_fat_entry: Cluster {} out of bounds ({} total data clusters)", cluster, self.total_data_clusters);
            return Err(FilesystemError::InvalidFAT); // Or InvalidParameter
        }

        let fat_offset = cluster * 4; // Each FAT entry is 4 bytes for FAT32
        let entry_sector = self.first_fat_sector + (fat_offset / self.bytes_per_sector as u32) as u64;
        let entry_offset_in_sector = (fat_offset % self.bytes_per_sector as u32) as usize;

        // Ensure we don't read beyond the FAT
        if entry_sector >= self.first_fat_sector + self.sectors_per_fat as u64 {
            console_println!("read_fat_entry: Calculated sector {} is beyond FAT table (ends at {})", entry_sector, self.first_fat_sector + self.sectors_per_fat as u64 -1);
            return Err(FilesystemError::InvalidFAT);
        }

        let mut disk_device = virtio_blk::VIRTIO_BLK.lock();
        let mut sector_buffer = [0u8; SECTOR_SIZE]; // Assuming SECTOR_SIZE is self.bytes_per_sector
        
        // Adjust buffer size if bytes_per_sector can vary and is not SECTOR_SIZE
        // For now, we assume bytes_per_sector is consistently SECTOR_SIZE for read_blocks.
        if self.bytes_per_sector as usize != SECTOR_SIZE {
            // This case needs a dynamic buffer or virtio_blk to support variable block read sizes directly.
            // For FAT32, bytes_per_sector is usually 512.
            console_println!("read_fat_entry: Mismatch between SECTOR_SIZE ({}) and bs.bytes_per_sector ({})", SECTOR_SIZE, self.bytes_per_sector);
            return Err(FilesystemError::DeviceError);
        }

        disk_device.read_blocks(entry_sector, &mut sector_buffer)
            .map_err(|e| {
                console_println!("read_fat_entry: virtio_blk read_blocks error for sector {}: {:?}", entry_sector, e);
                FilesystemError::IoError
            })?;
        drop(disk_device);

        let entry_bytes = &sector_buffer[entry_offset_in_sector .. entry_offset_in_sector + 4];
        let value = u32::from_le_bytes(entry_bytes.try_into().unwrap());
        
        // Mask out the top 4 bits for FAT32, as they are reserved
        Ok(value & 0x0FFFFFFF)
    }

    /// Writes a FAT entry for a given cluster number.
    /// Note: This writes to the first FAT. A robust implementation would update all FATs.
    fn write_fat_entry(&mut self, cluster: u32, value: u32) -> FilesystemResult<()> {
        let boot_sector = self.boot_sector.ok_or(FilesystemError::NotInitialized)?;
        if cluster >= self.total_data_clusters + 2 { 
             console_println!("write_fat_entry: Cluster {} out of bounds", cluster);
            return Err(FilesystemError::InvalidFAT);
        }

        let fat_offset = cluster * 4;
        let entry_sector = self.first_fat_sector + (fat_offset / self.bytes_per_sector as u32) as u64;
        let entry_offset_in_sector = (fat_offset % self.bytes_per_sector as u32) as usize;

        if entry_sector >= self.first_fat_sector + self.sectors_per_fat as u64 {
            console_println!("write_fat_entry: Calculated sector {} is beyond FAT table", entry_sector);
            return Err(FilesystemError::InvalidFAT);
        }
        
        // Mask value to FAT32 constraints (top 4 bits are reserved)
        let masked_value = value & 0x0FFFFFFF;

        let mut disk_device = virtio_blk::VIRTIO_BLK.lock();
        let mut sector_buffer = [0u8; SECTOR_SIZE]; 

        if self.bytes_per_sector as usize != SECTOR_SIZE {
             console_println!("write_fat_entry: Mismatch between SECTOR_SIZE and bs.bytes_per_sector");
            return Err(FilesystemError::DeviceError);
        }

        // Read the existing sector first
        disk_device.read_blocks(entry_sector, &mut sector_buffer)
            .map_err(|e| {
                console_println!("write_fat_entry: virtio_blk read_blocks error for sector {}: {:?}", entry_sector, e);
                FilesystemError::IoError
            })?;

        // Modify the entry
        let entry_bytes = masked_value.to_le_bytes();
        sector_buffer[entry_offset_in_sector .. entry_offset_in_sector + 4].copy_from_slice(&entry_bytes);

        // Write the modified sector back
        disk_device.write_blocks(entry_sector, &sector_buffer)
            .map_err(|e| {
                console_println!("write_fat_entry: virtio_blk write_blocks error for sector {}: {:?}", entry_sector, e);
                FilesystemError::IoError
            })?;
        
        // TODO: Update FSInfo sector if free cluster count / next free cluster changes.
        // TODO: Update secondary FAT tables if they exist (boot_sector.num_fats > 1).
        // For now, only updating the primary FAT.

        drop(disk_device);
        Ok(())
    }
    
    /// Read and parse a directory from a given starting cluster
    fn read_directory(&mut self, start_cluster: u32) -> FilesystemResult<()> {
        console_println!("[i] Reading directory from cluster {}...", start_cluster);
        self.files.clear();

        let mut current_cluster = start_cluster;
        let mut disk_device = virtio_blk::VIRTIO_BLK.lock();
        let mut cluster_buffer = Vec::<u8, {SECTOR_SIZE * 8}>::new();
        
        let cluster_size_bytes = self.sectors_per_cluster as usize * self.bytes_per_sector as usize;
        if cluster_buffer.resize_default(cluster_size_bytes).is_err() {
            drop(disk_device);
            console_println!("Failed to allocate cluster buffer for directory reading.");
            return Err(FilesystemError::IoError);
        }

        let dir_sector = self.cluster_to_sector(current_cluster);
        
        for i in 0..self.sectors_per_cluster {
            let sector_to_read = dir_sector + i as u64;
            let buffer_offset = i as usize * self.bytes_per_sector as usize;
            let mut sector_slice = &mut cluster_buffer[buffer_offset .. buffer_offset + self.bytes_per_sector as usize];
            
            disk_device.read_blocks(sector_to_read, sector_slice.try_into().unwrap())
                .map_err(|e| {
                    console_println!("Error reading directory sector {}: {:?}", sector_to_read, e);
                    FilesystemError::IoError
                })?;
        }
        drop(disk_device);

        let mut offset = 0;
        let mut entries_found = 0;
        
        while offset + 32 <= cluster_size_bytes {
            let name_bytes = &cluster_buffer[offset..offset+8];
            let ext_bytes = &cluster_buffer[offset+8..offset+11];
            let attributes = cluster_buffer[offset+11];
            
            if name_bytes[0] == 0 { break; }
            if name_bytes[0] == 0xE5 { offset += 32; continue; }
            if attributes == 0x0F { offset += 32; continue; }
            if (attributes & 0x08) != 0 || (attributes & 0x04) != 0 { 
                offset += 32; 
                continue; 
            }
            
            let first_cluster_hi = u16::from_le_bytes([cluster_buffer[offset+20], cluster_buffer[offset+21]]);
            let first_cluster_lo = u16::from_le_bytes([cluster_buffer[offset+26], cluster_buffer[offset+27]]);
            let file_size = u32::from_le_bytes([
                cluster_buffer[offset+28], cluster_buffer[offset+29], 
                cluster_buffer[offset+30], cluster_buffer[offset+31]
            ]);
            
            let entry_cluster = ((first_cluster_hi as u32) << 16) | (first_cluster_lo as u32);
            
            let mut filename = String::<256>::new();
            for &byte in name_bytes.iter().take_while(|&&b| b != b' ' && b != 0) {
                if (filename.push(byte as char)).is_err() { break; }
            }
            
            if ext_bytes[0] != b' ' && ext_bytes[0] != 0 {
                if filename.push('.').is_ok() {
                    for &byte in ext_bytes.iter().take_while(|&&b| b != b' ' && b != 0) {
                        if (filename.push(byte as char)).is_err() { break; }
                    }
                }
            }
            
            let is_directory = (attributes & 0x10) != 0;
            
            let file_entry = if is_directory {
                FileEntry::new_directory(&filename, entry_cluster as u64)?
            } else {
                FileEntry::new_file(&filename, entry_cluster as u64, file_size as usize)?
            };
            
            console_println!("  Found {}: {} (cluster: {}, size: {})", 
                if is_directory { "DIR " } else { "FILE" },
                filename, entry_cluster, file_size);
            
            if self.files.push(file_entry).is_err() {
                console_println!("[!] File cache full while reading directory cluster {}", current_cluster);
                break; 
            }
            
            entries_found += 1;
            offset += 32;
        }
        
        console_println!("[o] Found {} entries in directory cluster {}", entries_found, start_cluster);
        Ok(())
    }
    
    /// Read file content from a cluster
    fn read_file_content(&self, file: &FileEntry) -> FilesystemResult<heapless::Vec<u8, 32768>> {
        let mut file_content = heapless::Vec::new();
        
        // For simplicity, read from the first cluster only
        // In a full implementation, we'd follow the cluster chain
        let cluster = file.inode as u32;
        
        // Calculate sector from cluster using existing fields
        let boot_sector = self.boot_sector.ok_or(FilesystemError::NotInitialized)?;
        let sectors_per_cluster = boot_sector.sectors_per_cluster as u32;
        let reserved_sectors = boot_sector.reserved_sectors as u32;
        let num_fats = boot_sector.num_fats as u32;
        let sectors_per_fat = boot_sector.sectors_per_fat_32;
        
        let data_start = reserved_sectors + (num_fats * sectors_per_fat);
        let sector = data_start + (cluster - 2) * sectors_per_cluster;
        
        // Read the sector
        let mut sector_buf = [0u8; SECTOR_SIZE];
        let mut disk_device = virtio_blk::VIRTIO_BLK.lock();
        if !disk_device.is_initialized() {
            return Err(FilesystemError::DeviceError);
        }
        disk_device.read_blocks(sector as u64, &mut sector_buf)
            .map_err(|_| FilesystemError::IoError)?;
        drop(disk_device);
        
        let safe_size = file.size.min(SECTOR_SIZE).min(32768);
        for i in 0..safe_size {
            let byte_val = sector_buf[i];
            if file_content.push(byte_val).is_err() {
                break; // Buffer full
            }
        }
        
        Ok(file_content)
    }
    
    fn find_file(&self, filename: &str) -> FilesystemResult<FileEntry> {
        for file in &self.files {
            if file.name.as_str() == filename && !file.is_directory {
                return Ok(file.clone());
            }
        }
        Err(FilesystemError::FileNotFound)
    }

    /// Finds the first available free cluster in the FAT.
    /// Optionally starts searching from `start_search_from_cluster` (cluster 2 is the first data cluster).
    fn find_free_cluster(&self, start_search_from_cluster: Option<u32>) -> FilesystemResult<u32> {
        // Start searching from cluster 2 (first usable cluster) or the provided hint.
        let start_cluster = start_search_from_cluster.unwrap_or(2).max(2);

        // Iterate up to total_data_clusters + 2 because cluster numbers are 2-based.
        for cluster_idx in start_cluster..(self.total_data_clusters + 2) {
            let fat_value = self.read_fat_entry(cluster_idx)?;
            if fat_value == 0x00000000 { // Found a free cluster
                return Ok(cluster_idx);
            }
        }
        // If no free cluster found from start_cluster to the end, try from cluster 2 if start_cluster was > 2
        if start_search_from_cluster.is_some() && start_cluster > 2 {
            for cluster_idx in 2..start_cluster {
                 let fat_value = self.read_fat_entry(cluster_idx)?;
                if fat_value == 0x00000000 { 
                    return Ok(cluster_idx);
                }
            }
        }

        console_println!("find_free_cluster: No free clusters found on the filesystem.");
        Err(FilesystemError::FilesystemFull)
    }

    /// Allocates a single new cluster, marks it as EOC, and optionally links it to a previous cluster.
    /// Returns the number of the newly allocated cluster.
    fn allocate_cluster(&mut self, previous_cluster_in_chain: Option<u32>) -> FilesystemResult<u32> {
        // Find a free cluster. We can pass a hint to potentially speed up subsequent allocations.
        // For a single allocation, the hint isn't as critical but could be the last allocated cluster.
        let new_cluster = self.find_free_cluster(previous_cluster_in_chain)?;

        // Mark the new cluster as End Of Chain (EOC)
        self.write_fat_entry(new_cluster, 0x0FFFFFFF)?; 
        // TODO: Update FSInfo if used (decrement free cluster count, set next free hint)

        // If this new cluster is part of a chain, update the previous cluster's FAT entry
        if let Some(prev_cluster) = previous_cluster_in_chain {
            self.write_fat_entry(prev_cluster, new_cluster)?;
        }

        // TODO: Zero out the newly allocated cluster on disk for security/consistency if required by spec or policy.
        // This can be slow. For now, we are not zeroing it.
        // let cluster_sector = self.cluster_to_sector(new_cluster);
        // let mut disk = virtio_blk::VIRTIO_BLK.lock();
        // let zero_sector = [0u8; SECTOR_SIZE]; // Assuming SECTOR_SIZE is bytes_per_sector
        // for i in 0..self.sectors_per_cluster {
        //     disk.write_blocks(cluster_sector + i as u64, &zero_sector)?;
        // }

        Ok(new_cluster)
    }

    /// Frees a chain of clusters starting from `start_cluster`.
    fn free_cluster_chain(&mut self, start_cluster: u32) -> FilesystemResult<()> {
        if start_cluster < 2 || start_cluster >= (self.total_data_clusters + 2) {
            console_println!("free_cluster_chain: Invalid start_cluster {}", start_cluster);
            return Err(FilesystemError::InvalidFAT); // Or InvalidParameter
        }

        let mut current_cluster = start_cluster;
        loop {
            let next_cluster_val = self.read_fat_entry(current_cluster)?;
            
            // Mark current cluster as free
            self.write_fat_entry(current_cluster, 0x00000000)?;
            // TODO: Update FSInfo if used (increment free cluster count)

            // Check if this was the EOC or an invalid/bad cluster
            if next_cluster_val >= 0x0FFFFFF8 || next_cluster_val == 0x00000000 || next_cluster_val == 0x0FFFFFF7 {
                break; // End of chain, already free, or bad cluster
            }
            if next_cluster_val < 2 || next_cluster_val >= (self.total_data_clusters + 2) {
                console_println!("free_cluster_chain: Invalid next cluster {} in chain from cluster {}", next_cluster_val, current_cluster);
                return Err(FilesystemError::CorruptedFilesystem); // Chain is broken
            }
            current_cluster = next_cluster_val;
        }
        Ok(())
    }

    /// Converts a Rust string (UTF-8) to a FAT 8.3 filename (ASCII, uppercase).
    // Returns an array of 11 bytes, 8 for name and 3 for extension, space-padded.
    // Limited character set for FAT 8.3.
    fn filename_to_fat_8_3(name_str: &str) -> FilesystemResult<[u8; 11]> {
        let mut fat_name = [0x20u8; 11]; // Initialize with spaces
        let mut name_part: String<8> = String::new();
        let mut ext_part: String<3> = String::new();
        let mut dot_seen = false;
        let mut is_extension_part = false;

        if name_str.is_empty() || name_str.len() > 12 { // Max 8.3 + dot
            return Err(FilesystemError::FilenameTooLong);
        }
        if name_str == "." || name_str == ".." { // Handle . and .. specially
            if name_str == "." {
                fat_name[0] = b'.';
            } else {
                fat_name[0] = b'.';
                fat_name[1] = b'.';
            }
            return Ok(fat_name);
        }


        for c in name_str.chars() {
            if c == '.' {
                if dot_seen { // Multiple dots
                    return Err(FilesystemError::FilenameTooLong); // Or InvalidCharacter
                }
                dot_seen = true;
                is_extension_part = true;
                continue;
            }

            if !c.is_ascii() || c == '/' || c == '\\' || "<>:\"|?*".contains(c) { // Invalid characters
                 return Err(FilesystemError::FilenameTooLong); // Or InvalidCharacter
            }
            
            let upper_c = c.to_ascii_uppercase();

            if is_extension_part {
                if ext_part.len() < 3 {
                    ext_part.push(upper_c).map_err(|_| FilesystemError::FilenameTooLong)?;
                } else {
                    // Extension too long
                    return Err(FilesystemError::FilenameTooLong); 
                }
            } else {
                if name_part.len() < 8 {
                    name_part.push(upper_c).map_err(|_| FilesystemError::FilenameTooLong)?;
                } else {
                    // Name part too long, and no dot seen yet to switch to extension
                    return Err(FilesystemError::FilenameTooLong);
                }
            }
        }
        
        if name_part.is_empty() && !ext_part.is_empty() { // like ".txt"
            return Err(FilesystemError::FilenameTooLong);
        }


        for (i, byte) in name_part.as_bytes().iter().enumerate() {
            fat_name[i] = *byte;
        }
        for (i, byte) in ext_part.as_bytes().iter().enumerate() {
            fat_name[8 + i] = *byte;
        }

        Ok(fat_name)
    }

    /// Finds an empty or reusable directory entry slot within a given directory cluster chain.
    /// Returns (cluster_of_entry, offset_in_cluster_bytes)
    /// TODO: Extend to allocate new cluster for directory if full.
    fn find_empty_directory_entry(&mut self, dir_start_cluster: u32) -> FilesystemResult<(u32, usize)> {
        let mut current_dir_cluster = dir_start_cluster;
        let cluster_size_bytes = self.sectors_per_cluster as usize * self.bytes_per_sector as usize;
        let mut cluster_buffer = Vec::<u8, {SECTOR_SIZE * 8}>::new(); // Max 8 sectors per cluster
        if cluster_buffer.resize_default(cluster_size_bytes).is_err() {
             console_println!("find_empty_directory_entry: Failed to allocate cluster buffer.");
            return Err(FilesystemError::IoError);
        }

        loop { // Loop through clusters in the directory chain
            let dir_sector_start = self.cluster_to_sector(current_dir_cluster);
            let mut disk = virtio_blk::VIRTIO_BLK.lock();
            for i in 0..self.sectors_per_cluster {
                let sector_to_read = dir_sector_start + i as u64;
                let buffer_offset = i as usize * self.bytes_per_sector as usize;
                let sector_slice = &mut cluster_buffer[buffer_offset..buffer_offset + self.bytes_per_sector as usize];
                disk.read_blocks(sector_to_read, sector_slice.try_into().unwrap())
                    .map_err(|_| FilesystemError::IoError)?;
            }
            drop(disk);

            let mut offset = 0;
            while offset + 32 <= cluster_size_bytes {
                let first_byte = cluster_buffer[offset];
                if first_byte == 0x00 || first_byte == 0xE5 { // Found an empty (0x00) or deleted (0xE5) slot
                    return Ok((current_dir_cluster, offset));
                }
                offset += 32; // Size of a directory entry
            }

            // If we reach here, the current cluster is full. Try to find the next cluster.
            let next_cluster_val = self.read_fat_entry(current_dir_cluster)?;
            if next_cluster_val >= 0x0FFFFFF8 { // EOC or Bad Cluster
                // Directory is full, and no more clusters.
                // TODO: Implement directory expansion: allocate new cluster, zero it, link it.
                console_println!("find_empty_directory_entry: Directory cluster {} is full, and no next cluster. Expansion needed.", current_dir_cluster);
                return Err(FilesystemError::FilesystemFull); // Placeholder until expansion is implemented
            }
            if next_cluster_val < 2 || next_cluster_val >= (self.total_data_clusters + 2) {
                 console_println!("find_empty_directory_entry: Invalid next cluster value {} for dir cluster {}", next_cluster_val, current_dir_cluster);
                return Err(FilesystemError::CorruptedFilesystem);
            }
            current_dir_cluster = next_cluster_val;
            // Loop again with the new current_dir_cluster
        }
    }

    /// Writes a Fat32DirEntry to a specific location (cluster and offset within that cluster).
    fn write_directory_entry(&mut self, dir_cluster_of_entry: u32, entry_offset_in_cluster: usize, dir_entry_data: &[u8; 32]) -> FilesystemResult<()> {
        if entry_offset_in_cluster % 32 != 0 || entry_offset_in_cluster + 32 > (self.sectors_per_cluster as usize * self.bytes_per_sector as usize) {
            console_println!("write_directory_entry: Invalid entry_offset_in_cluster {}", entry_offset_in_cluster);
            return Err(FilesystemError::IoError); // Or InvalidParameter
        }

        let target_sector_in_cluster = entry_offset_in_cluster / (self.bytes_per_sector as usize);
        let offset_in_sector = entry_offset_in_cluster % (self.bytes_per_sector as usize);

        let first_sector_of_dir_cluster = self.cluster_to_sector(dir_cluster_of_entry);
        let target_sector_on_disk = first_sector_of_dir_cluster + target_sector_in_cluster as u64;

        let mut disk = virtio_blk::VIRTIO_BLK.lock();
        let mut sector_buffer = [0u8; SECTOR_SIZE]; // Assuming SECTOR_SIZE matches self.bytes_per_sector

        // Read the sector
        disk.read_blocks(target_sector_on_disk, &mut sector_buffer)
            .map_err(|e| {
                console_println!("write_directory_entry: Failed to read sector {}: {:?}", target_sector_on_disk, e);
                FilesystemError::IoError
            })?;

        // Modify the 32-byte entry
        sector_buffer[offset_in_sector .. offset_in_sector + 32].copy_from_slice(dir_entry_data);

        // Write the sector back
        disk.write_blocks(target_sector_on_disk, &sector_buffer)
            .map_err(|e| {
                console_println!("write_directory_entry: Failed to write sector {}: {:?}", target_sector_on_disk, e);
                FilesystemError::IoError
            })?;
        
        drop(disk);
        Ok(())
    }
}

impl FileSystem for Fat32FileSystem {
    fn list_files(&self) -> FilesystemResult<Vec<(heapless::String<64>, usize), 32>> {
        let mut result_vec = Vec::new();
        for entry in self.files.iter() {
            // Convert entry.name (String<256>) to String<64>
            let name_str = heapless::String::<64>::try_from(entry.name.as_str())
                .map_err(|_| FilesystemError::FilenameTooLong)?;

            match result_vec.push((name_str, entry.size)) {
                Ok(_) => {},
                Err(_) => return Err(FilesystemError::Other(
                    heapless::String::try_from("List files result vec full").unwrap_or_default()
                ))
            }
        }
        Ok(result_vec)
    }
    
    fn list_directory(&self, path: &str) -> FilesystemResult<Vec<(heapless::String<64>, usize, bool), 32>> {
        // For now, FAT32 only supports root directory listing (like the original list_files)
        // TODO: Implement full path resolution for FAT32 directories
        console_println!("fat32: list_directory('{}') - Currently only supports root '/'", path);
        
        if path != "/" && path != "" {
            console_println!("fat32: list_directory - Path resolution not implemented for FAT32, defaulting to root");
        }
        
        let mut result_vec = Vec::new();
        for entry in self.files.iter() {
            // Convert entry.name (String<256>) to String<64>
            let name_str = heapless::String::<64>::try_from(entry.name.as_str())
                .map_err(|_| FilesystemError::FilenameTooLong)?;

            match result_vec.push((name_str, entry.size, entry.is_directory)) {
                Ok(_) => {},
                Err(_) => return Err(FilesystemError::Other(
                    heapless::String::try_from("List directory result vec full").unwrap_or_default()
                ))
            }
        }
        Ok(result_vec)
    }
    
    fn read_file(&self, filename: &str) -> FilesystemResult<heapless::Vec<u8, 32768>> {
        let file = self.find_file(filename)?;
        self.read_file_content(&file)
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

    fn read_file_to_buffer(&self, filename: &str, buffer: &mut [u8]) -> FilesystemResult<usize> {
        let content = self.read_file(filename)?;
        let bytes_to_copy = content.len().min(buffer.len());
        buffer[..bytes_to_copy].copy_from_slice(&content[..bytes_to_copy]);
        Ok(bytes_to_copy)
    }
    
    fn get_file_size(&self, filename: &str) -> FilesystemResult<usize> {
        let file = self.find_file(filename)?;
        Ok(file.size)
    }

    // == Write Operations ==

    fn create_file(&mut self, path: &str) -> FilesystemResult<FileEntry> {
        if !self.mounted {
            return Err(FilesystemError::NotMounted);
        }
        // For now, assume path is just a filename in the root directory.
        // TODO: Implement path parsing and directory traversal.
        let filename_str = path; // Simple assumption for now.

        // Check if file already exists (basic check in current loaded 'self.files' from read_directory)
        // This check needs to be more robust, scanning the actual directory on disk.
        for existing_file in self.files.iter() {
            if existing_file.name == filename_str && !existing_file.is_directory {
                return Err(FilesystemError::FileAlreadyExists);
            }
        }
        
        let fat_8_3_name = Self::filename_to_fat_8_3(filename_str)?;

        // Find an empty slot in the root directory
        // For now, creating files only in the root directory (self.root_dir_cluster)
        let (entry_dir_cluster, entry_offset) = self.find_empty_directory_entry(self.root_dir_cluster)?;

        // Allocate a cluster for the new file's data
        let file_start_cluster = self.allocate_cluster(None)?; // None because it's the start of a new chain

        // Create the directory entry
        // TODO: Get current date/time
        let current_time_fat = 0u16; // Placeholder
        let current_date_fat = 0u16; // Placeholder

        let dir_entry = Fat32DirEntry {
            name: [fat_8_3_name[0], fat_8_3_name[1], fat_8_3_name[2], fat_8_3_name[3], fat_8_3_name[4], fat_8_3_name[5], fat_8_3_name[6], fat_8_3_name[7]],
            ext: [fat_8_3_name[8], fat_8_3_name[9], fat_8_3_name[10]],
            attributes: 0x20, // Archive bit set, typical for new files
            _reserved: 0,
            creation_time_tenth: 0, // Placeholder
            creation_time: current_time_fat,
            creation_date: current_date_fat,
            last_access_date: current_date_fat,
            first_cluster_hi: (file_start_cluster >> 16) as u16,
            write_time: current_time_fat,
            write_date: current_date_fat,
            first_cluster_lo: (file_start_cluster & 0xFFFF) as u16,
            file_size: 0, // New file has size 0
        };

        // Write the directory entry to disk
        let dir_entry_bytes: [u8; 32] = unsafe { core::mem::transmute(dir_entry) };
        self.write_directory_entry(entry_dir_cluster, entry_offset, &dir_entry_bytes)?;

        // Create FileEntry for return and cache
        let file_entry = FileEntry::new_file(filename_str, file_start_cluster as u64, 0)?;
        
        // Add to in-memory cache (or re-read directory)
        // For simplicity, we'll just add it here. A robust implementation might re-read.
        if self.files.push(file_entry.clone()).is_err() {
            console_println!("create_file: Warning - in-memory file cache is full.");
        }

        console_println!("[o] File '{}' created starting at cluster {}, entry in dir_cluster {} at offset {}", 
            filename_str, file_start_cluster, entry_dir_cluster, entry_offset);

        Ok(file_entry)
    }

    fn create_directory(&mut self, path: &str) -> FilesystemResult<FileEntry> {
        if !self.mounted {
            return Err(FilesystemError::NotMounted);
        }
        // For now, assume path is just a directory name in the root directory.
        // TODO: Implement path parsing and parent directory traversal.
        let dirname_str = path;
        let parent_dir_cluster = self.root_dir_cluster; // Assuming creation in root

        console_println!("[i] Creating directory '{}' (in root dir cluster {})", dirname_str, parent_dir_cluster);

        // Check if file/dir already exists (basic check in current loaded 'self.files' from read_directory)
        // This needs to be more robust, scanning the actual directory on disk.
        for existing_entry in self.files.iter() {
            if existing_entry.name == dirname_str {
                console_println!("Error: '{}' already exists.", dirname_str);
                return Err(FilesystemError::FileAlreadyExists);
            }
        }

        let fat_8_3_name = Self::filename_to_fat_8_3(dirname_str)?;

        // Find an empty slot in the parent directory (e.g., root)
        let (entry_parent_dir_cluster, entry_offset_in_parent) = self.find_empty_directory_entry(parent_dir_cluster)?;

        // Allocate a cluster for the new directory's data
        let new_dir_start_cluster = self.allocate_cluster(None)?;
        console_println!("Allocated cluster {} for new directory '{}'", new_dir_start_cluster, dirname_str);

        // Zero out the new directory's cluster before writing . and .. entries
        let bytes_per_cluster = self.sectors_per_cluster as usize * self.bytes_per_sector as usize;
        let mut zeroed_cluster_data = Vec::<u8, {SECTOR_SIZE*8}>::new();
        if zeroed_cluster_data.resize_default(bytes_per_cluster).is_err() {
            // If this fails, we should ideally free new_dir_start_cluster. For now, error out.
            // self.free_cluster_chain(new_dir_start_cluster)?;
            return Err(FilesystemError::IoError);
        } 
        // Fill with zeros - resize_default should do this for Vec types that init with Default::default()
        // If not, an explicit loop: for byte in zeroed_cluster_data.iter_mut() { *byte = 0; }
        
        let new_dir_disk_sector_start = self.cluster_to_sector(new_dir_start_cluster);
        let mut disk_writer = virtio_blk::VIRTIO_BLK.lock();
        for i in 0..self.sectors_per_cluster {
            let sector_idx_in_cluster = i as usize;
            let data_slice_for_sector = &zeroed_cluster_data[sector_idx_in_cluster * self.bytes_per_sector as usize .. (sector_idx_in_cluster + 1) * self.bytes_per_sector as usize];
            disk_writer.write_blocks(new_dir_disk_sector_start + i as u64, data_slice_for_sector.try_into().unwrap())?;
        }
        drop(disk_writer);
        console_println!("Zeroed out cluster {} for new directory content", new_dir_start_cluster);

        // Create "." entry
        let dot_entry = Fat32DirEntry {
            name: [b'.', b' ', b' ', b' ', b' ', b' ', b' ', b' '],
            ext: [b' ', b' ', b' '],
            attributes: 0x10, // Directory
            _reserved: 0,
            creation_time_tenth: 0, // Placeholder for time
            creation_time: 0,       // Placeholder for time
            creation_date: 0,       // Placeholder for date
            last_access_date: 0,    // Placeholder for date
            first_cluster_hi: (new_dir_start_cluster >> 16) as u16,
            write_time: 0,          // Placeholder for time
            write_date: 0,          // Placeholder for date
            first_cluster_lo: (new_dir_start_cluster & 0xFFFF) as u16,
            file_size: 0,
        };
        let dot_entry_bytes: [u8; 32] = unsafe { core::mem::transmute(dot_entry) };
        self.write_directory_entry(new_dir_start_cluster, 0, &dot_entry_bytes)?;
        console_println!("Wrote '.' entry to new directory cluster {}", new_dir_start_cluster);

        // Create ".." entry
        // If parent_dir_cluster is 0 (e.g. root of a FAT12/16, though we are FAT32 so root_dir_cluster is usually >=2),
        // then the .. entry's cluster should also be 0. For FAT32, root dir does not have . or .. typically,
        // but subdirectories do. The cluster for ".." for a dir in root should point to root_dir_cluster.
        // If root_dir_cluster itself *is* the root of the filesystem (e.g. cluster 2 for FAT32), its parent is conceptually 0.
        // For a subdirectory, parent_dir_cluster will be its actual parent.
        let dot_dot_points_to_cluster = if parent_dir_cluster == self.root_dir_cluster && self.root_dir_cluster < 2 {
            0 // FAT12/16 root convention, or uninitialized parent
        } else {
            parent_dir_cluster
        };

        let dot_dot_entry = Fat32DirEntry {
            name: [b'.', b'.', b' ', b' ', b' ', b' ', b' ', b' '],
            ext: [b' ', b' ', b' '],
            attributes: 0x10, // Directory
            _reserved: 0,
            creation_time_tenth: 0,
            creation_time: 0,
            creation_date: 0,
            last_access_date: 0,
            first_cluster_hi: (dot_dot_points_to_cluster >> 16) as u16,
            write_time: 0,
            write_date: 0,
            first_cluster_lo: (dot_dot_points_to_cluster & 0xFFFF) as u16,
            file_size: 0,
        };
        let dot_dot_entry_bytes: [u8; 32] = unsafe { core::mem::transmute(dot_dot_entry) };
        self.write_directory_entry(new_dir_start_cluster, 32, &dot_dot_entry_bytes)?;
        console_println!("Wrote '..' entry (to cluster {}) to new directory cluster {}", dot_dot_points_to_cluster, new_dir_start_cluster);

        // Create the directory entry for the new directory itself in the parent directory
        let new_dir_in_parent_entry = Fat32DirEntry {
            name: [fat_8_3_name[0], fat_8_3_name[1], fat_8_3_name[2], fat_8_3_name[3], fat_8_3_name[4], fat_8_3_name[5], fat_8_3_name[6], fat_8_3_name[7]],
            ext: [fat_8_3_name[8], fat_8_3_name[9], fat_8_3_name[10]],
            attributes: 0x10, // Directory bit set
            _reserved: 0,
            creation_time_tenth: 0, 
            creation_time: 0,
            creation_date: 0,
            last_access_date: 0,
            first_cluster_hi: (new_dir_start_cluster >> 16) as u16,
            write_time: 0,
            write_date: 0,
            first_cluster_lo: (new_dir_start_cluster & 0xFFFF) as u16,
            file_size: 0, // Directories have 0 file size in their entry
        };
        let new_dir_in_parent_bytes: [u8; 32] = unsafe { core::mem::transmute(new_dir_in_parent_entry) };
        self.write_directory_entry(entry_parent_dir_cluster, entry_offset_in_parent, &new_dir_in_parent_bytes)?;
        console_println!("Wrote entry for new directory '{}' into parent cluster {} at offset {}", dirname_str, entry_parent_dir_cluster, entry_offset_in_parent);

        // Create FileEntry for return and cache
        let file_entry = FileEntry::new_directory(dirname_str, new_dir_start_cluster as u64)?;
        
        if self.files.push(file_entry.clone()).is_err() {
            console_println!("create_directory: Warning - in-memory file cache is full.");
        }

        console_println!("[o] Directory '{}' created starting at cluster {}", dirname_str, new_dir_start_cluster);
        Ok(file_entry)
    }

    fn write_file(&mut self, file: &FileEntry, offset: u64, data: &[u8]) -> FilesystemResult<usize> {
        if !self.mounted {
            return Err(FilesystemError::NotMounted);
        }
        if file.is_directory {
            console_println!("write_file: Cannot write to a directory: {}", file.name);
            return Err(FilesystemError::IoError); // Or a more specific error like IsADirectory
        }

        let bytes_per_cluster = self.sectors_per_cluster as usize * self.bytes_per_sector as usize;
        if bytes_per_cluster == 0 {
            return Err(FilesystemError::NotInitialized); // Should have been set during init
        }

        console_println!(
            "📝 Writing to file '{}' (start_cluster: {}), offset: {}, data_len: {}",
            file.name,
            file.inode, // inode is the start_cluster for FAT32
            offset,
            data.len()
        );

        if data.is_empty() {
            return Ok(0); // Nothing to write
        }

        // --- Simplified initial implementation: --- 
        // 1. Only supports offset = 0 or offset = file.size (append)
        // 2. Assumes file.inode is the first cluster. If 0, it means file is empty and needs allocation.
        // 3. Does not yet correctly handle seeking to arbitrary offsets within existing data or partial overwrites.

        let mut current_file_size = file.size as u64;
        let mut start_cluster = file.inode as u32;

        if offset != 0 && offset != current_file_size {
            console_println!("write_file: Arbitrary offsets not yet fully supported. Offset: {}, Size: {}", offset, current_file_size);
            // For now, only allow writing at the beginning of an empty file or appending.
            if start_cluster != 0 && offset != 0 { // If file has clusters, only append for this simplified version
                 return Err(FilesystemError::IoError); // Or UnsupportedOperation for arbitrary offset
            }
            if start_cluster == 0 && offset != 0 { // Cannot write at non-zero offset to an unallocated file.
                return Err(FilesystemError::IoError);
            }
        }

        let mut bytes_written_total = 0usize;
        let mut data_remaining_to_write = data;
        let mut current_logical_offset = offset;

        let mut current_cluster_in_chain = if start_cluster == 0 { // File is empty, needs first cluster
            if offset != 0 { return Err(FilesystemError::IoError); /* Cannot start write with offset on empty file */ }
            let new_first_cluster = self.allocate_cluster(None)?;
            // Update file.inode (start_cluster) - This needs to be reflected in the dir entry later!
            start_cluster = new_first_cluster;
            console_println!("Allocated first cluster {} for file '{}'", start_cluster, file.name);
            new_first_cluster // Start with the newly allocated cluster
        } else {
            start_cluster
        };
        
        let mut last_allocated_cluster = current_cluster_in_chain;

        // Navigate to the target cluster if offset > 0 (very simplified for append-like logic)
        // This part needs significant improvement for true random access.
        if offset > 0 && start_cluster != 0 {
            let mut clusters_to_skip = offset / bytes_per_cluster as u64;
            let mut cluster_offset_within_file = 0u64;
            
            // Find the last cluster in the existing chain if appending
            if offset == current_file_size {
                let mut temp_cluster = start_cluster;
                loop {
                    let fat_val = self.read_fat_entry(temp_cluster)?;
                    if fat_val >= 0x0FFFFFF8 { // EOC or bad cluster
                        current_cluster_in_chain = temp_cluster;
                        last_allocated_cluster = temp_cluster;
                        break;
                    }
                    temp_cluster = fat_val;
                    if temp_cluster < 2 || temp_cluster >= (self.total_data_clusters + 2) {
                        return Err(FilesystemError::CorruptedFilesystem);
                    }
                }
            } else {
                // True seeking is not yet here, this is a placeholder for a more complex seek
                return Err(FilesystemError::IoError); // Arbitrary offset seek not implemented
            }
        }

        let mut cluster_data_buffer = Vec::<u8, {SECTOR_SIZE * 8}>::new(); // Max 8 sectors for now
        if cluster_data_buffer.resize_default(bytes_per_cluster).is_err() {
            return Err(FilesystemError::IoError);
        }

        while !data_remaining_to_write.is_empty() {
            let offset_in_cluster = (current_logical_offset % bytes_per_cluster as u64) as usize;
            let bytes_to_write_in_cluster = core::cmp::min(
                data_remaining_to_write.len(),
                bytes_per_cluster - offset_in_cluster,
            );

            // If offset_in_cluster > 0 or bytes_to_write_in_cluster < bytes_per_cluster,
            // we need to read the existing cluster data first for a partial write.
            // For simplicity, if offset_in_cluster is not 0, we read. (Overwriting from start of cluster doesn't need read)
            if offset_in_cluster > 0 || (bytes_to_write_in_cluster < bytes_per_cluster && current_file_size > current_logical_offset) {
                let sector_start = self.cluster_to_sector(current_cluster_in_chain);
                let mut disk = virtio_blk::VIRTIO_BLK.lock();
                for i in 0..self.sectors_per_cluster {
                    let s_offset = i as usize * self.bytes_per_sector as usize;
                    disk.read_blocks(sector_start + i as u64, (&mut cluster_data_buffer[s_offset..s_offset + self.bytes_per_sector as usize]).try_into().unwrap())?;
                }
                drop(disk);
            }

            // Copy data to our cluster buffer
            cluster_data_buffer[offset_in_cluster..offset_in_cluster + bytes_to_write_in_cluster]
                .copy_from_slice(&data_remaining_to_write[0..bytes_to_write_in_cluster]);

            // Write the whole cluster back to disk
            let cluster_disk_sector = self.cluster_to_sector(current_cluster_in_chain);
            let mut disk_writer = virtio_blk::VIRTIO_BLK.lock();
            for i in 0..self.sectors_per_cluster {
                let sector_idx_in_cluster = i as usize;
                let data_slice_for_sector = &cluster_data_buffer[sector_idx_in_cluster * self.bytes_per_sector as usize .. (sector_idx_in_cluster + 1) * self.bytes_per_sector as usize];
                disk_writer.write_blocks(cluster_disk_sector + i as u64, data_slice_for_sector.try_into().unwrap())?;
            }
            drop(disk_writer);

            bytes_written_total += bytes_to_write_in_cluster;
            data_remaining_to_write = &data_remaining_to_write[bytes_to_write_in_cluster..];
            current_logical_offset += bytes_to_write_in_cluster as u64;

            if !data_remaining_to_write.is_empty() {
                // Need another cluster
                let next_cluster_fat_val = self.read_fat_entry(current_cluster_in_chain)?;
                if next_cluster_fat_val >= 0x0FFFFFF8 { // Current cluster was EOC or bad
                    let new_next_cluster = self.allocate_cluster(Some(current_cluster_in_chain))?;
                    current_cluster_in_chain = new_next_cluster;
                    last_allocated_cluster = new_next_cluster;
                    console_println!("Extended file '{}', new cluster: {}", file.name, new_next_cluster);
                } else {
                    current_cluster_in_chain = next_cluster_fat_val;
                    if current_cluster_in_chain < 2 || current_cluster_in_chain >= (self.total_data_clusters + 2) {
                        return Err(FilesystemError::CorruptedFilesystem);
                    }
                }
            }
        }
        
        // Update file size in directory entry
        let new_size = core::cmp::max(current_file_size, offset + bytes_written_total as u64);
        // Find the directory entry on disk and update it.
        // This is a critical step that needs the path of the file to find its directory entry.
        // For now, we assume `file` argument's name can be used to find it in root dir if that was its location.
        // This is a major simplification!
        let dir_of_file_start_cluster = self.root_dir_cluster; // Assuming root for now
        // We need to find the *actual* directory entry on disk to update its size and first_cluster if it was newly allocated.
        // This requires iterating through the directory containing the file.
        // Placeholder for robust directory entry update:
        console_println!("TODO: Update directory entry for '{}' with new size {} and start_cluster {}", file.name, new_size, start_cluster);
        // For a PoC, if you knew the `FileEntry` in `self.files` was the one, you could update it.
        // But `file` is a shared ref. We need to update the on-disk entry.

        // Let's try to update the directory entry assuming it's in the root directory.
        // This is still simplified as it doesn't handle subdirectories properly.
        let mut dir_cluster_iter = dir_of_file_start_cluster;
        let mut found_and_updated_dirent = false;
        let fat_8_3_name_to_find = Self::filename_to_fat_8_3(&file.name)?;

        // Loop to find the directory entry
        // This loop is very basic and needs to be robust (like read_directory but for finding one entry)
        'dirent_update_loop: loop {
            let current_dir_sector_start = self.cluster_to_sector(dir_cluster_iter);
            let mut temp_cluster_buffer = Vec::<u8, {SECTOR_SIZE * 8}>::new();
            if temp_cluster_buffer.resize_default(bytes_per_cluster).is_err() { return Err(FilesystemError::IoError); }
            let mut disk = virtio_blk::VIRTIO_BLK.lock();
            for i in 0..self.sectors_per_cluster {
                let s_offset = i as usize * self.bytes_per_sector as usize;
                 disk.read_blocks(current_dir_sector_start + i as u64, (&mut temp_cluster_buffer[s_offset..s_offset + self.bytes_per_sector as usize]).try_into().unwrap())?;
            }
            drop(disk);

            let mut entry_offset_in_cluster_bytes = 0;
            while entry_offset_in_cluster_bytes + 32 <= bytes_per_cluster {
                let potential_match = &temp_cluster_buffer[entry_offset_in_cluster_bytes..entry_offset_in_cluster_bytes+11];
                if potential_match[0] == 0x00 { break 'dirent_update_loop; } // End of directory
                if potential_match[0] == 0xE5 { entry_offset_in_cluster_bytes += 32; continue; } // Deleted
                
                if potential_match == &fat_8_3_name_to_find[..] {
                    // Found it. Now, reconstruct the Fat32DirEntry, update, and write back.
                    let mut entry_data_bytes: [u8; 32] = temp_cluster_buffer[entry_offset_in_cluster_bytes..entry_offset_in_cluster_bytes+32].try_into().unwrap();
                    let mut dir_entry_to_update: Fat32DirEntry = unsafe { core::mem::transmute_copy(&entry_data_bytes) };

                    dir_entry_to_update.file_size = new_size as u32;
                    dir_entry_to_update.first_cluster_lo = (start_cluster & 0xFFFF) as u16;
                    dir_entry_to_update.first_cluster_hi = (start_cluster >> 16) as u16;
                    // TODO: Update write time/date
                    
                    let updated_bytes: [u8; 32] = unsafe { core::mem::transmute(dir_entry_to_update) };
                    self.write_directory_entry(dir_cluster_iter, entry_offset_in_cluster_bytes, &updated_bytes)?;
                    found_and_updated_dirent = true;
                    console_println!("Updated directory entry for '{}' with size: {}, start_cluster: {}", file.name, new_size, start_cluster);
                    break 'dirent_update_loop;
                }
                entry_offset_in_cluster_bytes += 32;
            }
            let next_fat = self.read_fat_entry(dir_cluster_iter)?;
            if next_fat >= 0x0FFFFFF8 { break; } // EOC
            dir_cluster_iter = next_fat;
        }

        if !found_and_updated_dirent {
            console_println!("write_file: Failed to find and update directory entry for '{}' after write.", file.name);
            // This is an error state, as the file data is written but its metadata isn't updated.
        }

        // Update in-memory cache if the file object passed was from there (tricky due to borrowing rules)
        // Best to re-read the directory or update the specific entry if a mutable reference to it was held.
        // For now, user of this function might need to invalidate/refresh their FileEntry object.

        Ok(bytes_written_total)
    }

    fn delete_file(&mut self, path: &str) -> FilesystemResult<()> {
        if !self.mounted {
            return Err(FilesystemError::NotMounted);
        }
        // For now, assume path is just a filename in the root directory.
        // TODO: Implement path parsing and proper directory traversal.
        let filename_str = path;

        console_println!("🗑️ Deleting file '{}' (assumed in root)", filename_str);

        let fat_8_3_name_to_find = Self::filename_to_fat_8_3(filename_str)?;
        let dir_start_cluster = self.root_dir_cluster;

        let mut current_dir_cluster = dir_start_cluster;
        let bytes_per_cluster = self.sectors_per_cluster as usize * self.bytes_per_sector as usize;
        let mut temp_cluster_buffer = Vec::<u8, {SECTOR_SIZE * 8}>::new();
        if temp_cluster_buffer.resize_default(bytes_per_cluster).is_err() {
            return Err(FilesystemError::IoError);
        }

        loop { // Iterate through directory clusters
            let current_dir_sector_start = self.cluster_to_sector(current_dir_cluster);
            let mut disk = virtio_blk::VIRTIO_BLK.lock();
            for i in 0..self.sectors_per_cluster {
                let s_offset = i as usize * self.bytes_per_sector as usize;
                disk.read_blocks(current_dir_sector_start + i as u64, (&mut temp_cluster_buffer[s_offset..s_offset + self.bytes_per_sector as usize]).try_into().unwrap())?;
            }
            drop(disk);

            let mut entry_offset_in_cluster_bytes = 0;
            while entry_offset_in_cluster_bytes + 32 <= bytes_per_cluster {
                let entry_name_slice = &temp_cluster_buffer[entry_offset_in_cluster_bytes..entry_offset_in_cluster_bytes+11];
                
                if entry_name_slice[0] == 0x00 { // End of directory, file not found in this chain
                    console_println!("File '{}' not found for deletion.", filename_str);
                    return Err(FilesystemError::FileNotFound);
                }
                if entry_name_slice[0] == 0xE5 { // Deleted entry, skip
                    entry_offset_in_cluster_bytes += 32;
                    continue;
                }

                if entry_name_slice == &fat_8_3_name_to_find[..] {
                    // Found the file's directory entry.
                    let attributes = temp_cluster_buffer[entry_offset_in_cluster_bytes + 11];
                    if (attributes & 0x10) != 0 { // It's a directory
                        console_println!("Attempted to delete directory '{}' using delete_file.", filename_str);
                        return Err(FilesystemError::IoError); // Or IsADirectoryError
                    }

                    let first_cluster_hi = u16::from_le_bytes([temp_cluster_buffer[entry_offset_in_cluster_bytes+20], temp_cluster_buffer[entry_offset_in_cluster_bytes+21]]);
                    let first_cluster_lo = u16::from_le_bytes([temp_cluster_buffer[entry_offset_in_cluster_bytes+26], temp_cluster_buffer[entry_offset_in_cluster_bytes+27]]);
                    let file_start_cluster = ((first_cluster_hi as u32) << 16) | (first_cluster_lo as u32);

                    // Step 1: Free the cluster chain used by the file
                    if file_start_cluster >= 2 { // Only free if it points to a valid cluster area
                        console_println!("Freeing cluster chain starting at {} for file '{}'", file_start_cluster, filename_str);
                        self.free_cluster_chain(file_start_cluster)?;
                    } else {
                        console_println!("File '{}' has no allocated clusters (start_cluster: {}), nothing to free in FAT.", filename_str, file_start_cluster);
                    }

                    // Step 2: Mark the directory entry as deleted (0xE5)
                    let target_sector_in_cluster = entry_offset_in_cluster_bytes / (self.bytes_per_sector as usize);
                    let offset_in_sector = entry_offset_in_cluster_bytes % (self.bytes_per_sector as usize);
                    let target_sector_on_disk = self.cluster_to_sector(current_dir_cluster) + target_sector_in_cluster as u64;

                    // Re-read sector to modify, to avoid using potentially stale temp_cluster_buffer if it spanned sectors
                    // More robustly, self.write_directory_entry could be adapted or a new helper made.
                    let mut sector_to_modify_buffer = [0u8; SECTOR_SIZE];
                    let mut disk_modifier = virtio_blk::VIRTIO_BLK.lock();
                    disk_modifier.read_blocks(target_sector_on_disk, &mut sector_to_modify_buffer)?;
                    sector_to_modify_buffer[offset_in_sector] = 0xE5; // Mark as deleted
                    disk_modifier.write_blocks(target_sector_on_disk, &sector_to_modify_buffer)?;
                    drop(disk_modifier);
                    
                    console_println!("File '{}' marked as deleted in directory cluster {}, offset {}.", filename_str, current_dir_cluster, entry_offset_in_cluster_bytes);

                    // Step 3: Update in-memory cache
                    self.files.retain(|f| f.name != filename_str);
                    return Ok(());
                }
                entry_offset_in_cluster_bytes += 32;
            }

            // If here, entry not found in this cluster. Go to next cluster in directory chain.
            let next_fat_val = self.read_fat_entry(current_dir_cluster)?;
            if next_fat_val >= 0x0FFFFFF8 { // EOC for directory
                break;
            }
            if next_fat_val < 2 || next_fat_val >= (self.total_data_clusters + 2) {
                console_println!("delete_file: Corrupted directory chain for dir cluster {}. Next was {}", current_dir_cluster, next_fat_val);
                return Err(FilesystemError::CorruptedFilesystem);
            }
            current_dir_cluster = next_fat_val;
        }

        console_println!("File '{}' not found for deletion after checking all dir clusters.", filename_str);
        Err(FilesystemError::FileNotFound)
    }

    fn delete_directory(&mut self, path: &str) -> FilesystemResult<()> {
        if !self.mounted {
            return Err(FilesystemError::NotMounted);
        }
        // For now, assume path is just a directory name in the root directory.
        let dirname_str = path;
        let parent_dir_cluster = self.root_dir_cluster; // Assuming deletion from root

        console_println!("🗑️ Deleting directory '{}' (assumed in root)", dirname_str);

        let fat_8_3_name_to_find = Self::filename_to_fat_8_3(dirname_str)?;
        
        let mut current_parent_dir_cluster = parent_dir_cluster;
        let bytes_per_cluster = self.sectors_per_cluster as usize * self.bytes_per_sector as usize;
        let mut parent_cluster_buffer = Vec::<u8, {SECTOR_SIZE * 8}>::new();
        if parent_cluster_buffer.resize_default(bytes_per_cluster).is_err() { return Err(FilesystemError::IoError); }

        // Find the directory entry in the parent directory
        loop { // Iterate through parent directory clusters
            let parent_dir_sector_start = self.cluster_to_sector(current_parent_dir_cluster);
            let mut disk = virtio_blk::VIRTIO_BLK.lock();
            for i in 0..self.sectors_per_cluster {
                let s_offset = i as usize * self.bytes_per_sector as usize;
                disk.read_blocks(parent_dir_sector_start + i as u64, (&mut parent_cluster_buffer[s_offset..s_offset + self.bytes_per_sector as usize]).try_into().unwrap())?;
            }
            drop(disk);

            let mut entry_offset_in_parent_bytes = 0;
            while entry_offset_in_parent_bytes + 32 <= bytes_per_cluster {
                let entry_name_slice = &parent_cluster_buffer[entry_offset_in_parent_bytes..entry_offset_in_parent_bytes+11];
                
                if entry_name_slice[0] == 0x00 { // End of parent directory
                    console_println!("Directory '{}' not found for deletion.", dirname_str);
                    return Err(FilesystemError::DirectoryNotFound);
                }
                if entry_name_slice[0] == 0xE5 { // Deleted entry, skip
                    entry_offset_in_parent_bytes += 32;
                    continue;
                }

                if entry_name_slice == &fat_8_3_name_to_find[..] {
                    // Found the directory's entry in its parent.
                    let attributes = parent_cluster_buffer[entry_offset_in_parent_bytes + 11];
                    if (attributes & 0x10) == 0 { // It's a file, not a directory
                        console_println!("Attempted to delete file '{}' using delete_directory.", dirname_str);
                        return Err(FilesystemError::IoError); // Or NotADirectoryError
                    }

                    let first_cluster_hi = u16::from_le_bytes([parent_cluster_buffer[entry_offset_in_parent_bytes+20], parent_cluster_buffer[entry_offset_in_parent_bytes+21]]);
                    let first_cluster_lo = u16::from_le_bytes([parent_cluster_buffer[entry_offset_in_parent_bytes+26], parent_cluster_buffer[entry_offset_in_parent_bytes+27]]);
                    let dir_to_delete_start_cluster = ((first_cluster_hi as u32) << 16) | (first_cluster_lo as u32);

                    if dir_to_delete_start_cluster < 2 { // Should point to a valid cluster
                         console_println!("Directory '{}' has invalid start cluster {}.", dirname_str, dir_to_delete_start_cluster);
                        return Err(FilesystemError::CorruptedFilesystem);
                    }

                    // Check if the directory is empty (only contains . and ..)
                    let mut target_dir_cluster_buffer = Vec::<u8, {SECTOR_SIZE*8}>::new();
                    if target_dir_cluster_buffer.resize_default(bytes_per_cluster).is_err() { return Err(FilesystemError::IoError);}
                    
                    let mut target_dir_current_cluster = dir_to_delete_start_cluster;
                    let mut entry_count_in_target_dir = 0;
                    
                    // Loop to read the directory to be deleted
                    'emptiness_check: loop {
                        let target_dir_sector_start = self.cluster_to_sector(target_dir_current_cluster);
                        let mut disk_reader = virtio_blk::VIRTIO_BLK.lock();
                        for i in 0..self.sectors_per_cluster {
                            let s_offset = i as usize * self.bytes_per_sector as usize;
                            disk_reader.read_blocks(target_dir_sector_start + i as u64, (&mut target_dir_cluster_buffer[s_offset..s_offset + self.bytes_per_sector as usize]).try_into().unwrap())?;
                        }
                        drop(disk_reader);

                        let mut offset_in_target_dir = 0;
                        while offset_in_target_dir + 32 <= bytes_per_cluster {
                            let name_byte_one = target_dir_cluster_buffer[offset_in_target_dir];
                            if name_byte_one == 0x00 { break 'emptiness_check; } // End of this directory cluster
                            if name_byte_one != 0xE5 { // Not a deleted entry
                                // Check if it's NOT . or ..
                                let current_entry_name = &target_dir_cluster_buffer[offset_in_target_dir .. offset_in_target_dir+11];
                                let is_dot = current_entry_name == b".          ";
                                let is_dot_dot = current_entry_name == b"..         ";
                                if !is_dot && !is_dot_dot {
                                    console_println!("Directory '{}' is not empty. Found entry: {:?}", dirname_str, core::str::from_utf8(current_entry_name).unwrap_or("non-utf8 name"));
                                    return Err(FilesystemError::DirectoryNotFound); // Standard says EACCES or EEXIST or ENOTEMPTY. Let's use DirectoryNotFound as a proxy for NotEmpty
                                }
                                entry_count_in_target_dir +=1;
                            }
                            offset_in_target_dir += 32;
                        }
                        let next_target_dir_fat = self.read_fat_entry(target_dir_current_cluster)?;
                        if next_target_dir_fat >= 0x0FFFFFF8 { break; } // EOC for target dir
                        target_dir_current_cluster = next_target_dir_fat;
                        if target_dir_current_cluster < 2 || target_dir_current_cluster >= (self.total_data_clusters + 2) {
                             return Err(FilesystemError::CorruptedFilesystem);
                        }
                    }
                    // After checking all clusters, if entry_count_in_target_dir > 2 (for . and ..), it's not empty.
                    // This logic is a bit off, the check inside the loop is better.
                    // If we exited the loop due to finding a non . or .. entry, the error was already returned.
                    // If we exited due to EOD, and only found . and .., it's empty.

                    // Free the cluster chain used by the directory itself
                    console_println!("Freeing cluster chain starting at {} for directory '{}'", dir_to_delete_start_cluster, dirname_str);
                    self.free_cluster_chain(dir_to_delete_start_cluster)?;

                    // Mark the directory entry in its parent as deleted (0xE5)
                    let target_sector_in_parent_cluster = entry_offset_in_parent_bytes / (self.bytes_per_sector as usize);
                    let offset_in_parent_sector = entry_offset_in_parent_bytes % (self.bytes_per_sector as usize);
                    let target_sector_on_disk = self.cluster_to_sector(current_parent_dir_cluster) + target_sector_in_parent_cluster as u64;
                    
                    let mut sector_to_modify_buffer = [0u8; SECTOR_SIZE];
                    let mut disk_modifier = virtio_blk::VIRTIO_BLK.lock();
                    disk_modifier.read_blocks(target_sector_on_disk, &mut sector_to_modify_buffer)?;
                    sector_to_modify_buffer[offset_in_parent_sector] = 0xE5; // Mark as deleted
                    disk_modifier.write_blocks(target_sector_on_disk, &sector_to_modify_buffer)?;
                    drop(disk_modifier);
                    
                    console_println!("Directory '{}' marked as deleted in parent dir cluster {}, offset {}.", dirname_str, current_parent_dir_cluster, entry_offset_in_parent_bytes);

                    self.files.retain(|f| f.name != dirname_str);
                    return Ok(());
                }
                entry_offset_in_parent_bytes += 32;
            }

            let next_parent_dir_fat = self.read_fat_entry(current_parent_dir_cluster)?;
            if next_parent_dir_fat >= 0x0FFFFFF8 { break; } // EOC for parent directory
             if next_parent_dir_fat < 2 || next_parent_dir_fat >= (self.total_data_clusters + 2) {
                return Err(FilesystemError::CorruptedFilesystem);
            }
            current_parent_dir_cluster = next_parent_dir_fat;
        }

        console_println!("Directory '{}' not found for deletion after checking all parent dir clusters.", dirname_str);
        Err(FilesystemError::DirectoryNotFound)
    }
    
    fn truncate_file(&mut self, file: &FileEntry, new_size: u64) -> FilesystemResult<()> {
        if !self.mounted {
            return Err(FilesystemError::NotMounted);
        }
        if file.is_directory {
            console_println!("truncate_file: Cannot truncate a directory: {}", file.name);
            return Err(FilesystemError::IoError); // Or IsADirectoryError
        }

        console_println!(
            "✂️ Truncating file '{}' (start_cluster: {}) from size {} to new_size {}",
            file.name,
            file.inode, // inode is start_cluster
            file.size,
            new_size
        );

        let current_size = file.size as u64;
        let start_cluster = file.inode as u32;
        let bytes_per_cluster = self.sectors_per_cluster as usize * self.bytes_per_sector as usize;

        if new_size == current_size {
            return Ok(()); // No change needed
        }

        if start_cluster < 2 && current_size > 0 { // File has size but no valid cluster - inconsistent
            console_println!("truncate_file: File '{}' has size {} but invalid start cluster {}", file.name, current_size, start_cluster);
            return Err(FilesystemError::CorruptedFilesystem);
        }
        if start_cluster < 2 && new_size == 0 && current_size == 0 { // Truncating an already empty, unallocated file to 0
             return Ok(());
        }
        if start_cluster < 2 && new_size > 0 { // Trying to expand a file that never had clusters
            // This scenario should be handled by write_file allocating initial cluster.
            // For truncate, if it's 0->N, it implies allocation + zeroing.
            // Let's treat this as needing initial allocation if expanding from 0 size & 0 cluster.
            if current_size == 0 { 
                let mut first_cluster = self.allocate_cluster(None)?;
                // Update the file's start cluster - This is tricky as `file` is immutable.
                // The directory entry needs update. For now, assume caller handles refreshing FileEntry.
                console_println!("truncate_file: Allocated initial cluster {} for '{}' (expanding from 0)", first_cluster, file.name);
                // Now proceed to expand further if new_size > bytes_per_cluster
                // This part merges with the expansion logic below.
                // For simplicity, we'll update the start_cluster for internal logic, but dir entry update is key.
                let mut temp_file = file.clone(); // Yuk, but for logic flow with immutable input
                temp_file.inode = first_cluster as u64;
                temp_file.size = 0; // Start from 0 for expansion logic
                return self.truncate_file(&temp_file, new_size); // Recursive call with updated temp FileEntry
            } else {
                return Err(FilesystemError::CorruptedFilesystem); // Should not happen if current_size > 0
            }
        }

        if new_size < current_size { // Shrinking
            console_println!("Shrinking file '{}'", file.name);
            if new_size == 0 {
                // Shrinking to zero means freeing all clusters and setting start_cluster to 0 in dir entry.
                if start_cluster >= 2 {
                    self.free_cluster_chain(start_cluster)?;
                }
                // Update directory entry: size=0, start_cluster=0
                // This requires finding the dir entry, like in write_file/delete_file.
                // TODO: Robustly update directory entry.
                 console_println!("TODO: Update dir entry for '{}' to size 0, cluster 0 after shrink to 0", file.name);
            } else {
                let clusters_needed = (new_size + bytes_per_cluster as u64 - 1) / bytes_per_cluster as u64;
                let mut current_cluster_in_chain = start_cluster;
                let mut clusters_traversed = 1u64;

                while clusters_traversed < clusters_needed {
                    let next_cluster_val = self.read_fat_entry(current_cluster_in_chain)?;
                    if next_cluster_val >= 0x0FFFFFF8 { // EOC or bad cluster unexpectedly
                        console_println!("truncate (shrink): Unexpected EOC for '{}' at cluster {} while needing {} clusters.", file.name, current_cluster_in_chain, clusters_needed);
                        return Err(FilesystemError::CorruptedFilesystem);
                    }
                    current_cluster_in_chain = next_cluster_val;
                    if current_cluster_in_chain < 2 || current_cluster_in_chain >= (self.total_data_clusters + 2) {
                        return Err(FilesystemError::CorruptedFilesystem);
                    }
                    clusters_traversed += 1;
                }

                // current_cluster_in_chain is now the new last cluster of the file.
                // Mark it as EOC, then free the rest of the chain.
                let next_cluster_to_free = self.read_fat_entry(current_cluster_in_chain)?;
                self.write_fat_entry(current_cluster_in_chain, 0x0FFFFFFF)?; // Mark as EOC

                if next_cluster_to_free < 0x0FFFFFF8 && next_cluster_to_free >=2 {
                    console_println!("truncate (shrink): Freeing chain for '{}' starting from cluster {}", file.name, next_cluster_to_free);
                    self.free_cluster_chain(next_cluster_to_free)?;
                }
                // TODO: Update directory entry with new_size.
                 console_println!("TODO: Update dir entry for '{}' to size {} after shrink", file.name, new_size);
            }
        } else { // Expanding (new_size > current_size)
            console_println!("Expanding file '{}'", file.name);
            let mut current_cluster_in_chain = start_cluster; // Should be valid (>=2) due to checks above or initial alloc
            let mut current_logical_size = current_size;

            // Traverse to the end of the current chain if file is not empty
            if current_size > 0 {
                loop {
                    let fat_val = self.read_fat_entry(current_cluster_in_chain)?;
                    if fat_val >= 0x0FFFFFF8 { // EOC or bad cluster
                        break;
                    }
                    current_cluster_in_chain = fat_val;
                     if current_cluster_in_chain < 2 || current_cluster_in_chain >= (self.total_data_clusters + 2) {
                        return Err(FilesystemError::CorruptedFilesystem);
                    }
                }
            } else if start_cluster < 2 { // File was size 0 and no cluster, should have been handled by initial alloc logic
                 console_println!("truncate_file (expand): File '{}' has size 0 but no initial cluster after alloc attempt.", file.name);
                return Err(FilesystemError::CorruptedFilesystem); // Should not be reached if initial alloc worked
            }
            // current_cluster_in_chain is now the last allocated cluster for the file.

            let mut cluster_data_buffer = Vec::<u8, {SECTOR_SIZE * 8}>::new();
            if cluster_data_buffer.resize_default(bytes_per_cluster).is_err() { return Err(FilesystemError::IoError); }
            // Ensure it's zeroed for writing zero bytes
            for byte in cluster_data_buffer.iter_mut() { *byte = 0; }

            // Zero out the remaining part of the current last cluster if new_size starts within it.
            let offset_in_last_cluster = current_size % bytes_per_cluster as u64;
            if offset_in_last_cluster > 0 && new_size > current_size {
                let bytes_to_zero_in_cluster = core::cmp::min(bytes_per_cluster as u64 - offset_in_last_cluster, new_size - current_size) as usize;
                if bytes_to_zero_in_cluster > 0 {
                    console_println!("Zeroing {} bytes in cluster {} for '{}'", bytes_to_zero_in_cluster, current_cluster_in_chain, file.name);
                    // Read-modify-write the cluster (only the part that needs zeroing)
                    // This is simplified; a full RMW of the sector containing the range is safer.
                    // For now, just zero the tail of a temp buffer and write that part.
                    // This part is complex to do correctly without full cluster RMW. Assume we write full clusters.
                    // Let's zero out the relevant sectors for simplicity of this function
                    let disk_cluster_sector_start = self.cluster_to_sector(current_cluster_in_chain);
                    let mut disk = virtio_blk::VIRTIO_BLK.lock();
                    // This needs careful calculation of which sectors and what range within them to zero.
                    // Simplified: If extending within a cluster, the write_file logic would handle it better.
                    // Here, we assume we are adding *new* fully zeroed clusters, or zeroing from a certain point.
                    // The current logic just allocates new clusters and they should be zeroed.
                }
            }

            while current_logical_size < new_size {
                let new_allocated_cluster = self.allocate_cluster(Some(current_cluster_in_chain))?;
                console_println!("truncate (expand): Allocated new cluster {} for '{}'", new_allocated_cluster, file.name);
                
                // Zero out the newly allocated cluster
                let new_cluster_sector_start = self.cluster_to_sector(new_allocated_cluster);
                let mut disk_writer = virtio_blk::VIRTIO_BLK.lock();
                for i in 0..self.sectors_per_cluster {
                    let sector_idx_in_cluster = i as usize;
                    // Use the pre-zeroed cluster_data_buffer
                    let zero_sector_slice = &cluster_data_buffer[0..self.bytes_per_sector as usize];
                    disk_writer.write_blocks(new_cluster_sector_start + i as u64, zero_sector_slice.try_into().unwrap())?;
                }
                drop(disk_writer);
                console_println!("Zeroed new cluster {}", new_allocated_cluster);

                current_cluster_in_chain = new_allocated_cluster;
                current_logical_size += bytes_per_cluster as u64; 
                // If current_logical_size overshoots new_size, it's fine, file is allocated by cluster.
            }
            // TODO: Update directory entry with new_size and potentially new start_cluster if it was 0.
            console_println!("TODO: Update dir entry for '{}' to size {} after expand", file.name, new_size);
        }
        
        // Generic directory entry update (Placeholder - needs robust implementation)
        // This should use the *original file.name* and find its entry to update size and start_cluster if changed.
        let mut dir_cluster_iter = self.root_dir_cluster; // Assuming root for now
        let mut found_and_updated_dirent = false;
        let fat_8_3_name_to_find = Self::filename_to_fat_8_3(&file.name)?;

        'dirent_update_loop: loop {
            let current_dir_sector_start = self.cluster_to_sector(dir_cluster_iter);
            let mut temp_cluster_buffer = Vec::<u8, {SECTOR_SIZE * 8}>::new();
            if temp_cluster_buffer.resize_default(bytes_per_cluster).is_err() { return Err(FilesystemError::IoError); }
            let mut disk = virtio_blk::VIRTIO_BLK.lock();
            for i in 0..self.sectors_per_cluster {
                let s_offset = i as usize * self.bytes_per_sector as usize;
                 disk.read_blocks(current_dir_sector_start + i as u64, (&mut temp_cluster_buffer[s_offset..s_offset + self.bytes_per_sector as usize]).try_into().unwrap())?;
            }
            drop(disk);

            let mut entry_offset_in_cluster_bytes = 0;
            while entry_offset_in_cluster_bytes + 32 <= bytes_per_cluster {
                let potential_match_name = &temp_cluster_buffer[entry_offset_in_cluster_bytes..entry_offset_in_cluster_bytes+11];
                if potential_match_name[0] == 0x00 { break 'dirent_update_loop; } 
                if potential_match_name[0] == 0xE5 { entry_offset_in_cluster_bytes += 32; continue; }
                
                if potential_match_name == &fat_8_3_name_to_find[..] {
                    let mut entry_data_bytes: [u8; 32] = temp_cluster_buffer[entry_offset_in_cluster_bytes..entry_offset_in_cluster_bytes+32].try_into().unwrap();
                    let mut dir_entry_to_update: Fat32DirEntry = unsafe { core::mem::transmute_copy(&entry_data_bytes) };

                    dir_entry_to_update.file_size = new_size as u32;
                    if new_size == 0 { // If truncated to 0, standard practice is to set cluster to 0.
                        dir_entry_to_update.first_cluster_lo = 0;
                        dir_entry_to_update.first_cluster_hi = 0;
                    } else if file.inode == 0 && start_cluster >=2 { // Was expanded from 0 size/0 cluster, update start cluster
                        dir_entry_to_update.first_cluster_lo = (start_cluster & 0xFFFF) as u16;
                        dir_entry_to_update.first_cluster_hi = (start_cluster >> 16) as u16;
                    }
                    // TODO: Update write time/date
                    
                    let updated_bytes: [u8; 32] = unsafe { core::mem::transmute(dir_entry_to_update) };
                    self.write_directory_entry(dir_cluster_iter, entry_offset_in_cluster_bytes, &updated_bytes)?;
                    found_and_updated_dirent = true;
                    console_println!("Updated directory entry for '{}' with size: {}, original start_cluster: {}, new_start_cluster_in_dirent: {}", 
                        file.name, new_size, file.inode, (dir_entry_to_update.first_cluster_hi as u32) << 16 | dir_entry_to_update.first_cluster_lo as u32);
                    break 'dirent_update_loop;
                }
                entry_offset_in_cluster_bytes += 32;
            }
            let next_fat = self.read_fat_entry(dir_cluster_iter)?;
            if next_fat >= 0x0FFFFFF8 { break; } 
            dir_cluster_iter = next_fat;
             if dir_cluster_iter < 2 || dir_cluster_iter >= (self.total_data_clusters + 2) {
                return Err(FilesystemError::CorruptedFilesystem);
            }
        }
        if !found_and_updated_dirent {
             console_println!("truncate_file: Failed to find/update dirent for '{}'", file.name);
        }

        Ok(())
    }

    fn sync(&mut self) -> FilesystemResult<()> {
        console_println!("Fat32FileSystem: sync operation called.");
        Ok(())
    }
}

/// FAT32-specific test functions for debugging
impl Fat32FileSystem {
    /// Test function for debugging FAT32 filesystem operations
    /// This contains the low-level tests that were previously in main.rs
    pub fn run_debug_tests() -> FilesystemResult<()> {
        use crate::{console_println, virtio_blk};
        
        console_println!("[i] Running FAT32 debug tests...");
        
        // Test 1: Read boot sector
        console_println!("[i] Test 1: Reading boot sector...");
        {
            let mut disk_device = virtio_blk::VIRTIO_BLK.lock();
            let mut buffer = [0u8; 512];
            match disk_device.read_blocks(0, &mut buffer) {
                Ok(()) => {
                    console_println!("[o] Boot sector read successful");
                    
                    // Parse key FAT32 fields
                    let sectors_per_cluster = buffer[13];
                    let reserved_sectors = u16::from_le_bytes([buffer[14], buffer[15]]);
                    let num_fats = buffer[16];
                    let sectors_per_fat = u32::from_le_bytes([buffer[36], buffer[37], buffer[38], buffer[39]]);
                    let root_cluster = u32::from_le_bytes([buffer[44], buffer[45], buffer[46], buffer[47]]);
                    
                    console_println!("[i] FAT32 Boot Sector Analysis:");
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
                    console_println!("[i] Test 2: Reading root directory...");
                    match disk_device.read_blocks(root_sector as u64, &mut buffer) {
                        Ok(()) => {
                            console_println!("[o] Root directory read successful");
                            console_println!("[i] First 32 bytes: {:02x?}", &buffer[0..32]);
                            
                            // Look for directory entries
                            if buffer[0] != 0 && buffer[0] != 0xE5 {
                                console_println!("[o] Found directory entry!");
                                let name_bytes = &buffer[0..8];
                                let ext_bytes = &buffer[8..11];
                                console_println!("  Name: {:?}", name_bytes);
                                console_println!("  Ext: {:?}", ext_bytes);
                                console_println!("  Attributes: 0x{:02x}", buffer[11]);
                            }
                        }
                        Err(e) => {
                            console_println!("[x] Failed to read root directory: {:?}", e);
                            return Err(FilesystemError::IoError);
                        }
                    }
                }
                Err(e) => {
                    console_println!("[x] Boot sector read failed: {:?}", e);
                    return Err(FilesystemError::IoError);
                }
            }
        }
        
        console_println!("[o] FAT32 debug tests complete");
        Ok(())
    }
} 