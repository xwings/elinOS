// Inode management for ext2

use super::structures::*;
use super::superblock::SuperblockManager;
use super::super::traits::{FilesystemError, FilesystemResult};
use crate::console_println;
use heapless::Vec;

/// Manages ext2 inode operations
pub struct InodeManager {
    inode_size: u16,
    inodes_per_group: u32,
}

impl InodeManager {
    pub fn new() -> Self {
        Self {
            inode_size: 256, // Default ext2 inode size
            inodes_per_group: 0,
        }
    }
    
    /// Initialize inode manager with superblock info
    pub fn init(&mut self, sb_mgr: &SuperblockManager) -> FilesystemResult<()> {
        let sb = sb_mgr.get_superblock().ok_or(FilesystemError::InvalidSuperblock)?;
        
        self.inode_size = if sb.s_rev_level == 0 { 128 } else { sb.s_inode_size };
        self.inodes_per_group = sb.s_inodes_per_group;
        
        console_println!("📂 Inode manager initialized");
        console_println!("   Inode size: {} bytes", self.inode_size);
        console_println!("   Inodes per group: {}", self.inodes_per_group);
        
        Ok(())
    }
    
    /// Read an inode from disk
    pub fn read_inode(&self, inode_num: u32, sb_mgr: &SuperblockManager) -> FilesystemResult<Ext2Inode> {
        if inode_num == 0 {
            return Err(FilesystemError::InvalidPath);
        }
        
        // For simplicity, assume all inodes are in group 0
        let group_num = (inode_num - 1) / self.inodes_per_group;
        let local_inode_index = (inode_num - 1) % self.inodes_per_group;
        
        if group_num != 0 {
            // For now, only support group 0
            return Err(FilesystemError::FileNotFound);
        }
        
        // Get inode table location from group descriptor
        let group_desc = sb_mgr.get_group_descriptor()
            .ok_or(FilesystemError::InvalidSuperblock)?;
        let inode_table_block = group_desc.bg_inode_table_lo as u64;
        
        let block_size = sb_mgr.get_block_size();
        let inode_offset = local_inode_index as usize * self.inode_size as usize;
        let block_offset = inode_offset / block_size;
        let offset_in_block = inode_offset % block_size;
        
        // Read the block containing the inode
        let block_num = inode_table_block + block_offset as u64;
        
        // Debug inode reading calculation
        console_println!("🔧 Reading inode {} calculation:", inode_num);
        console_println!("   Group: {}, Local index: {}", group_num, local_inode_index);
        console_println!("   Inode table block: {}", inode_table_block);
        console_println!("   Inode size: {}, Block size: {}", self.inode_size, block_size);
        console_println!("   Inode offset: {}, Block offset: {}, Offset in block: {}", inode_offset, block_offset, offset_in_block);
        console_println!("   Reading from block: {}", block_num);
        
        let block_data = sb_mgr.read_block_data(block_num)?;
        
        // Extract inode from block
        if offset_in_block + (self.inode_size as usize) > block_data.len() {
            return Err(FilesystemError::CorruptedFilesystem);
        }
        
        let inode: Ext2Inode = unsafe {
            core::ptr::read(block_data[offset_in_block..].as_ptr() as *const Ext2Inode)
        };
        
        // Debug the actual inode data
        let raw_mode = inode.i_mode;
        let raw_size_lo = inode.i_size_lo;
        let raw_blocks_lo = inode.i_blocks_lo;
        let raw_block_0 = inode.i_block[0];
        
        console_println!("🔍 Raw inode {} data:", inode_num);
        console_println!("   Raw mode: 0x{:04x}", raw_mode);
        console_println!("   Raw size_lo: {}", raw_size_lo);
        console_println!("   Raw blocks_lo: {}", raw_blocks_lo);
        console_println!("   Raw block[0]: {}", raw_block_0);
        
        Ok(inode)
    }
    
    /// Write an inode to disk
    pub fn write_inode(&self, inode_num: u32, inode: &Ext2Inode, sb_mgr: &SuperblockManager) -> FilesystemResult<()> {
        if inode_num == 0 {
            return Err(FilesystemError::InvalidPath);
        }
        
        let group_num = (inode_num - 1) / self.inodes_per_group;
        let local_inode_index = (inode_num - 1) % self.inodes_per_group;
        
        if group_num != 0 {
            return Err(FilesystemError::FileNotFound);
        }
        
        // Get inode table location from group descriptor
        let group_desc = sb_mgr.get_group_descriptor()
            .ok_or(FilesystemError::InvalidSuperblock)?;
        let inode_table_block = group_desc.bg_inode_table_lo as u64;
        
        let block_size = sb_mgr.get_block_size();
        let inode_offset = local_inode_index as usize * self.inode_size as usize;
        let block_offset = inode_offset / block_size;
        let offset_in_block = inode_offset % block_size;
        
        let block_num = inode_table_block + block_offset as u64;
        let mut block_data = sb_mgr.read_block_data(block_num)?;
        
        // Update inode in block
        if offset_in_block + (self.inode_size as usize) > block_data.len() {
            return Err(FilesystemError::CorruptedFilesystem);
        }
        
        unsafe {
            core::ptr::copy_nonoverlapping(
                inode as *const Ext2Inode as *const u8,
                block_data[offset_in_block..].as_mut_ptr(),
                core::mem::size_of::<Ext2Inode>()
            );
        }
        
        sb_mgr.write_block_data(block_num as u32, &block_data)?;
        Ok(())
    }
    
    /// Allocate a new inode
    pub fn allocate_inode(&self, mode: u16, uid: u16, gid: u16, links_count: u16, flags: u32, sb_mgr: &SuperblockManager) -> FilesystemResult<u32> {
        // Find free inode
        let free_inode_num = self.find_free_inode(sb_mgr)?;
        
        // Mark inode as used in bitmap
        let group_desc = sb_mgr.get_group_descriptor()
            .ok_or(FilesystemError::InvalidSuperblock)?;
        let inode_bitmap_block = group_desc.bg_inode_bitmap_lo;
        let mut inode_bitmap_data = sb_mgr.read_block_data(inode_bitmap_block as u64)?;
        
        // Set the bit in the bitmap
        let bit_index = (free_inode_num - 1) as usize;
        let byte_index = bit_index / 8;
        let bit_in_byte_index = bit_index % 8;
        
        if byte_index < inode_bitmap_data.len() {
            inode_bitmap_data[byte_index] |= 1 << bit_in_byte_index;
            sb_mgr.write_block_data(inode_bitmap_block as u32, &inode_bitmap_data)?;
            console_println!("allocate_inode: Marked inode {} as used in bitmap.", free_inode_num);
        } else {
            return Err(FilesystemError::CorruptedFilesystem);
        }
        
        // Create new inode
        let new_inode = Ext2Inode::new(mode, uid, gid, links_count, flags);
        
        // Write inode to disk
        self.write_inode(free_inode_num, &new_inode, sb_mgr)?;
        
        console_println!("🆕 Created new inode {} with mode 0x{:04x}", free_inode_num, mode);
        Ok(free_inode_num)
    }
    
    /// Find a free inode using the actual bitmap
    fn find_free_inode(&self, sb_mgr: &SuperblockManager) -> FilesystemResult<u32> {
        let group_desc = sb_mgr.get_group_descriptor()
            .ok_or(FilesystemError::InvalidSuperblock)?;
        let sb = sb_mgr.get_superblock()
            .ok_or(FilesystemError::InvalidSuperblock)?;

        if group_desc.bg_free_inodes_count_lo == 0 {
            console_println!("find_free_inode: No free inodes in group 0 per descriptor.");
            return Err(FilesystemError::FilesystemFull);
        }

        let inode_bitmap_block = group_desc.bg_inode_bitmap_lo;
        console_println!("find_free_inode: Reading inode bitmap from block {}", inode_bitmap_block);
        let inode_bitmap_data = sb_mgr.read_block_data(inode_bitmap_block as u64)?;

        // Find free bit in bitmap
        for (byte_index, byte) in inode_bitmap_data.iter().enumerate() {
            if *byte != 0xFF { // If not all bits are 1, there's a 0 bit in this byte
                for bit_in_byte_index in 0..8 {
                    if (*byte & (1 << bit_in_byte_index)) == 0 {
                        let bit_index = byte_index * 8 + bit_in_byte_index;
                        if bit_index >= sb.s_inodes_per_group as usize {
                            continue; // Out of range for this group
                        }
                        let inode_num = bit_index as u32 + 1;
                        console_println!("find_free_inode: Found free inode bit {} -> inode num {}", bit_index, inode_num);
                        return Ok(inode_num);
                    }
                }
            }
        }
        
        console_println!("find_free_inode: No free bit found in inode bitmap for group 0.");
        Err(FilesystemError::FilesystemFull)
    }
    
    /// Free an inode
    pub fn free_inode(&self, inode_num: u32, sb_mgr: &SuperblockManager) -> FilesystemResult<()> {
        // Mark inode as free by setting dtime
        let mut inode = self.read_inode(inode_num, sb_mgr)?;
        inode.i_dtime = 1; // TODO: Use current time
        inode.i_links_count = 0;
        
        self.write_inode(inode_num, &inode, sb_mgr)?;
        
        // TODO: Update inode bitmap
        console_println!("🗑️  Freed inode {}", inode_num);
        Ok(())
    }
    
    /// Get file size from inode
    pub fn get_file_size(&self, inode: &Ext2Inode) -> usize {
        inode.get_size() as usize
    }
    
    /// Update file size in inode
    pub fn set_file_size(&self, inode: &mut Ext2Inode, size: u64) {
        inode.set_size(size);
    }
    
    /// Check if inode is a directory
    pub fn is_directory(&self, inode: &Ext2Inode) -> bool {
        inode.is_directory()
    }
    
    /// Check if inode is a regular file
    pub fn is_regular_file(&self, inode: &Ext2Inode) -> bool {
        inode.is_regular_file()
    }
    

} 