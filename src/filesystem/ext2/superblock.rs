// Superblock management for ext2

use super::structures::*;
use super::super::traits::{FilesystemError, FilesystemResult};
use crate::{console_println, virtio_blk};
use heapless::Vec;

/// Manages ext2 superblock operations
pub struct SuperblockManager {
    superblock: Option<Ext2Superblock>,
    group_desc: Option<Ext2GroupDesc>,
    block_size: usize,
}

impl SuperblockManager {
    pub fn new() -> Self {
        Self {
            superblock: None,
            group_desc: None,
            block_size: 1024, // Default ext2 block size
        }
    }
    
    /// Initialize superblock and group descriptor
    pub fn init(&mut self) -> FilesystemResult<()> {
        self.read_superblock()?;
        self.read_group_descriptor()?;
        Ok(())
    }
    
    /// Read and validate superblock from disk
    fn read_superblock(&mut self) -> FilesystemResult<()> {
        console_println!("ℹ️ Reading ext2 superblock...");
        
        let mut disk_device = virtio_blk::VIRTIO_BLK.lock();
        
        if !disk_device.is_initialized() {
            return Err(FilesystemError::DeviceError);
        }

        // Read superblock sectors (1024 bytes starting at offset 1024)
        let start_sector = EXT2_SUPERBLOCK_OFFSET / SECTOR_SIZE; // sector 2
        let mut sb_buffer = [0u8; 1024];
        
        // Read 2 sectors to get full superblock
        for i in 0..2 {
            let current_sector = (start_sector + i) as u64;
            let mut sector_buf = [0u8; SECTOR_SIZE];
            
            disk_device.read_blocks(current_sector, &mut sector_buf)
                .map_err(|_| FilesystemError::IoError)?;
            
            sb_buffer[i * SECTOR_SIZE..(i + 1) * SECTOR_SIZE].copy_from_slice(&sector_buf);
        }
        
        drop(disk_device);
        
        // Parse superblock
        let sb: Ext2Superblock = unsafe { core::ptr::read(sb_buffer.as_ptr() as *const Ext2Superblock) };
        
        // Copy values from packed struct to avoid reference issues
        let magic = sb.s_magic;
        let log_block_size = sb.s_log_block_size;
        let total_blocks = sb.s_blocks_count_lo;
        let total_inodes = sb.s_inodes_count;
        
        // Validate magic number
        if magic != EXT2_MAGIC {
            console_println!("❌ Invalid ext2 magic: 0x{:X}, expected 0x{:X}", magic, EXT2_MAGIC);
            return Err(FilesystemError::InvalidSuperblock);
        }
        
        // Calculate block size
        self.block_size = 1024 << log_block_size;
        
        console_println!("✅ Valid ext2 superblock found!");
        console_println!("   Block size: {} bytes", self.block_size);
        console_println!("   Total blocks: {}", total_blocks);
        console_println!("   Total inodes: {}", total_inodes);
        
        self.superblock = Some(sb);
        Ok(())
    }
    
    /// Read group descriptor
    fn read_group_descriptor(&mut self) -> FilesystemResult<()> {
        console_println!("ℹ️ Reading group descriptor...");
        
        let _sb = self.superblock.as_ref().ok_or(FilesystemError::InvalidSuperblock)?;
        
        // Group descriptor is in the block after superblock
        let gd_block = if self.block_size == 1024 { 2 } else { 1 };
        let gd_data = self.read_block_data(gd_block)?;
        
        // Parse first group descriptor
        let gd: Ext2GroupDesc = unsafe { core::ptr::read(gd_data.as_ptr() as *const Ext2GroupDesc) };
        
        // Copy values from packed struct to avoid reference issues
        let block_bitmap = gd.bg_block_bitmap_lo;
        let inode_bitmap = gd.bg_inode_bitmap_lo;
        let inode_table = gd.bg_inode_table_lo;
        
        console_println!("✅ Group descriptor loaded");
        console_println!("   Block bitmap: {}", block_bitmap);
        console_println!("   Inode bitmap: {}", inode_bitmap);
        console_println!("   Inode table: {}", inode_table);
        
        self.group_desc = Some(gd);
        Ok(())
    }
    
    /// Read a block from disk
    pub fn read_block_data(&self, block_num: u64) -> FilesystemResult<Vec<u8, 4096>> {
        let mut disk_device = virtio_blk::VIRTIO_BLK.lock();
        
        if !disk_device.is_initialized() {
            return Err(FilesystemError::DeviceError);
        }
        
        let sectors_per_block = self.block_size / SECTOR_SIZE;
        let start_sector = block_num * (sectors_per_block as u64);
        
        let mut block_data = Vec::new();
        
        for i in 0..sectors_per_block {
            let sector = start_sector + (i as u64);
            let mut sector_buf = [0u8; SECTOR_SIZE];
            
            disk_device.read_blocks(sector, &mut sector_buf)
                .map_err(|_| FilesystemError::IoError)?;
            
            for byte in sector_buf.iter() {
                block_data.push(*byte).map_err(|_| FilesystemError::FilesystemFull)?;
            }
        }
        
        drop(disk_device);
        Ok(block_data)
    }
    
    /// Write a block to disk
    pub fn write_block_data(&self, block_num: u32, data: &[u8]) -> FilesystemResult<()> {
        let mut disk_device = virtio_blk::VIRTIO_BLK.lock();
        
        if !disk_device.is_initialized() {
            return Err(FilesystemError::DeviceError);
        }
        
        let sectors_per_block = self.block_size / SECTOR_SIZE;
        let start_sector = (block_num as u64) * (sectors_per_block as u64);
        
        for i in 0..sectors_per_block {
            let sector = start_sector + (i as u64);
            let sector_start = i * SECTOR_SIZE;
            let sector_end = core::cmp::min(sector_start + SECTOR_SIZE, data.len());
            
            let mut sector_buf = [0u8; SECTOR_SIZE];
            
            if sector_end > sector_start {
                let copy_len = sector_end - sector_start;
                sector_buf[..copy_len].copy_from_slice(&data[sector_start..sector_end]);
            }
            
            disk_device.write_blocks(sector, &sector_buf)
                .map_err(|_| FilesystemError::IoError)?;
        }
        
        drop(disk_device);
        Ok(())
    }
    
    /// Write superblock to disk
    pub fn write_superblock(&mut self, sb: &Ext2Superblock) -> FilesystemResult<()> {
        let mut sb_buffer = [0u8; 1024];
        
        // Copy superblock to buffer
        unsafe {
            core::ptr::copy_nonoverlapping(
                sb as *const Ext2Superblock as *const u8,
                sb_buffer.as_mut_ptr(),
                core::mem::size_of::<Ext2Superblock>()
            );
        }
        
        let mut disk_device = virtio_blk::VIRTIO_BLK.lock();
        
        if !disk_device.is_initialized() {
            return Err(FilesystemError::DeviceError);
        }
        
        let start_sector = EXT2_SUPERBLOCK_OFFSET / SECTOR_SIZE;
        
        // Write 2 sectors
        for i in 0..2 {
            let sector = (start_sector + i) as u64;
            let sector_start = i * SECTOR_SIZE;
            let sector_buf = &sb_buffer[sector_start..sector_start + SECTOR_SIZE];
            
            let mut write_buf = [0u8; SECTOR_SIZE];
            write_buf.copy_from_slice(sector_buf);
            
            disk_device.write_blocks(sector, &write_buf)
                .map_err(|_| FilesystemError::IoError)?;
        }
        
        drop(disk_device);
        self.superblock = Some(*sb);
        Ok(())
    }
    
    /// Write group descriptor to disk
    pub fn write_group_descriptor(&mut self, gd: &Ext2GroupDesc) -> FilesystemResult<()> {
        let gd_block = if self.block_size == 1024 { 2 } else { 1 };
        
        let mut gd_data = [0u8; 4096];
        let data_len = core::cmp::min(self.block_size, 4096);
        
        // Copy group descriptor to buffer
        unsafe {
            core::ptr::copy_nonoverlapping(
                gd as *const Ext2GroupDesc as *const u8,
                gd_data.as_mut_ptr(),
                core::mem::size_of::<Ext2GroupDesc>()
            );
        }
        
        self.write_block_data(gd_block, &gd_data[..data_len])?;
        self.group_desc = Some(*gd);
        Ok(())
    }
    
    /// Getters
    pub fn get_superblock(&self) -> Option<&Ext2Superblock> {
        self.superblock.as_ref()
    }
    
    pub fn get_group_descriptor(&self) -> Option<&Ext2GroupDesc> {
        self.group_desc.as_ref()
    }
    
    pub fn get_block_size(&self) -> usize {
        self.block_size
    }
    
    /// Update superblock counters
    pub fn update_free_blocks(&mut self, delta: i32) -> FilesystemResult<()> {
        if let Some(ref mut sb) = self.superblock {
            if delta < 0 && sb.s_free_blocks_count_lo < (-delta) as u32 {
                return Err(FilesystemError::FilesystemFull);
            }
            sb.s_free_blocks_count_lo = (sb.s_free_blocks_count_lo as i32 + delta) as u32;
        }
        Ok(())
    }
    
    pub fn update_free_inodes(&mut self, delta: i32) -> FilesystemResult<()> {
        if let Some(ref mut sb) = self.superblock {
            if delta < 0 && sb.s_free_inodes_count < (-delta) as u32 {
                return Err(FilesystemError::FilesystemFull);
            }
            sb.s_free_inodes_count = (sb.s_free_inodes_count as i32 + delta) as u32;
        }
        Ok(())
    }
    
    /// Allocate a new block (simplified implementation)
    pub fn allocate_block(&mut self) -> FilesystemResult<u32> {
        // For now, use a simple incrementing counter starting from block 1000
        // In a real implementation, you'd check the block bitmap
        
        static mut NEXT_BLOCK: u32 = 1000;
        
        unsafe {
            let block_num = NEXT_BLOCK;
            NEXT_BLOCK += 1;
            
            // Simple validation - don't exceed reasonable limits
            if block_num > 100000 {
                return Err(FilesystemError::FilesystemFull);
            }
            
            console_println!("🧱 Allocated block {}", block_num);
            Ok(block_num)
        }
    }
    
    /// Sync superblock to disk
    pub fn sync(&mut self) -> FilesystemResult<()> {
        if let Some(sb) = self.superblock {
            self.write_superblock(&sb)?;
        }
        if let Some(gd) = self.group_desc {
            self.write_group_descriptor(&gd)?;
        }
        Ok(())
    }
} 