// Block management for ext2

use super::structures::*;
use super::superblock::SuperblockManager;
use super::super::traits::{FilesystemError, FilesystemResult};
use crate::console_println;
use heapless::Vec;

/// ext2 inode flags
const EXT2_EXTENTS_FL: u32 = 0x00080000;  // Inode uses extents

/// Extent magic number
const EXT2_EXT_MAGIC: u16 = 0xF30A;

/// Manages ext2 block operations
pub struct BlockManager {
}

impl BlockManager {
    pub fn new() -> Self {
        Self {}
    }
    
    pub fn init(&mut self, _sb_mgr: &SuperblockManager) -> FilesystemResult<()> {
        console_println!("🧱 Block manager initialized");
        Ok(())
    }
    
    pub fn read_file_content(&self, inode: &Ext2Inode, file_size: usize, sb_mgr: &SuperblockManager) -> FilesystemResult<Vec<u8, 4096>> {
        console_println!("📖 Reading file content of size {}", file_size);
        
        // Copy flags and first block to avoid packed field issues
        let i_flags = inode.i_flags;
        let i_blocks_lo = inode.i_blocks_lo;
        let first_block = inode.i_block[0];
        
        console_println!("   🔍 File details: size={}, blocks={}, first_block={}, uses_extents={}", 
            file_size, i_blocks_lo, first_block, (i_flags & EXT2_EXTENTS_FL) != 0);
        
        // If file size is 0 but blocks are allocated, there might be content to read
        // This can happen if the file was created but size wasn't updated properly
        let effective_size = if file_size == 0 && i_blocks_lo > 0 && first_block != 0 {
            console_println!("   ⚠️  File size is 0 but blocks are allocated, trying to read actual content");
            sb_mgr.get_block_size() // Read at least one block to see if there's content
        } else if file_size == 0 {
            console_println!("   ℹ️  File is truly empty (size=0, no blocks allocated)");
            return Ok(Vec::new());
        } else {
            file_size
        };
        
        // Check if this inode uses extents
        if (i_flags & EXT2_EXTENTS_FL) != 0 {
            console_println!("   🌟 File uses extents - reading from extent tree");
            self.read_file_content_from_extents(inode, effective_size, sb_mgr)
        } else {
            console_println!("   📋 File uses direct blocks - reading from traditional blocks");
            self.read_file_content_from_blocks(inode, effective_size, sb_mgr)
        }
    }
    
    /// Read file content from extent-based inode
    fn read_file_content_from_extents(&self, inode: &Ext2Inode, file_size: usize, sb_mgr: &SuperblockManager) -> FilesystemResult<Vec<u8, 4096>> {
        let mut file_content = Vec::new();
        let mut bytes_read = 0;
        
        // Copy i_block array to avoid packed field alignment issues
        let i_block_copy = inode.i_block;
        
        // Parse extent header
        let extent_header: Ext2ExtentHeader = unsafe {
            let header_ptr = i_block_copy.as_ptr() as *const Ext2ExtentHeader;
            *header_ptr
        };
        
        let eh_magic = extent_header.eh_magic;
        let eh_entries = extent_header.eh_entries;
        let eh_depth = extent_header.eh_depth;
        
        console_println!("   🔍 File extent header: magic=0x{:04x}, entries={}, depth={}", 
            eh_magic, eh_entries, eh_depth);
        
        if eh_magic != EXT2_EXT_MAGIC {
            console_println!("   ❌ Invalid extent magic for file");
            return Err(FilesystemError::CorruptedFilesystem);
        }
        
        if eh_depth != 0 {
            console_println!("   ❌ Multi-level extent trees not supported for files");
            return Err(FilesystemError::UnsupportedFilesystem);
        }
        
        // Parse extent entries
        let extents_start = core::mem::size_of::<Ext2ExtentHeader>();
        let i_block_bytes = unsafe {
            core::slice::from_raw_parts(
                i_block_copy.as_ptr() as *const u8,
                60  // i_block is 15 * 4 = 60 bytes
            )
        };
        
        for i in 0..eh_entries as usize {
            if bytes_read >= file_size {
                break;
            }
            
            let extent_offset = extents_start + i * core::mem::size_of::<Ext2Extent>();
            
            if extent_offset + core::mem::size_of::<Ext2Extent>() > i_block_bytes.len() {
                console_println!("   ⚠️ File extent {} extends beyond i_block", i);
                break;
            }
            
            let extent: Ext2Extent = unsafe {
                let extent_ptr = (i_block_bytes.as_ptr().add(extent_offset)) as *const Ext2Extent;
                *extent_ptr
            };
            
            let ee_block = extent.ee_block;
            let ee_len = extent.ee_len;
            let ee_start_hi = extent.ee_start_hi;
            let ee_start_lo = extent.ee_start_lo;
            
            // Calculate physical block number
            let physical_block = ((ee_start_hi as u64) << 32) | (ee_start_lo as u64);
            
            console_println!("   🔍 File extent {}: logical={}, len={}, physical={}", 
                i, ee_block, ee_len, physical_block);
            
            // Read file data from this extent
            for block_offset in 0..ee_len {
                if bytes_read >= file_size {
                    break;
                }
                
                let block_num = physical_block + block_offset as u64;
                
                console_println!("   📍 Reading file block {} (from extent)", block_num);
                
                let block_data = match sb_mgr.read_block_data(block_num) {
                    Ok(data) => data,
                    Err(_) => {
                        console_println!("   ⚠️ Failed to read file extent block {}", block_num);
                        continue;
                    }
                };
                
                let bytes_to_copy = core::cmp::min(file_size - bytes_read, block_data.len());
                
                for i in 0..bytes_to_copy {
                    if file_content.push(block_data[i]).is_err() {
                        console_println!("   ⚠️ File content buffer full");
                        return Ok(file_content);
                    }
                    bytes_read += 1;
                }
            }
        }
        
        console_println!("   ✅ Read {} bytes from extent-based file", bytes_read);
        Ok(file_content)
    }
    
    /// Read file content from traditional direct block pointers
    fn read_file_content_from_blocks(&self, inode: &Ext2Inode, file_size: usize, sb_mgr: &SuperblockManager) -> FilesystemResult<Vec<u8, 4096>> {
        let mut file_content = Vec::new();
        let mut bytes_read = 0;
        
        // Copy i_block array to avoid packed field alignment issues
        let i_block_copy = inode.i_block;
        
        console_println!("   📋 Reading from direct blocks, target size: {}", file_size);
        console_println!("   🔍 First 5 block numbers: {:?}", &i_block_copy[..5]);
        
        // Read file data from direct blocks
        for (i, &block_num) in i_block_copy.iter().take(12).enumerate() {
            console_println!("   📍 Block {}: {}", i, block_num);
            
            if block_num == 0 {
                console_println!("   ⚠️  Block {} is 0, stopping", i);
                break;
            }
            
            if bytes_read >= file_size {
                console_println!("   ✅ Read enough bytes ({}), stopping", bytes_read);
                break;
            }
            
            // Validate block number
            if block_num > 1000000 {
                console_println!("   ⚠️ Skipping invalid block number: {}", block_num);
                continue;
            }
            
            console_println!("   📖 Reading block {} from disk", block_num);
            let block_data = match sb_mgr.read_block_data(block_num as u64) {
                Ok(data) => {
                    console_println!("   ✅ Successfully read block {}, got {} bytes", block_num, data.len());
                    data
                },
                Err(e) => {
                    console_println!("   ❌ Failed to read block {}: {:?}", block_num, e);
                    continue;
                }
            };
            
            let bytes_to_copy = core::cmp::min(file_size - bytes_read, block_data.len());
            console_println!("   📝 Copying {} bytes from block {}", bytes_to_copy, block_num);
            
            for i in 0..bytes_to_copy {
                if file_content.push(block_data[i]).is_err() {
                    console_println!("   ⚠️ File content buffer full");
                    break;
                }
                bytes_read += 1;
                if bytes_read >= file_size {
                    break;
                }
            }
            
            console_println!("   📊 Total bytes read so far: {}", bytes_read);
        }
        
        console_println!("   ✅ Read {} bytes from block-based file", bytes_read);
        Ok(file_content)
    }
    
    pub fn write_file_content(&self, inode: &mut Ext2Inode, offset: u64, data: &[u8]) -> FilesystemResult<usize> {
        console_println!("✏️  Writing {} bytes at offset {}", data.len(), offset);
        // TODO: Implement actual file writing
        Ok(data.len())
    }
    
    pub fn free_inode_blocks(&self, inode: &Ext2Inode) -> FilesystemResult<()> {
        console_println!("🗑️  Freeing blocks for inode");
        // TODO: Implement actual block freeing
        Ok(())
    }
    
    pub fn truncate_file(&self, inode: &mut Ext2Inode, new_size: u64) -> FilesystemResult<()> {
        console_println!("✂️  Truncating file to {} bytes", new_size);
        inode.set_size(new_size);
        Ok(())
    }
} 