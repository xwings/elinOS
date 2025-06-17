// Directory management for ext2

use super::structures::*;
use super::superblock::SuperblockManager;
use super::inode::InodeManager;
use super::super::traits::{FileEntry, FilesystemError, FilesystemResult};
use crate::console_println;
use heapless::Vec;
use core::mem;

/// Manages ext2 directory operations
pub struct DirectoryManager {
}

impl DirectoryManager {
    pub fn new() -> Self {
        Self {}
    }
    
    pub fn init(&mut self, _sb_mgr: &SuperblockManager, _inode_mgr: &InodeManager) -> FilesystemResult<()> {
        console_println!("ℹ️ Directory manager initialized");
        Ok(())
    }
    
    pub fn read_directory_entries(&self, inode: &Ext2Inode, files: &mut Vec<FileEntry, 64>, sb_mgr: &SuperblockManager, inode_mgr: &InodeManager) -> FilesystemResult<()> {
        console_println!("ℹ️ Reading directory entries...");
        
        if !inode.is_directory() {
            return Err(FilesystemError::NotADirectory);
        }
        
        // For now, read the first direct block only (simplified)
        if inode.i_block[0] != 0 {
            let block_data = sb_mgr.read_block_data(inode.i_block[0] as u64)?;
            self.parse_directory_block(&block_data, files, sb_mgr, inode_mgr)?;
        }
        
        Ok(())
    }
    
    pub fn is_directory(&self, inode: &Ext2Inode) -> bool {
        inode.is_directory()
    }
    
    pub fn find_entry_in_dir(&self, dir_inode_num: u32, entry_name: &str, sb_mgr: &SuperblockManager, inode_mgr: &InodeManager) -> FilesystemResult<Option<(Ext2DirEntry, u32, usize)>> {
        console_println!("ℹ️ Looking for '{}' in directory inode {}", entry_name, dir_inode_num);
        
        let dir_inode = inode_mgr.read_inode(dir_inode_num, sb_mgr)?;
        
        if !dir_inode.is_directory() {
            return Err(FilesystemError::NotADirectory);
        }
        
        // Search in first direct block (simplified)
        if dir_inode.i_block[0] != 0 {
            let block_num = dir_inode.i_block[0];
            console_println!("      Searching in block {}", block_num);
            let block_data = sb_mgr.read_block_data(block_num as u64)?;
            let result = self.find_entry_in_block(&block_data, entry_name, block_num);
            
            if let Ok(Some((entry, _, _))) = &result {
                let inode_num = entry.inode;
                console_println!("   ✅ Found '{}' -> inode {}", entry_name, inode_num);
            } else {
                console_println!("   ❌ '{}' not found", entry_name);
            }
            
            return result;
        }
        
        Ok(None)
    }
    
    pub fn list_directory(&self, inode: &Ext2Inode, sb_mgr: &SuperblockManager, inode_mgr: &InodeManager) -> FilesystemResult<Vec<(heapless::String<64>, usize, bool), 32>> {
        let mut result = Vec::new();
        
        if !inode.is_directory() {
            return Err(FilesystemError::NotADirectory);
        }
        
        // Read first direct block (simplified)
        if inode.i_block[0] != 0 {
            let block_data = sb_mgr.read_block_data(inode.i_block[0] as u64)?;
            self.parse_directory_block_for_listing(&block_data, &mut result, sb_mgr, inode_mgr)?;
        }
        
        Ok(result)
    }
    
    pub fn add_directory_entry(&self, parent_inode: u32, child_inode: u32, name: &str, file_type: u8, sb_mgr: &mut SuperblockManager, inode_mgr: &InodeManager) -> FilesystemResult<()> {
        console_println!("➕ Adding directory entry: {} -> {} (type {})", name, child_inode, file_type);
        
        if name.len() > 255 {
            return Err(FilesystemError::FilenameTooLong);
        }
        
        let name_bytes = name.as_bytes();
        let name_len = name_bytes.len() as u8;
        let required_rec_len = Self::calculate_rec_len(name_len);
        
        // Read parent directory inode
        let mut parent_dir_inode = inode_mgr.read_inode(parent_inode, sb_mgr)?;
        
        // Check if it's actually a directory
        if !parent_dir_inode.is_directory() {
            return Err(FilesystemError::NotADirectory);
        }
        
        // For simplicity, only handle direct block directories
        let first_block = parent_dir_inode.i_block[0];
        
        if first_block == 0 {
            // Need to allocate a new block for this directory
            let new_block = sb_mgr.allocate_block()?;
            parent_dir_inode.i_block[0] = new_block;
            parent_dir_inode.i_blocks_lo += (sb_mgr.get_block_size() / 512) as u32;
            
            // Initialize the block with empty directory structure
            let block_size = sb_mgr.get_block_size();
            let mut block_data = Vec::<u8, 4096>::new();
            for _ in 0..block_size {
                block_data.push(0).map_err(|_| FilesystemError::FilesystemFull)?;
            }
            
            // Create the first entry spanning the entire block
            let new_entry = Ext2DirEntry {
                inode: child_inode,
                rec_len: block_size as u16,
                name_len,
                file_type,
            };
            
            // Write the directory entry structure
            unsafe {
                let entry_ptr = block_data.as_mut_ptr() as *mut Ext2DirEntry;
                *entry_ptr = new_entry;
            }
            
            // Write the filename after the entry structure
            let name_offset = core::mem::size_of::<Ext2DirEntry>();
            for (i, &byte) in name_bytes.iter().enumerate() {
                if name_offset + i < block_data.len() {
                    block_data[name_offset + i] = byte;
                }
            }
            
            sb_mgr.write_block_data(new_block, &block_data)?;
            
            // Write back the updated parent inode
            inode_mgr.write_inode(parent_inode, &parent_dir_inode, sb_mgr)?;
            
            console_println!("✅ Added '{}' to new directory block {}", name, new_block);
        } else {
            // Add to existing directory block using entry splitting logic
            let mut block_data = sb_mgr.read_block_data(first_block as u64)?;
            let mut entry_added = false;
            let mut offset = 0;
            
            while offset < block_data.len() {
                if offset + core::mem::size_of::<Ext2DirEntry>() > block_data.len() {
                    break;
                }
                
                let dir_entry: Ext2DirEntry = unsafe {
                    core::ptr::read(block_data[offset..].as_ptr() as *const Ext2DirEntry)
                };
                
                let current_rec_len = dir_entry.rec_len as usize;
                let current_name_len = dir_entry.name_len;
                let current_inode = dir_entry.inode;
                
                                if current_rec_len == 0 {
                    break;
                }
                
                // Calculate space actually used by current entry
                let space_used_by_current = Self::calculate_rec_len(current_name_len) as usize;
                
                // Scenario 1: Reuse deleted entry (inode == 0)
                if current_inode == 0 && current_rec_len >= required_rec_len as usize {
                    console_println!("ℹ️ Reusing deleted entry at offset {}", offset);
                    let new_entry = Ext2DirEntry {
                        inode: child_inode,
                        rec_len: current_rec_len as u16,
                        name_len,
                        file_type,
                    };
                    
                    // Write the new entry
                    unsafe {
                        let entry_ptr = block_data[offset..].as_mut_ptr() as *mut Ext2DirEntry;
                        *entry_ptr = new_entry;
                    }
                    
                    // Write the filename
                    let name_start = offset + core::mem::size_of::<Ext2DirEntry>();
                    for (i, &byte) in name_bytes.iter().enumerate() {
                        if name_start + i < block_data.len() {
                            block_data[name_start + i] = byte;
                        }
                    }
                    
                    entry_added = true;
                    break;
                }
                
                // Scenario 2: Split current entry if it has enough slack space
                if current_inode != 0 && current_rec_len >= space_used_by_current + required_rec_len as usize {
                    console_println!("✂️  Splitting entry at offset {} (current_rec_len={}, used={}, needed={})", 
                                    offset, current_rec_len, space_used_by_current, required_rec_len);
                    
                    // Shorten the current entry to its actual size
                    unsafe {
                        let current_entry_ptr = block_data[offset..].as_mut_ptr() as *mut Ext2DirEntry;
                        (*current_entry_ptr).rec_len = space_used_by_current as u16;
                    }
                    
                    // Create new entry in the freed space
                    let new_entry_offset = offset + space_used_by_current;
                    let remaining_space = current_rec_len - space_used_by_current;
                    
                    let new_entry = Ext2DirEntry {
                        inode: child_inode,
                        rec_len: remaining_space as u16,
                        name_len,
                        file_type,
                    };
                    
                    // Write the new entry
                    unsafe {
                        let new_entry_ptr = block_data[new_entry_offset..].as_mut_ptr() as *mut Ext2DirEntry;
                        *new_entry_ptr = new_entry;
                    }
                    
                    // Write the filename
                    let name_start = new_entry_offset + core::mem::size_of::<Ext2DirEntry>();
                    for (i, &byte) in name_bytes.iter().enumerate() {
                        if name_start + i < block_data.len() {
                            block_data[name_start + i] = byte;
                        }
                    }
                    
                    entry_added = true;
                    break;
                }
                
                // Move to next entry, but check if this is the last entry
                if offset + current_rec_len >= block_data.len() {
                    break;
                }
                
                offset += current_rec_len;
            }
            
            if entry_added {
                sb_mgr.write_block_data(first_block, &block_data)?;
                console_println!("✅ Added '{}' to existing directory block {}", name, first_block);
            } else {
                console_println!("❌ No space found in directory block for '{}'", name);
                return Err(FilesystemError::FilesystemFull);
            }
        }
        
        Ok(())
    }
    
    fn calculate_rec_len(name_len: u8) -> u16 {
        // Directory entry header (8 bytes) + name length, aligned to 4-byte boundary
        ((core::mem::size_of::<Ext2DirEntry>() + name_len as usize + 3) & !3) as u16
    }
    
    pub fn remove_directory_entry(&self, parent_inode: u32, name: &str, sb_mgr: &SuperblockManager, inode_mgr: &InodeManager) -> FilesystemResult<()> {
        //console_println!("➖ Removing directory entry: '{}' from inode {}", name, parent_inode);
        
        // First, find the entry to get its location
        if let Some((_, found_inode, _)) = self.find_entry_in_dir(parent_inode, name, sb_mgr, inode_mgr)? {
          //  console_println!("ℹ️ Found entry '{}' with inode {}, proceeding with removal", name, found_inode);
            
            // Read the parent directory inode
            let parent_dir_inode = inode_mgr.read_inode(parent_inode, sb_mgr)?;
            
            // For simplicity, only handle direct block directories
            let first_block = parent_dir_inode.i_block[0];
            
            if first_block == 0 {
                console_println!("❌ Parent directory has no blocks allocated");
                return Err(FilesystemError::FileNotFound);
            }
            
            // Read the directory block
            let mut block_data = sb_mgr.read_block_data(first_block as u64)?;
            
            // Find and remove the entry
            self.remove_entry_from_block(&mut block_data, name)?;
            
            // Write the updated block back to disk
            sb_mgr.write_block_data(first_block, &block_data)?;
            
            console_println!("✅ Successfully removed directory entry '{}' from inode {}", name, parent_inode);
            Ok(())
        } else {
            console_println!("❌ Entry '{}' not found in directory inode {}", name, parent_inode);
            Err(FilesystemError::FileNotFound)
        }
    }
    
    fn remove_entry_from_block(&self, block_data: &mut Vec<u8, 4096>, target_name: &str) -> FilesystemResult<()> {
        let mut offset = 0;
        
        while offset < block_data.len() {
            if offset + core::mem::size_of::<Ext2DirEntry>() > block_data.len() {
                break;
            }
            
            let dir_entry: Ext2DirEntry = unsafe {
                core::ptr::read(block_data[offset..].as_ptr() as *const Ext2DirEntry)
            };
            
            let rec_len = dir_entry.rec_len as usize;
            let name_len = dir_entry.name_len as usize;
            let inode_num = dir_entry.inode;
            
            if inode_num == 0 || rec_len == 0 {
                break;
            }
            
            // Extract filename
            let name_start = offset + core::mem::size_of::<Ext2DirEntry>();
            let name_end = name_start + name_len;
            
            if name_end <= block_data.len() {
                let name_bytes = &block_data[name_start..name_end];
                if let Ok(name_str) = core::str::from_utf8(name_bytes) {
                    if name_str == target_name {
                        console_println!("ℹ️ Found target entry '{}' at offset {}, marking as deleted", target_name, offset);
                        
                        // Mark the entry as deleted by setting inode to 0
                        unsafe {
                            let entry_ptr = block_data[offset..].as_mut_ptr() as *mut Ext2DirEntry;
                            (*entry_ptr).inode = 0;
                            // Keep rec_len and other fields for proper directory traversal
                        }
                        
                        console_println!("🗑️  Entry '{}' marked as deleted (inode=0)", target_name);
                        return Ok(());
                    }
                }
            }
            
            offset += rec_len;
            
            if offset >= block_data.len() {
                break;
            }
        }
        
        console_println!("❌ Target entry '{}' not found in directory block", target_name);
        Err(FilesystemError::FileNotFound)
    }
    
    pub fn create_dot_entries(&self, dir_inode: u32, parent_inode: u32, sb_mgr: &mut SuperblockManager, inode_mgr: &InodeManager) -> FilesystemResult<()> {
        console_println!("ℹ️ Creating . and .. entries for inode {}", dir_inode);
        
        // Add "." entry (current directory)
        self.add_directory_entry(dir_inode, dir_inode, ".", EXT2_FT_DIR, sb_mgr, inode_mgr)?;
        
        // Add ".." entry (parent directory)
        self.add_directory_entry(dir_inode, parent_inode, "..", EXT2_FT_DIR, sb_mgr, inode_mgr)?;
        
        console_println!("✅ Created . and .. entries for directory inode {}", dir_inode);
        Ok(())
    }
    
    pub fn is_empty_directory(&self, inode: &Ext2Inode, sb_mgr: &SuperblockManager) -> FilesystemResult<bool> {
        if !inode.is_directory() {
            return Err(FilesystemError::NotADirectory);
        }
        
        // Check if directory only contains . and .. entries
        let mut entry_count = 0;
        
        if inode.i_block[0] != 0 {
            let block_data = sb_mgr.read_block_data(inode.i_block[0] as u64)?;
            entry_count = self.count_directory_entries(&block_data)?;
        }
        
        // Directory is empty if it only has . and .. entries (count <= 2)
        Ok(entry_count <= 2)
    }
    
    // Helper methods
    
    fn parse_directory_block(&self, block_data: &[u8], files: &mut Vec<FileEntry, 64>, sb_mgr: &SuperblockManager, inode_mgr: &InodeManager) -> FilesystemResult<()> {
        let mut offset = 0;
        
        while offset < block_data.len() {
            if offset + mem::size_of::<Ext2DirEntry>() > block_data.len() {
                break;
            }
            
            let dir_entry: Ext2DirEntry = unsafe {
                core::ptr::read(block_data[offset..].as_ptr() as *const Ext2DirEntry)
            };
            
            // Copy values from packed struct
            let inode_num = dir_entry.inode;
            let rec_len = dir_entry.rec_len as usize;
            let name_len = dir_entry.name_len as usize;
            let file_type = dir_entry.file_type;
            
            if inode_num == 0 || rec_len == 0 || rec_len > block_data.len() - offset {
                break;
            }
            
            // Extract filename
            let name_start = offset + mem::size_of::<Ext2DirEntry>();
            let name_end = name_start + name_len;
            
            if name_end <= block_data.len() {
                let name_bytes = &block_data[name_start..name_end];
                if let Ok(name_str) = core::str::from_utf8(name_bytes) {
                    // Skip . and .. entries
                    if name_str != "." && name_str != ".." {
                        // Get inode to determine size
                        if let Ok(entry_inode) = inode_mgr.read_inode(inode_num, sb_mgr) {
                            let is_dir = entry_inode.is_directory();
                            let size = if is_dir { 0 } else { entry_inode.get_size() as usize };
                            
                            if let Ok(file_entry) = if is_dir {
                                FileEntry::new_directory(name_str, inode_num as u64)
                            } else {
                                FileEntry::new_file(name_str, inode_num as u64, size)
                            } {
                                let _ = files.push(file_entry);
                            }
                        }
                    }
                }
            }
            
            offset += rec_len;
        }
        
        Ok(())
    }
    
    fn parse_directory_block_for_listing(&self, block_data: &[u8], result: &mut Vec<(heapless::String<64>, usize, bool), 32>, sb_mgr: &SuperblockManager, inode_mgr: &InodeManager) -> FilesystemResult<()> {
        let mut offset = 0;
        console_println!("ℹ️ Parsing directory block ({} bytes):", block_data.len());
        
        while offset < block_data.len() {
            if offset + mem::size_of::<Ext2DirEntry>() > block_data.len() {
                console_println!("   ⚠️  Not enough space for dir entry at offset {}, breaking", offset);
                break;
            }
            
            let dir_entry: Ext2DirEntry = unsafe {
                core::ptr::read(block_data[offset..].as_ptr() as *const Ext2DirEntry)
            };
            
            // Copy values from packed struct
            let inode_num = dir_entry.inode;
            let rec_len = dir_entry.rec_len as usize;
            let name_len = dir_entry.name_len as usize;
            let file_type = dir_entry.file_type;
            
            console_println!("   Entry at offset {}: inode={}, rec_len={}, name_len={}, file_type={}", 
                offset, inode_num, rec_len, name_len, file_type);
            
            // Validate entry
            if inode_num == 0 {
                console_println!("   ❌ Invalid entry (inode=0), breaking");
                break;
            }
            
            if rec_len == 0 || rec_len > block_data.len() - offset {
                console_println!("   ❌ Invalid rec_len {}, breaking", rec_len);
                break;
            }
            
            if name_len == 0 || name_len > 255 {
                console_println!("   ❌ Invalid name_len {}, breaking", name_len);
                break;
            }
            
            // Validate inode number is reasonable (should be < 100000 for small filesystems)
            if inode_num > 100000 {
                console_println!("   ❌ Suspicious inode number {}, skipping", inode_num);
                offset += rec_len;
                continue;
            }
            
            // Extract filename
            let name_start = offset + mem::size_of::<Ext2DirEntry>();
            let name_end = name_start + name_len;
            
            if name_end > block_data.len() {
                console_println!("   ❌ Name extends beyond block boundary, breaking");
                break;
            }
            
            let name_bytes = &block_data[name_start..name_end];
            if let Ok(name_str) = core::str::from_utf8(name_bytes) {
                console_println!("   ℹ️ Found entry: '{}'", name_str);
                if let Ok(short_name) = heapless::String::try_from(name_str) {
                    // Use the file_type from directory entry as primary source
                    // EXT2_FT_DIR = 2, EXT2_FT_REG_FILE = 1
                    let is_dir = file_type == EXT2_FT_DIR;
                    
                    // Try to read inode to get size, but don't rely on it for type determination
                    let size = match inode_mgr.read_inode(inode_num, sb_mgr) {
                        Ok(entry_inode) => {
                            if is_dir { 0 } else { entry_inode.get_size() as usize }
                        },
                        Err(_) => {
                            console_println!("   ❌ Failed to read inode {} for '{}', using size 0", inode_num, name_str);
                            0
                        }
                    };
                    
                    console_println!("   ✅ Added: '{}' (dir: {}, size: {})", name_str, is_dir, size);
                    let _ = result.push((short_name, size, is_dir));
                } else {
                    console_println!("   ❌ Filename too long: '{}'", name_str);
                }
            } else {
                console_println!("   ❌ Invalid UTF-8 in filename at offset {}", name_start);
            }
            
            offset += rec_len;
            
            // Prevent infinite loops
            if offset >= block_data.len() {
                break;
            }
        }
        
        console_println!("ℹ️ Directory parsing complete, found {} entries", result.len());
        Ok(())
    }
    
    fn find_entry_in_block(&self, block_data: &[u8], entry_name: &str, block_num: u32) -> FilesystemResult<Option<(Ext2DirEntry, u32, usize)>> {
        let mut offset = 0;
        console_println!("      Scanning block {} for '{}':", block_num, entry_name);
        
        while offset < block_data.len() {
            if offset + mem::size_of::<Ext2DirEntry>() > block_data.len() {
                break;
            }
            
            let dir_entry: Ext2DirEntry = unsafe {
                core::ptr::read(block_data[offset..].as_ptr() as *const Ext2DirEntry)
            };
            
            // Copy values from packed struct
            let inode_num = dir_entry.inode;
            let rec_len = dir_entry.rec_len as usize;
            let name_len = dir_entry.name_len as usize;
            
            if inode_num == 0 || rec_len == 0 || rec_len > block_data.len() - offset {
                break;
            }
            
            // Extract filename
            let name_start = offset + mem::size_of::<Ext2DirEntry>();
            let name_end = name_start + name_len;
            
            if name_end <= block_data.len() {
                let name_bytes = &block_data[name_start..name_end];
                if let Ok(name_str) = core::str::from_utf8(name_bytes) {
                    console_println!("          Entry: '{}' -> inode {}", name_str, inode_num);
                    if name_str == entry_name {
                        console_println!("         ✅ MATCH found!");
                        return Ok(Some((dir_entry, inode_num, offset)));
                    }
                }
            }
            
            offset += rec_len;
        }
        
        console_println!("      ❌ No match found in block");
        Ok(None)
    }
    
    fn count_directory_entries(&self, block_data: &[u8]) -> FilesystemResult<usize> {
        let mut count = 0;
        let mut offset = 0;
        
        while offset < block_data.len() {
            if offset + mem::size_of::<Ext2DirEntry>() > block_data.len() {
                break;
            }
            
            let dir_entry: Ext2DirEntry = unsafe {
                core::ptr::read(block_data[offset..].as_ptr() as *const Ext2DirEntry)
            };
            
            // Copy values from packed struct
            let inode_num = dir_entry.inode;
            let rec_len = dir_entry.rec_len as usize;
            
            if inode_num == 0 || rec_len == 0 || rec_len > block_data.len() - offset {
                break;
            }
            
            count += 1;
            offset += rec_len;
        }
        
        Ok(count)
    }
} 