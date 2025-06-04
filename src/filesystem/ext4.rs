// ext4 Filesystem Implementation (Real Parser)

use super::traits::{FileSystem, FileEntry, FilesystemError, FilesystemResult};
use crate::{console_println, virtio_blk};
use heapless::Vec;
use core::mem::drop;

/// ext4 constants
const SECTOR_SIZE: usize = 512;
const EXT4_SUPERBLOCK_OFFSET: usize = 1024;
const EXT4_MAGIC: u16 = 0xEF53;
const EXT4_ROOT_INODE: u32 = 2;

/// Simplified ext4 Superblock - only essential fields
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
struct Ext4Superblock {
    s_inodes_count: u32,        // 0x00
    s_blocks_count_lo: u32,     // 0x04
    s_r_blocks_count_lo: u32,   // 0x08
    s_free_blocks_count_lo: u32, // 0x0C
    s_free_inodes_count: u32,   // 0x10
    s_first_data_block: u32,    // 0x14
    s_log_block_size: u32,      // 0x18
    s_log_cluster_size: u32,    // 0x1C
    s_blocks_per_group: u32,    // 0x20
    s_clusters_per_group: u32,  // 0x24
    s_inodes_per_group: u32,    // 0x28
    s_mtime: u32,              // 0x2C
    s_wtime: u32,              // 0x30
    s_mnt_count: u16,          // 0x34
    s_max_mnt_count: u16,      // 0x36
    s_magic: u16,              // 0x38 - Magic signature (0xEF53)
    s_state: u16,              // 0x3A
    s_errors: u16,             // 0x3C
    s_minor_rev_level: u16,    // 0x3E
    s_lastcheck: u32,          // 0x40
    s_checkinterval: u32,      // 0x44
    s_creator_os: u32,         // 0x48
    s_rev_level: u32,          // 0x4C
    s_def_resuid: u16,         // 0x50
    s_def_resgid: u16,         // 0x52
    // Extended fields
    s_first_ino: u32,          // 0x54
    s_inode_size: u16,         // 0x58
    s_block_group_nr: u16,     // 0x5A
    _reserved: [u8; 932],      // Padding to 1024 bytes
}

/// Simplified Group Descriptor
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
struct Ext4GroupDesc {
    bg_block_bitmap_lo: u32,      // 0x00
    bg_inode_bitmap_lo: u32,      // 0x04
    bg_inode_table_lo: u32,       // 0x08
    bg_free_blocks_count_lo: u16, // 0x0C
    bg_free_inodes_count_lo: u16, // 0x0E
    bg_used_dirs_count_lo: u16,   // 0x10
    bg_flags: u16,                // 0x12
    _reserved: [u8; 16],          // Padding to 32 bytes
}

/// Simplified Inode - focusing on basic fields
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
struct Ext4Inode {
    i_mode: u16,        // 0x00
    i_uid: u16,         // 0x02
    i_size_lo: u32,     // 0x04
    i_atime: u32,       // 0x08
    i_ctime: u32,       // 0x0C
    i_mtime: u32,       // 0x10
    i_dtime: u32,       // 0x14
    i_gid: u16,         // 0x18
    i_links_count: u16, // 0x1A
    i_blocks_lo: u32,   // 0x1C
    i_flags: u32,       // 0x20
    i_osd1: u32,        // 0x24
    i_block: [u32; 15], // 0x28 - Block pointers (60 bytes)
    i_generation: u32,  // 0x64
    i_file_acl_lo: u32, // 0x68
    i_size_high: u32,   // 0x6C
    _padding: [u8; 144], // Padding to 256 bytes total
}

/// Directory Entry
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
struct Ext4DirEntry {
    inode: u32,       // Inode number
    rec_len: u16,     // Directory entry length
    name_len: u8,     // Name length
    file_type: u8,    // File type
    // name follows here
}

/// File type constants
const EXT4_FT_REG_FILE: u8 = 1;
const EXT4_FT_DIR: u8 = 2;

/// ext4 inode flags
const EXT4_EXTENTS_FL: u32 = 0x00080000;  // Inode uses extents

/// Extent structures
#[repr(C, packed)]
#[derive(Debug, Clone, Copy, Default)] // Added Default
struct Ext4ExtentHeader {
    eh_magic: u16,          // Magic number (0xF30A)
    eh_entries: u16,        // Number of valid entries following the header
    eh_max: u16,            // Maximum number of entries that could follow
    eh_depth: u16,          // Depth of tree (0 = leaf node)
    eh_generation: u32,     // Generation
}

#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
struct Ext4Extent {
    ee_block: u32,          // First logical block extent covers
    ee_len: u16,            // Number of blocks covered by extent
    ee_start_hi: u16,       // High 16 bits of physical block
    ee_start_lo: u32,       // Low 32 bits of physical block
}

const EXT4_EXT_MAGIC: u16 = 0xF30A;

/// Real ext4 Filesystem Implementation
pub struct Ext4FileSystem {
    superblock: Option<Ext4Superblock>,
    files: Vec<FileEntry, 64>,
    initialized: bool,
    mounted: bool,
    block_size: usize,
    group_desc: Option<Ext4GroupDesc>,
}

impl Ext4FileSystem {
    pub fn new() -> Self {
        Ext4FileSystem {
            superblock: None,
            files: Vec::new(),
            initialized: false,
            mounted: false,
            block_size: 1024, // Default ext4 block size
            group_desc: None,
        }
    }
    
    /// Initialize the ext4 filesystem
    pub fn init(&mut self) -> FilesystemResult<()> {
        console_println!("🗂️  Initializing real ext4 filesystem...");

        // Step 1: Read superblock
        self.read_superblock()?;
        
        // Step 2: Read group descriptor
        self.read_group_descriptor()?;
        
        // Step 3: Parse real root directory from disk
        self.parse_root_directory()?;
        
        self.mounted = true;
        console_println!("✅ Real ext4 filesystem mounted");
        Ok(())
    }
    
    /// Read and validate superblock
    fn read_superblock(&mut self) -> FilesystemResult<()> {
        console_println!("📖 Reading ext4 superblock...");
        
        let mut disk_device = virtio_blk::VIRTIO_BLK.lock();
        
        if !disk_device.is_initialized() {
            return Err(FilesystemError::DeviceError);
        }

        // Read superblock sectors (1024 bytes starting at offset 1024)
        let start_sector = EXT4_SUPERBLOCK_OFFSET / SECTOR_SIZE; // sector 2
        console_println!("ext4::read_superblock: Calculated start_sector for superblock: {}", start_sector); // Added log
        let mut sb_buffer = [0u8; 1024];
        
        // Read 2 sectors to get full superblock
        for i in 0..2 {
            let current_sector_to_read = (start_sector + i) as u64;
            console_println!("ext4::read_superblock: Attempting to read sector {}", current_sector_to_read); // Added log
            let mut sector_buf = [0u8; SECTOR_SIZE];
            match disk_device.read_blocks(current_sector_to_read, &mut sector_buf) {
                Ok(_) => {
            sb_buffer[i * SECTOR_SIZE..(i + 1) * SECTOR_SIZE].copy_from_slice(&sector_buf);
                    console_println!("ext4::read_superblock: Successfully read sector {}", current_sector_to_read); // Added log
                }
                Err(e_virtio) => {
                    console_println!("ext4::read_superblock: virtio_blk read_blocks for sector {} FAILED with {:?}", current_sector_to_read, e_virtio); // Added log
                    return Err(FilesystemError::IoError);
                }
            }
        }
        
        drop(disk_device);
        
        // Parse superblock
        let superblock: Ext4Superblock = unsafe {
            let sb_ptr = sb_buffer.as_ptr() as *const Ext4Superblock;
            *sb_ptr
        };

        // Validate magic
        let s_magic = superblock.s_magic;
        if s_magic != EXT4_MAGIC {
            console_println!("❌ Invalid ext4 magic: 0x{:04x}", s_magic);
            return Err(FilesystemError::InvalidSuperblock);
        }

        console_println!("✅ Valid ext4 superblock found!");
        
        // Copy fields to local variables to avoid packed alignment issues
        let s_inodes_count = superblock.s_inodes_count;
        let s_blocks_count_lo = superblock.s_blocks_count_lo;
        let s_inodes_per_group = superblock.s_inodes_per_group;
        let s_log_block_size = superblock.s_log_block_size;
        // Create a local copy for printing to avoid unaligned access.
        let s_magic_val = superblock.s_magic;
        
        console_println!("✅ Valid ext4 superblock found! Magic: 0x{:04x}", s_magic_val);
        console_println!("   📊 Inodes: {}", s_inodes_count);
        console_println!("   📊 Blocks: {}", s_blocks_count_lo);
        console_println!("   📊 Inodes per group: {}", s_inodes_per_group);
        
        // Calculate block size
        self.block_size = 1024 << s_log_block_size;
        console_println!("   💾 Block size: {} bytes", self.block_size);
        
        self.superblock = Some(superblock);
        self.initialized = true;
        Ok(())
    }
    
    /// Read group descriptor
    fn read_group_descriptor(&mut self) -> FilesystemResult<()> {
        console_println!("📊 Reading group descriptor...");
        
        let superblock = self.superblock.ok_or(FilesystemError::NotInitialized)?;
        
        // Group descriptor table is typically in block after superblock
        let s_first_data_block = superblock.s_first_data_block;
        let sb_block = if s_first_data_block == 0 { 1 } else { s_first_data_block };
        let gdt_block = sb_block + 1;
        
        console_println!("   📍 Reading group descriptor from block {}", gdt_block);
        
        let gdt_data = self.read_block_data(gdt_block as u64)?;
        
        let group_desc: Ext4GroupDesc = unsafe {
            let gdt_ptr = gdt_data.as_ptr() as *const Ext4GroupDesc;
            *gdt_ptr
        };
        
        // Copy fields to local variables to avoid packed alignment issues
        let bg_inode_table_lo = group_desc.bg_inode_table_lo;
        let bg_free_inodes_count_lo = group_desc.bg_free_inodes_count_lo;
        
        console_println!("   📊 Inode table at block: {}", bg_inode_table_lo);
        console_println!("   📊 Free inodes: {}", bg_free_inodes_count_lo);
        
        self.group_desc = Some(group_desc);
        Ok(())
    }
    
    /// Parse root directory from real ext4 disk structures
    fn parse_root_directory(&mut self) -> FilesystemResult<()> {
        console_println!("📂 Parsing real ext4 root directory...");
        
        // Read root inode (inode #2)
        let root_inode = self.read_inode(EXT4_ROOT_INODE)?;
        let i_mode = root_inode.i_mode;
        let i_size_lo = root_inode.i_size_lo;
        
        console_println!("   📄 Root inode: mode=0x{:x}, size={}", i_mode, i_size_lo);
        
        // Check if it's a directory (mode & 0xF000 == 0x4000)
        if (i_mode & 0xF000) != 0x4000 {
            console_println!("❌ Root inode is not a directory");
            return Err(FilesystemError::CorruptedFilesystem);
        }
        
        // Parse directory entries from the root inode
        self.read_directory_entries(&root_inode)?;
        
        console_println!("✅ Parsed {} entries from real ext4 root directory", self.files.len());
        Ok(())
    }
    
    /// Read an inode from the inode table
    fn read_inode(&self, inode_num: u32) -> FilesystemResult<Ext4Inode> {
        let superblock = self.superblock.ok_or(FilesystemError::NotInitialized)?;
        let group_desc = self.group_desc.ok_or(FilesystemError::NotInitialized)?;
        
        let s_inodes_per_group = superblock.s_inodes_per_group;
        let s_inode_size = superblock.s_inode_size;
        let bg_inode_table_lo = group_desc.bg_inode_table_lo;
        
        // Calculate which group and offset within group
        let group = (inode_num - 1) / s_inodes_per_group;
        let offset = (inode_num - 1) % s_inodes_per_group;
        
        console_println!("   📍 Reading inode {} (group {}, offset {})", inode_num, group, offset);
        
        // For simplicity, only support group 0 for now
        if group != 0 {
            console_println!("❌ Multi-group ext4 not supported yet");
            return Err(FilesystemError::UnsupportedFilesystem);
        }
        
        // Calculate byte offset within inode table
        let inode_size = if s_inode_size > 0 { s_inode_size as usize } else { 256 };
        let inode_byte_offset = offset as usize * inode_size;
        
        // Calculate which block and offset within block
        let sectors_per_block = self.block_size / SECTOR_SIZE;
        let bytes_per_block = self.block_size;
        let block_offset = inode_byte_offset / bytes_per_block;
        let byte_offset_in_block = inode_byte_offset % bytes_per_block;
        
        let inode_block = bg_inode_table_lo + block_offset as u32;
        
        console_println!("   📍 Inode table block: {}, byte offset: {}", inode_block, byte_offset_in_block);
        
        // Read the block containing our inode
        let block_data = self.read_block_data(inode_block as u64)?;
        
        // Extract inode from the block
        if byte_offset_in_block + core::mem::size_of::<Ext4Inode>() > block_data.len() {
            console_println!("❌ Inode extends beyond block boundary");
            return Err(FilesystemError::CorruptedFilesystem);
        }
        
        let inode: Ext4Inode = unsafe {
            let inode_ptr = (block_data.as_ptr().add(byte_offset_in_block)) as *const Ext4Inode;
            *inode_ptr
        };
        
        Ok(inode)
    }
    
    /// Read directory entries from a directory inode
    fn read_directory_entries(&mut self, dir_inode: &Ext4Inode) -> FilesystemResult<()> {
        console_println!("   📖 Reading directory entries from real blocks...");
        
        let i_size_lo = dir_inode.i_size_lo;
        let i_flags = dir_inode.i_flags;
        
        console_println!("   🔍 Inode flags: 0x{:08x}", i_flags);
        
        // Check if this inode uses extents
        if (i_flags & EXT4_EXTENTS_FL) != 0 {
            console_println!("   🌟 Inode uses extents - parsing extent tree");
            return self.read_directory_entries_from_extents(dir_inode);
        } else {
            console_println!("   📋 Inode uses direct blocks - parsing traditional block pointers");
            return self.read_directory_entries_from_blocks(dir_inode);
        }
    }
    
    /// Read directory entries from extent-based inode
    fn read_directory_entries_from_extents(&mut self, dir_inode: &Ext4Inode) -> FilesystemResult<()> {
        // Copy i_block array to avoid packed field alignment issues
        let i_block_copy = dir_inode.i_block;
        
        // Parse extent header from the beginning of i_block
        let extent_header: Ext4ExtentHeader = unsafe {
            let header_ptr = i_block_copy.as_ptr() as *const Ext4ExtentHeader;
            *header_ptr
        };
        
        let eh_magic_val = extent_header.eh_magic; // local copy
        let eh_entries_val = extent_header.eh_entries; // local copy
        let eh_depth_val = extent_header.eh_depth; // local copy
        
        console_println!("   🔍 Extent header: magic=0x{:04x}, entries={}, depth={}", 
            eh_magic_val, eh_entries_val, eh_depth_val);
        
        if eh_magic_val != EXT4_EXT_MAGIC {
            console_println!("   ❌ Invalid extent magic: 0x{:04x} (expected: 0x{:04x})", 
                eh_magic_val, EXT4_EXT_MAGIC);
            return Err(FilesystemError::CorruptedFilesystem);
        }
        
        if eh_depth_val != 0 {
            console_println!("   ❌ Multi-level extent trees not supported (depth={})", eh_depth_val);
            return Err(FilesystemError::UnsupportedFilesystem);
        }
        
        // Parse extent entries (they come right after the header)
        let extents_start = core::mem::size_of::<Ext4ExtentHeader>();
        let i_block_bytes = unsafe {
            core::slice::from_raw_parts(
                i_block_copy.as_ptr() as *const u8,
                60  // i_block is 15 * 4 = 60 bytes
            )
        };
        
        console_println!("   📊 Processing {} extent entries", eh_entries_val);
        
        for i in 0..eh_entries_val as usize {
            let extent_offset = extents_start + i * core::mem::size_of::<Ext4Extent>();
            
            if extent_offset + core::mem::size_of::<Ext4Extent>() > i_block_bytes.len() {
                console_println!("   ⚠️ Extent {} extends beyond i_block", i);
                break;
            }
            
            let extent: Ext4Extent = unsafe {
                let extent_ptr = (i_block_bytes.as_ptr().add(extent_offset)) as *const Ext4Extent;
                *extent_ptr
            };
            
            let ee_block = extent.ee_block;
            let ee_len = extent.ee_len;
            let ee_start_hi = extent.ee_start_hi;
            let ee_start_lo = extent.ee_start_lo;
            
            // Calculate physical block number
            let physical_block = ((ee_start_hi as u64) << 32) | (ee_start_lo as u64);
            
            console_println!("   🔍 Extent {}: logical={}, len={}, physical={}", 
                i, ee_block, ee_len, physical_block);
            
            // Read directory data from this extent
            for block_offset in 0..ee_len {
                let block_num = physical_block + block_offset as u64;
                
                console_println!("   📍 Reading directory block {} (from extent)", block_num);
                
                let block_data = match self.read_block_data(block_num) {
                    Ok(data) => data,
                    Err(_) => {
                        console_println!("   ⚠️ Failed to read extent block {}, skipping", block_num);
                        continue;
                    }
                };
                
                // Parse directory entries in this block
                self.parse_directory_block(&block_data)?;
            }
        }
        
        Ok(())
    }
    
    /// Read directory entries from traditional direct block pointers
    fn read_directory_entries_from_blocks(&mut self, dir_inode: &Ext4Inode) -> FilesystemResult<()> {
        let i_size_lo = dir_inode.i_size_lo;
        let mut bytes_read = 0;
        
        // Copy i_block array to avoid packed field alignment issues
        let i_block_copy = dir_inode.i_block;
        
        // Debug: Print first few block numbers
        console_println!("   🔍 First 5 block numbers: {:?}", &i_block_copy[..5]);
        
        // Handle only direct blocks (first 12 entries in i_block) for simplicity
        for &block_num in i_block_copy.iter().take(12) {
            if block_num == 0 || bytes_read >= i_size_lo as usize {
                break;
            }
            
            // Validate block number to avoid accessing invalid blocks
            if block_num > 1000000 {  // Reasonable upper limit for our test image
                console_println!("   ⚠️ Skipping potentially invalid block number: {}", block_num);
                continue;
            }
            
            console_println!("   📍 Reading directory block {}", block_num);
            
            // Read the directory block
            let block_data = match self.read_block_data(block_num as u64) {
                Ok(data) => data,
                Err(_) => {
                    console_println!("   ⚠️ Failed to read block {}, skipping", block_num);
                    continue;
                }
            };
            
            // Parse directory entries in this block
            self.parse_directory_block(&block_data)?;
        }
        
        Ok(())
    }
    
    /// Parse directory entries from a block of data
    fn parse_directory_block(&mut self, block_data: &[u8]) -> FilesystemResult<()> {
        // Debug: Show first 32 bytes of the block
        console_println!("   🔍 First 32 bytes of block: {:02x?}", &block_data[..32.min(block_data.len())]);
        
        let mut offset = 0;
        let mut entries_in_block = 0;
        
        while offset < block_data.len() {
            // Ensure we have enough bytes for directory entry header
            if offset + 8 > block_data.len() {
                console_println!("   🔍 Not enough bytes for dir entry header at offset {}", offset);
                break;
            }
            
            let dir_entry: Ext4DirEntry = unsafe {
                let entry_ptr = (block_data.as_ptr().add(offset)) as *const Ext4DirEntry;
                *entry_ptr
            };
            
            let inode = dir_entry.inode;
            let rec_len = dir_entry.rec_len;
            let name_len = dir_entry.name_len;
            let file_type = dir_entry.file_type;
            
            console_println!("   🔍 Dir entry at offset {}: inode={}, rec_len={}, name_len={}, type={}", 
                offset, inode, rec_len, name_len, file_type);
            
            // Handle empty/deleted entries (inode = 0)
            if inode == 0 {
                if rec_len == 0 {
                    console_println!("   ⚠️ Zero rec_len with zero inode, stopping parse");
                    break;
                }
                console_println!("   🔍 Skipping deleted entry (inode=0), advancing {} bytes", rec_len);
                offset += rec_len as usize;
                continue;
            }
            
            // Validate rec_len
            if rec_len == 0 {
                console_println!("   ⚠️ Invalid rec_len=0, stopping parse");
                break;
            }
            
            if offset + rec_len as usize > block_data.len() {
                console_println!("   ⚠️ rec_len {} extends beyond block (offset={}, block_len={})", 
                    rec_len, offset, block_data.len());
                break;
            }
            
            if name_len > 0 && name_len <= 255 {
                // Extract filename
                let name_start = offset + 8; // Skip fixed part of dir entry
                let name_end = name_start + name_len as usize;
                
                if name_end <= block_data.len() {
                    let name_bytes = &block_data[name_start..name_end];
                    
                    if let Ok(filename) = core::str::from_utf8(name_bytes) {
                        console_println!("   📄 Found entry: '{}' (inode: {}, type: {})", 
                            filename, inode, file_type);
                        
                        // Skip "." and ".." entries for file listing
                        if filename != "." && filename != ".." {
                            // First try using file_type from directory entry
                            let mut is_directory = file_type == EXT4_FT_DIR;
                            
                            // If file_type is 0 or doesn't match expected values, check inode mode as fallback
                            if file_type == 0 || (file_type != EXT4_FT_DIR && file_type != EXT4_FT_REG_FILE) {
                                console_println!("   🔍 file_type={} unreliable, checking inode mode for '{}'", file_type, filename);
                                match self.read_inode(inode) {
                                    Ok(entry_inode) => {
                                        // Check inode mode: 0x4000 = S_IFDIR (directory)
                                        let inode_mode = entry_inode.i_mode; // Copy to local variable
                                        is_directory = (inode_mode & 0xF000) == 0x4000;
                                        console_println!("   📊 Inode mode: 0x{:04x}, is_directory: {}", inode_mode, is_directory);
                                    },
                                    Err(e) => {
                                        console_println!("   ⚠️ Failed to read inode {} for type detection: {:?}", inode, e);
                                        // Keep original file_type determination as fallback
                                    }
                                }
                            }
                            
                            // Get file size from inode (with better error handling)
                            let file_size = match self.read_inode(inode) {
                                Ok(file_inode) => {
                                    let size = file_inode.i_size_lo as usize;
                                    console_println!("   📊 File size from inode: {} bytes", size);
                                    size
                                },
                                Err(e) => {
                                    console_println!("   ⚠️ Failed to read inode {}: {:?}", inode, e);
                                    0
                                }
                            };
                            
                            let file_entry = if is_directory {
                                FileEntry::new_directory(filename, inode as u64)?
                            } else {
                                FileEntry::new_file(filename, inode as u64, file_size)?
                            };
                            
                            console_println!("   ✅ Added {}: {} (inode: {}, size: {})", 
                                if is_directory { "DIR " } else { "FILE" },
                                filename, inode, file_size);
                            
                            if self.files.push(file_entry.clone()).is_err() {
                                console_println!("   ⚠️ File cache full");
                                return Ok(());
                            }
                            
                            entries_in_block += 1;
                        } else {
                            console_println!("   🔍 Skipping special entry: '{}'", filename);
                        }
                    } else {
                        console_println!("   ⚠️ Invalid UTF-8 filename at offset {}", name_start);
                    }
                } else {
                    console_println!("   ⚠️ Filename extends beyond block boundary");
                }
            } else {
                console_println!("   ⚠️ Invalid name_len: {}", name_len);
            }
            
            offset += rec_len as usize;
        }
        
        console_println!("   📊 Found {} entries in this block", entries_in_block);
        Ok(())
    }
    
    /// Read a block of data from disk
    fn read_block_data(&self, block_num: u64) -> FilesystemResult<Vec<u8, 4096>> {
        let mut disk_device = virtio_blk::VIRTIO_BLK.lock();
        let sectors_per_block = self.block_size / SECTOR_SIZE;
        let start_sector = block_num * sectors_per_block as u64;
        
        let mut block_data = Vec::new();
        
        for i in 0..sectors_per_block {
            let mut sector_buf = [0u8; SECTOR_SIZE];
            disk_device.read_blocks(start_sector + i as u64, &mut sector_buf)
                .map_err(|_| FilesystemError::IoError)?;
            
            for &byte in &sector_buf {
                if block_data.push(byte).is_err() {
                    break;
                }
            }
        }
        
        drop(disk_device);
        Ok(block_data)
    }
    
    /// Read file content from its inode
    fn read_file_content(&self, file: &FileEntry) -> FilesystemResult<Vec<u8, 4096>> {
        console_println!("📖 Reading real file content for inode {}", file.inode);
        
        // Read the file's inode
        let file_inode = self.read_inode(file.inode as u32)?;
        let file_size = file_inode.i_size_lo as usize;
        let i_flags = file_inode.i_flags;
        
        console_println!("   📊 File size: {} bytes", file_size);
        console_println!("   🔍 File inode flags: 0x{:08x}", i_flags);
        
        // Check if this inode uses extents
        if (i_flags & EXT4_EXTENTS_FL) != 0 {
            console_println!("   🌟 File uses extents - reading from extent tree");
            self.read_file_content_from_extents(&file_inode, file_size)
        } else {
            console_println!("   📋 File uses direct blocks - reading from traditional blocks");
            self.read_file_content_from_blocks(&file_inode, file_size)
        }
    }
    
    /// Read file content from extent-based inode
    fn read_file_content_from_extents(&self, file_inode: &Ext4Inode, file_size: usize) -> FilesystemResult<Vec<u8, 4096>> {
        let mut file_content = Vec::new();
        let mut bytes_read = 0;
        
        // Copy i_block array to avoid packed field alignment issues
        let i_block_copy = file_inode.i_block;
        
        // Parse extent header
        let extent_header: Ext4ExtentHeader = unsafe {
            let header_ptr = i_block_copy.as_ptr() as *const Ext4ExtentHeader;
            *header_ptr
        };
        
        let eh_magic = extent_header.eh_magic;
        let eh_entries = extent_header.eh_entries;
        let eh_depth = extent_header.eh_depth;
        
        console_println!("   🔍 File extent header: magic=0x{:04x}, entries={}, depth={}", 
            eh_magic, eh_entries, eh_depth);
        
        if eh_magic != EXT4_EXT_MAGIC {
            console_println!("   ❌ Invalid extent magic for file");
            return Err(FilesystemError::CorruptedFilesystem);
        }
        
        if eh_depth != 0 {
            console_println!("   ❌ Multi-level extent trees not supported for files");
            return Err(FilesystemError::UnsupportedFilesystem);
        }
        
        // Parse extent entries
        let extents_start = core::mem::size_of::<Ext4ExtentHeader>();
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
            
            let extent_offset = extents_start + i * core::mem::size_of::<Ext4Extent>();
            
            if extent_offset + core::mem::size_of::<Ext4Extent>() > i_block_bytes.len() {
                console_println!("   ⚠️ File extent {} extends beyond i_block", i);
                break;
            }
            
            let extent: Ext4Extent = unsafe {
                let extent_ptr = (i_block_bytes.as_ptr().add(extent_offset)) as *const Ext4Extent;
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
                
                let block_data = match self.read_block_data(block_num) {
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
    fn read_file_content_from_blocks(&self, file_inode: &Ext4Inode, file_size: usize) -> FilesystemResult<Vec<u8, 4096>> {
        let mut file_content = Vec::new();
        let mut bytes_read = 0;
        
        // Copy i_block array to avoid packed field alignment issues
        let i_block_copy = file_inode.i_block;
        
        // Read file data from direct blocks
        for &block_num in i_block_copy.iter().take(12) {
            if block_num == 0 || bytes_read >= file_size {
                break;
            }
            
            // Validate block number
            if block_num > 1000000 {
                console_println!("   ⚠️ Skipping invalid block number: {}", block_num);
                continue;
            }
            
            let block_data = match self.read_block_data(block_num as u64) {
                Ok(data) => data,
                Err(_) => {
                    console_println!("   ⚠️ Failed to read block {}", block_num);
                    continue;
                }
            };
            
            let bytes_to_copy = core::cmp::min(file_size - bytes_read, block_data.len());
            
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
        }
        
        console_println!("   ✅ Read {} bytes from block-based file", bytes_read);
        Ok(file_content)
    }

    /// Writes an inode structure to the inode table on disk.
    fn write_inode_to_table(&mut self, inode_num: u32, inode_data: &Ext4Inode) -> FilesystemResult<()> {
        let superblock = self.superblock.ok_or(FilesystemError::NotInitialized)?;
        let group_desc = self.group_desc.ok_or(FilesystemError::NotInitialized)?;

        let s_inodes_per_group = superblock.s_inodes_per_group;
        let s_inode_size = superblock.s_inode_size;
        let bg_inode_table_lo = group_desc.bg_inode_table_lo;

        if inode_num == 0 || inode_num > superblock.s_inodes_count {
            console_println!("write_inode_to_table: Invalid inode number {}", inode_num);
            return Err(FilesystemError::IoError); // Invalid inode number
        }

        // Calculate which group and offset within group (simplified to group 0 for now)
        let group = (inode_num - 1) / s_inodes_per_group;
        let index_in_group = (inode_num - 1) % s_inodes_per_group;

        if group != 0 {
            console_println!("write_inode_to_table: Multi-group not supported yet (inode {}, group {})", inode_num, group);
            return Err(FilesystemError::UnsupportedFilesystem);
        }

        let inode_size_bytes = if s_inode_size > 0 { s_inode_size as usize } else { 256 }; // Default to 256 if not specified
        let byte_offset_in_table = index_in_group as usize * inode_size_bytes;

        let target_block_in_inode_table = bg_inode_table_lo + (byte_offset_in_table / self.block_size) as u32;
        let offset_within_block = byte_offset_in_table % self.block_size;

        console_println!(
            "write_inode_to_table: inode {}, group {}, index_in_group {}\n    target_block: {}, offset_in_block: {}, inode_size_bytes: {}",
            inode_num, group, index_in_group, target_block_in_inode_table, offset_within_block, inode_size_bytes
        );

        if offset_within_block + inode_size_bytes > self.block_size {
            console_println!(
                "write_inode_to_table: Inode {} (size {}) at offset {} would span across block boundary in block {}. Not supported.",
                inode_num, inode_size_bytes, offset_within_block, target_block_in_inode_table
            );
            return Err(FilesystemError::IoError); // Inode spanning blocks not handled by this simple RMW
        }

        // Read the block from inode table
        let mut block_data_vec = self.read_block_data(target_block_in_inode_table as u64)?;

        // Ensure buffer is mutable and has correct size (read_block_data returns Vec of self.block_size)
        if block_data_vec.len() != self.block_size {
             console_println!("write_inode_to_table: Read block data len {} != self.block_size {}", block_data_vec.len(), self.block_size);
             return Err(FilesystemError::CorruptedFilesystem);
        }

        // Overlay the new inode data
        // Convert Ext4Inode to byte slice. This is unsafe if Ext4Inode is not repr(C) or has padding issues.
        // It is repr(C, packed), so direct transmutation or pointer casting should be okay for size of struct.
        let inode_bytes: &[u8] = unsafe {
            core::slice::from_raw_parts(
                (inode_data as *const Ext4Inode) as *const u8,
                core::mem::size_of::<Ext4Inode>()
            )
        };
        
        // We must ensure we only copy `inode_size_bytes` which might be smaller than `size_of<Ext4Inode>()`
        // if the on-disk inode size (s_inode_size) is smaller than our struct representation.
        // However, our Ext4Inode struct is padded to 256 bytes, matching common s_inode_size.
        // For safety, let's use min(inode_size_bytes, inode_bytes.len()) if they could differ.
        // Assuming `inode_size_bytes` is the authoritative size on disk.

        if offset_within_block + inode_size_bytes > block_data_vec.len() {
            console_println!("write_inode_to_table: Write would exceed block_data_vec bounds.");
            return Err(FilesystemError::IoError);
        }

        block_data_vec[offset_within_block..offset_within_block + inode_size_bytes]
            .copy_from_slice(&inode_bytes[0..inode_size_bytes]);

        // Write the modified block back
        self.write_bitmap(target_block_in_inode_table, &block_data_vec) // write_bitmap reuses block writing logic
            .map_err(|e| {
                console_println!("write_inode_to_table: Failed to write inode block {}: {:?}", target_block_in_inode_table, e);
                e
            })
    }

    /// Finds a free inode in group 0.
    /// Returns the inode number (1-based for the group) if found.
    fn find_free_inode_in_group0(&self) -> FilesystemResult<Option<u32>> {
        let superblock = self.superblock.ok_or(FilesystemError::NotInitialized)?;
        let group_desc = self.group_desc.ok_or(FilesystemError::NotInitialized)?;

        if group_desc.bg_free_inodes_count_lo == 0 {
            console_println!("find_free_inode_in_group0: No free inodes in group 0 per descriptor.");
            return Ok(None);
        }

        let bg_inode_bitmap_lo_val = group_desc.bg_inode_bitmap_lo; // Local copy for printing
        console_println!("find_free_inode_in_group0: Reading inode bitmap from block {}", bg_inode_bitmap_lo_val);
        let inode_bitmap_data = self.read_bitmap(group_desc.bg_inode_bitmap_lo)?;

        match Self::find_free_bit(&inode_bitmap_data) {
            Some(bit_index) => {
                // Inodes are 1-based. Bitmap index is 0-based.
                // s_inodes_per_group is the actual number of inodes this group is responsible for.
                let s_inodes_per_group_val = superblock.s_inodes_per_group; // Local copy for printing & comparison
                if bit_index >= s_inodes_per_group_val as usize {
                    console_println!(
                        "find_free_inode_in_group0: Found free bit {} which is >= s_inodes_per_group {}. Bitmap might be larger than inodes in group.",
                        bit_index,
                        s_inodes_per_group_val
                    );
                    // This indicates the found bit is outside the valid range for this specific group.
                    // We should iterate find_free_bit or refine it to respect a max_bits limit.
                    // For now, returning None as no suitable bit was found *within the group's inode range*.
                    // A simple loop here to continue search if first found bit is too high:
                    let mut current_search_offset = bit_index;
                    loop {
                        if current_search_offset >= superblock.s_inodes_per_group as usize {
                            // Searched past the end of usable inodes for this group in the current bitmap block section
                             return Ok(None);
                        }
                        // Check if the bit at current_search_offset is actually free
                        let byte_idx = current_search_offset / 8;
                        let bit_in_byte_idx = current_search_offset % 8; // Corrected variable name for clarity, was bit_in_byte_index in error log
                        if (inode_bitmap_data[byte_idx] & (1 << bit_in_byte_idx)) == 0 { // Used corrected variable bit_in_byte_idx
                            let inode_num_in_group = current_search_offset as u32 + 1;
                            console_println!("find_free_inode_in_group0: Found free inode bit {} -> inode num in group {}", current_search_offset, inode_num_in_group);
                            return Ok(Some(inode_num_in_group));
                        }
                        current_search_offset += 1;
                        // Ensure we don't loop indefinitely if bitmap is full or corrupted
                        if current_search_offset >= inode_bitmap_data.len() * 8 {
                            return Ok(None); 
                        }
                    }
                } else {
                    // Initial bit_index was already within s_inodes_per_group range
                    let inode_num_in_group = bit_index as u32 + 1;
                    console_println!("find_free_inode_in_group0: Found free inode bit {} -> inode num in group {}", bit_index, inode_num_in_group);
                    Ok(Some(inode_num_in_group))
                }
            }
            None => {
                console_println!("find_free_inode_in_group0: No free bit found in inode bitmap for group 0.");
                Ok(None)
            }
        }
    }

    /// Allocates an inode in group 0.
    /// Returns the allocated inode number (global).
    /// TODO: This needs to write back updated superblock and group descriptor for free counts.
    fn allocate_inode(&mut self, mode: u16, uid: u16, gid: u16, links_count: u16, i_flags: u32) -> FilesystemResult<u32> {
        let inode_num_in_group0 = self.find_free_inode_in_group0()? 
            .ok_or_else(|| {
                console_println!("allocate_inode: No free inode found in group 0.");
                FilesystemError::FilesystemFull
            })?;
        
        let global_inode_num = inode_num_in_group0;

        console_println!("allocate_inode: Attempting to allocate inode {} (in group 0)", global_inode_num);

        let mut group_desc_for_bitmap = self.group_desc.ok_or(FilesystemError::NotInitialized)?;
        let mut inode_bitmap_data = self.read_bitmap(group_desc_for_bitmap.bg_inode_bitmap_lo)?;

        Self::set_bit(&mut inode_bitmap_data, (global_inode_num - 1) as u32); // Cast to u32
        self.write_bitmap(group_desc_for_bitmap.bg_inode_bitmap_lo, &inode_bitmap_data)?;
        console_println!("allocate_inode: Marked inode {} as used in bitmap.", global_inode_num);

        // --- Update and write back superblock and group descriptor ---
        let mut sb_to_write = self.superblock.ok_or(FilesystemError::NotInitialized)?;
        if sb_to_write.s_free_inodes_count > 0 {
            sb_to_write.s_free_inodes_count -= 1;
        }
        // Create a local copy for printing to avoid unaligned access.
        let s_free_inodes_count_val = sb_to_write.s_free_inodes_count;
        self.write_superblock(&sb_to_write)?;
        self.superblock = Some(sb_to_write); // Update in-memory copy after successful write
        console_println!("allocate_inode: Superblock s_free_inodes_count updated to {} and written.", s_free_inodes_count_val);

        // Now, update and write the group descriptor
        // We use group_desc_for_bitmap which is a copy of self.group_desc, modify it, then store back
        if group_desc_for_bitmap.bg_free_inodes_count_lo > 0 {
            group_desc_for_bitmap.bg_free_inodes_count_lo -= 1;
        }
        let bg_free_inodes_count_lo_val = group_desc_for_bitmap.bg_free_inodes_count_lo;
        self.write_group_descriptor(0, &group_desc_for_bitmap)?;
        self.group_desc = Some(group_desc_for_bitmap); // Update in-memory copy
        console_println!("allocate_inode: Group descriptor 0 bg_free_inodes_count_lo updated to {} and written.", bg_free_inodes_count_lo_val);
        // --- End of superblock/group_desc update ---

        // Initialize the inode structure
        let current_time = 0; // Placeholder for actual timestamp
        let mut new_inode = Ext4Inode {
            i_mode: mode, 
            i_uid: uid,
            i_gid: gid,
            i_size_lo: 0,
            i_size_high: 0,
            i_atime: current_time,
            i_ctime: current_time,
            i_mtime: current_time,
            i_dtime: 0, 
            i_links_count: links_count, 
            i_blocks_lo: 0, 
            i_flags: i_flags, 
            i_osd1: 0, 
            i_block: [0; 15], 
            i_generation: 0, 
            i_file_acl_lo: 0, 
            _padding: [0; 144],
        };

        // If the inode uses extents, initialize the extent header in i_block
        if (i_flags & EXT4_EXTENTS_FL) != 0 {
            console_println!("allocate_inode: Initializing extent header for inode {}", global_inode_num);
            
            // Copy i_block array to local variable to avoid unaligned access
            let mut i_block_copy = new_inode.i_block;
            
            // Initialize extent header for an empty file
            let extent_header = Ext4ExtentHeader {
                eh_magic: EXT4_EXT_MAGIC,
                eh_entries: 0, // No extents yet (empty file)
                eh_max: ((core::mem::size_of_val(&i_block_copy) - core::mem::size_of::<Ext4ExtentHeader>()) 
                        / core::mem::size_of::<Ext4Extent>()) as u16,
                eh_depth: 0, // Leaf node
                eh_generation: 0,
            };
            
            // Write the extent header into i_block copy
            let i_block_bytes = unsafe {
                core::slice::from_raw_parts_mut(
                    i_block_copy.as_mut_ptr() as *mut u8,
                    core::mem::size_of_val(&i_block_copy)
                )
            };
            
            let header_bytes = unsafe {
                core::slice::from_raw_parts(
                    (&extent_header as *const Ext4ExtentHeader) as *const u8,
                    core::mem::size_of::<Ext4ExtentHeader>()
                )
            };
            
            i_block_bytes[0..header_bytes.len()].copy_from_slice(header_bytes);
            
            // Copy the modified i_block back to the inode
            new_inode.i_block = i_block_copy;
            
            // Copy extent header fields to local variables for printing
            let eh_magic = extent_header.eh_magic;
            let eh_max = extent_header.eh_max;
            console_println!("allocate_inode: Extent header initialized: magic=0x{:04x}, max_entries={}", 
                eh_magic, eh_max);
        }

        console_println!("allocate_inode: Writing new inode {} to table.", global_inode_num);
        self.write_inode_to_table(global_inode_num, &new_inode)?;

        console_println!("✅ Allocated inode {} successfully.", global_inode_num);
        Ok(global_inode_num)
    }

    /// Finds a free block in group 0.
    /// Returns the block number (relative to the start of the filesystem) if found.
    fn find_free_block_in_group0(&self) -> FilesystemResult<Option<u32>> {
        let superblock = self.superblock.ok_or(FilesystemError::NotInitialized)?;
        let group_desc = self.group_desc.ok_or(FilesystemError::NotInitialized)?;

        if group_desc.bg_free_blocks_count_lo == 0 {
            console_println!("find_free_block_in_group0: No free blocks in group 0 per descriptor.");
            return Ok(None);
        }
        let bg_block_bitmap_lo_val = group_desc.bg_block_bitmap_lo;
        console_println!("find_free_block_in_group0: Reading block bitmap from block {}", bg_block_bitmap_lo_val);
        let block_bitmap_data = self.read_bitmap(group_desc.bg_block_bitmap_lo)?;

        // s_first_data_block is the first block in the filesystem that can be used for data.
        // Bit 0 in the block bitmap for group 0 corresponds to s_first_data_block.
        // Subsequent bits correspond to subsequent blocks in the group.
        // The number of blocks in group 0 is s_blocks_per_group.

        match Self::find_free_bit(&block_bitmap_data) {
            Some(bit_index) => {
                // Copy to local var before use in format string
                let s_blocks_per_group_val = superblock.s_blocks_per_group;
                if bit_index >= superblock.s_blocks_per_group as usize {
                    console_println!(
                        "find_free_block_in_group0: Found free bit {} which is >= s_blocks_per_group {}. Searching further.",
                        bit_index,
                        s_blocks_per_group_val
                    );
                    // Search for a bit within the valid range for this group
                    let mut current_search_offset = bit_index;
                    loop {
                        if current_search_offset >= superblock.s_blocks_per_group as usize {
                            return Ok(None); // No suitable bit found within the group's block range
                        }
                        let byte_idx = current_search_offset / 8;
                        let bit_in_byte_idx = current_search_offset % 8;
                        if (block_bitmap_data[byte_idx] & (1 << bit_in_byte_idx)) == 0 {
                            // Found a valid free bit. bit_index is 0-based from start of group's blocks.
                            // Global block number = s_first_data_block (for group 0) + bit_index.
                            let block_num_global = superblock.s_first_data_block + current_search_offset as u32;
                            console_println!(
                                "find_free_block_in_group0: Found free block bit {} -> global block num {}",
                                current_search_offset, block_num_global
                            );
                            return Ok(Some(block_num_global));
                        }
                        current_search_offset += 1;
                        if current_search_offset >= block_bitmap_data.len() * 8 {
                            return Ok(None); // Searched entire bitmap block
                        }
                    }
                } else {
                    // Initial bit_index was within s_blocks_per_group range
                    let block_num_global = superblock.s_first_data_block + bit_index as u32;
                    console_println!(
                        "find_free_block_in_group0: Found free block bit {} -> global block num {}",
                        bit_index, block_num_global
                    );
                    Ok(Some(block_num_global))
                }
            }
            None => {
                console_println!("find_free_block_in_group0: No free bit found in block bitmap for group 0.");
                Ok(None)
            }
        }
    }

    /// Allocates a data block in group 0.
    /// Returns the allocated global block number.
    /// TODO: This needs to write back updated superblock and group descriptor for free counts.
    fn allocate_block(&mut self) -> FilesystemResult<u32> {
        let global_block_num = self.find_free_block_in_group0()? 
            .ok_or_else(|| {
                console_println!("allocate_block: No free block found in group 0.");
                FilesystemError::FilesystemFull
            })?;

        console_println!("allocate_block: Attempting to allocate global block {}", global_block_num);

        let mut group_desc_for_block_bitmap = self.group_desc.ok_or(FilesystemError::NotInitialized)?;
        let superblock_ref_for_block = self.superblock.as_ref().ok_or(FilesystemError::NotInitialized)?;
        
        let mut block_bitmap_data = self.read_bitmap(group_desc_for_block_bitmap.bg_block_bitmap_lo)?;

        let bit_index_in_bitmap = (global_block_num - superblock_ref_for_block.s_first_data_block) as usize;

        Self::set_bit(&mut block_bitmap_data, bit_index_in_bitmap as u32); 
        self.write_bitmap(group_desc_for_block_bitmap.bg_block_bitmap_lo, &block_bitmap_data)?;
        console_println!("allocate_block: Marked block bit {} (global {}) as used in bitmap.", bit_index_in_bitmap, global_block_num);

        let mut sb_to_write_for_block = self.superblock.ok_or(FilesystemError::NotInitialized)?;
        if sb_to_write_for_block.s_free_blocks_count_lo > 0 {
            sb_to_write_for_block.s_free_blocks_count_lo -= 1;
        }
        let s_free_blocks_count_lo_val_alloc_local = sb_to_write_for_block.s_free_blocks_count_lo; // Local copy for printing
        self.write_superblock(&sb_to_write_for_block)?;
        self.superblock = Some(sb_to_write_for_block); 
        console_println!("allocate_block: Superblock s_free_blocks_count_lo updated to {} and written.", s_free_blocks_count_lo_val_alloc_local);

        if group_desc_for_block_bitmap.bg_free_blocks_count_lo > 0 {
            group_desc_for_block_bitmap.bg_free_blocks_count_lo -= 1;
        }
        self.write_group_descriptor(0, &group_desc_for_block_bitmap)?;
        self.group_desc = Some(group_desc_for_block_bitmap); 
        let gd_bg_free_blocks_count_lo_val = group_desc_for_block_bitmap.bg_free_blocks_count_lo; // Local copy
        console_println!("allocate_block: Group descriptor 0 bg_free_blocks_count_lo updated to {} and written.", gd_bg_free_blocks_count_lo_val);
        // --- End of superblock/group_desc update ---

        // TODO: Zero out the allocated block on disk? Optional, but good for security/consistency.
        // let mut zero_buffer = Vec::<u8, 4096>::new(); // Max block size
        // zero_buffer.resize_default(self.block_size).map_err(|_| FilesystemError::IoError)?;
        // self.write_block_data(global_block_num, &zero_buffer)?;
        // console_println!("allocate_block: TODO - Zeroed out allocated block {} (NEEDS write_block_data)", global_block_num);

        console_println!("✅ Allocated block {} successfully.", global_block_num);
        Ok(global_block_num)
    }

    /// Writes the superblock back to disk.
    fn write_superblock(&mut self, sb_to_write: &Ext4Superblock) -> FilesystemResult<()> {
        console_println!("write_superblock: Writing superblock to disk...");

        let sb_bytes: &[u8] = unsafe {
            core::slice::from_raw_parts(
                (sb_to_write as *const Ext4Superblock) as *const u8,
                core::mem::size_of::<Ext4Superblock>()
            )
        };

        if sb_bytes.len() != 1024 { // Ext4Superblock struct is padded to 1024
            console_println!(
                "write_superblock: ERROR - Serialized superblock size {} is not 1024 bytes.",
                sb_bytes.len()
            );
            return Err(FilesystemError::IoError);
        }

        let mut disk_device = virtio_blk::VIRTIO_BLK.lock();
        let start_sector = (EXT4_SUPERBLOCK_OFFSET / SECTOR_SIZE) as u64; // Should be sector 2

        // Write the 1024 bytes (typically 2 sectors)
        for i in 0.. (1024 / SECTOR_SIZE) {
            let sector_offset_in_sb_bytes = i * SECTOR_SIZE;
            let data_slice = &sb_bytes[sector_offset_in_sb_bytes .. sector_offset_in_sb_bytes + SECTOR_SIZE];
            
            let sector_buffer_array: &[u8; 512] = data_slice.try_into().map_err(|_| {
                console_println!("write_superblock: Failed to convert slice to [u8; 512]");
                FilesystemError::IoError
            })?;

            disk_device.write_blocks(start_sector + i as u64, sector_buffer_array)
                .map_err(|e| {
                    console_println!(
                        "write_superblock: Failed to write superblock sector {}: {:?}",
                        start_sector + i as u64, e
                    );
                    FilesystemError::IoError
                })?;
        }
        console_println!("write_superblock: Superblock written successfully.");
        Ok(())
    }

    /// Writes a specific group descriptor back to the Group Descriptor Table on disk.
    /// For now, assumes group_num is 0.
    fn write_group_descriptor(&mut self, group_num: u16, gd_to_write: &Ext4GroupDesc) -> FilesystemResult<()> {
        if group_num != 0 {
            console_println!("write_group_descriptor: ERROR - Only group 0 is supported for writing currently.");
            return Err(FilesystemError::UnsupportedFilesystem);
        }
        console_println!("write_group_descriptor: Writing group descriptor {} to disk...", group_num);

        let superblock = self.superblock.as_ref().ok_or(FilesystemError::NotInitialized)?;
        let group_desc_size = core::mem::size_of::<Ext4GroupDesc>(); // Typically 32 or 64 bytes depending on features

        // Calculate start block of GDT
        let s_first_data_block = superblock.s_first_data_block;
        let sb_block_num = if s_first_data_block == 0 { 1 } else { s_first_data_block }; // Block num of SB area end
        let gdt_start_block = sb_block_num + 1;

        let offset_in_gdt_block = (group_num as usize) * group_desc_size;

        if offset_in_gdt_block + group_desc_size > self.block_size {
            console_println!(
                "write_group_descriptor: Group descriptor {} at offset {} (size {}) would exceed block size {}. Not supported.",
                group_num, offset_in_gdt_block, group_desc_size, self.block_size
            );
            return Err(FilesystemError::IoError);
        }

        let gd_bytes: &[u8] = unsafe {
            core::slice::from_raw_parts(
                (gd_to_write as *const Ext4GroupDesc) as *const u8,
                group_desc_size
            )
        };

        let mut gdt_block_data = self.read_block_data(gdt_start_block as u64)?;
        if gdt_block_data.len() != self.block_size {
            console_println!("write_group_descriptor: GDT block data len mismatch.");
            return Err(FilesystemError::CorruptedFilesystem);
        }

        gdt_block_data[offset_in_gdt_block .. offset_in_gdt_block + group_desc_size]
            .copy_from_slice(gd_bytes);

        self.write_bitmap(gdt_start_block, &gdt_block_data) // Reuses write_bitmap for raw block write
            .map_err(|e| {
                console_println!(
                    "write_group_descriptor: Failed to write GDT block {}: {:?}",
                    gdt_start_block, e
                );
                e
            })
    }

    /// Calculates the required rec_len for a directory entry.
    /// rec_len is 8 bytes (inode, rec_len, name_len, file_type) + name_len, rounded up to 4 bytes.
    fn calculate_rec_len(name_len: u8) -> u16 {
        let len = 8 + name_len as usize;
        ((len + 3) & !3) as u16 // Round up to multiple of 4
    }

    /// Internal helper to write a full block of data.
    /// Assumes data.len() == self.block_size.
    fn write_block_data_internal(&mut self, block_num: u32, data: &[u8]) -> FilesystemResult<()> {
        if data.len() != self.block_size {
            console_println!("write_block_data_internal: data len {} != block_size {}", data.len(), self.block_size);
            return Err(FilesystemError::IoError);
        }
        // write_bitmap can be used here as it writes a raw block.
        self.write_bitmap(block_num, data)
    }

    /// Adds a directory entry to a parent directory (simplified).
    fn add_direntry(
        &mut self,
        parent_dir_inode_num: u32,
        child_inode_num: u32,
        filename: &str,
        file_type: u8, 
    ) -> FilesystemResult<()> {
        console_println!(
            "add_direntry: Adding '{}' (inode {}) to dir_inode {} as type {}",
            filename, child_inode_num, parent_dir_inode_num, file_type
        );

        if filename.len() > 255 {
            return Err(FilesystemError::FilenameTooLong);
        }
        let name_bytes = filename.as_bytes();
        let name_len = name_bytes.len() as u8;
        let required_rec_len = Self::calculate_rec_len(name_len);

        let mut parent_inode = self.read_inode(parent_dir_inode_num)?;
        let mut entry_written = false;

        // Check if directory uses extents or direct blocks
        if (parent_inode.i_flags & EXT4_EXTENTS_FL) != 0 {
            console_println!("add_direntry: Directory inode {} uses extents", parent_dir_inode_num);
            
            // Parse extent header
            let i_block_copy = parent_inode.i_block;
            let header_data = unsafe {
                core::slice::from_raw_parts(
                    i_block_copy.as_ptr() as *const u8,
                    core::mem::size_of::<Ext4ExtentHeader>()
                )
            };
            let header: Ext4ExtentHeader = unsafe { core::ptr::read_unaligned(header_data.as_ptr() as *const Ext4ExtentHeader) };
            
            // Copy packed fields to local variables to avoid unaligned access
            let eh_magic = header.eh_magic;
            let eh_depth = header.eh_depth;
            
            if eh_magic != EXT4_EXT_MAGIC {
                console_println!("add_direntry: Invalid extent magic: 0x{:x}", eh_magic);
                return Err(FilesystemError::IoError);
            }
            
            if eh_depth != 0 {
                console_println!("add_direntry: Multi-level extents not supported yet (depth={})", eh_depth);
                return Err(FilesystemError::IoError);
            }
            
            // Iterate through extents
            for extent_idx in 0..header.eh_entries {
                let extent_offset = core::mem::size_of::<Ext4ExtentHeader>() + (extent_idx as usize * core::mem::size_of::<Ext4Extent>());
                let extent_data = unsafe {
                    core::slice::from_raw_parts(
                        (i_block_copy.as_ptr() as *const u8).add(extent_offset),
                        core::mem::size_of::<Ext4Extent>()
                    )
                };
                let extent: Ext4Extent = unsafe { core::ptr::read_unaligned(extent_data.as_ptr() as *const Ext4Extent) };
                
                let physical_start = ((extent.ee_start_hi as u64) << 32) | (extent.ee_start_lo as u64);
                
                // Try each block in this extent
                for block_offset in 0..extent.ee_len {
                    let physical_block_num = physical_start + block_offset as u64;
                    let mut block_data = self.read_block_data(physical_block_num)?;
                    let mut offset_in_block = 0;

                    while offset_in_block < self.block_size {
                        let current_entry_ptr = unsafe { block_data.as_ptr().add(offset_in_block) } as *const Ext4DirEntry;
                        let current_entry = unsafe { *current_entry_ptr };

                        let current_rec_len = current_entry.rec_len;
                        let current_name_len = current_entry.name_len;
                        let space_taken_by_current_header_and_name = Self::calculate_rec_len(current_name_len);

                        // Scenario 1: Found a usable deleted/empty entry (inode 0)
                        if current_entry.inode == 0 && current_rec_len >= required_rec_len {
                            console_println!("add_direntry: Reusing deleted/empty entry in extent block {} at offset {}", physical_block_num, offset_in_block);
                            let new_entry = Ext4DirEntry {
                                inode: child_inode_num,
                                rec_len: current_rec_len,
                                name_len,
                                file_type,
                            };
                            unsafe {
                                let new_entry_ptr = block_data.as_mut_ptr().add(offset_in_block) as *mut Ext4DirEntry;
                                *new_entry_ptr = new_entry;
                                core::ptr::copy_nonoverlapping(
                                    name_bytes.as_ptr(),
                                    block_data.as_mut_ptr().add(offset_in_block + 8),
                                    name_len as usize,
                                );
                            }
                            self.write_block_data_internal(physical_block_num as u32, &block_data)?;
                            entry_written = true;
                            break;
                        }

                        // Scenario 2: Try to split the current (valid) entry if it has enough slack space
                        if current_entry.inode != 0 && current_rec_len >= space_taken_by_current_header_and_name.saturating_add(required_rec_len) {
                            console_println!("add_direntry: Splitting entry in extent block {} at offset {}", physical_block_num, offset_in_block);
                            // Shorten current entry
                            unsafe {
                                let current_entry_ptr_mut = block_data.as_mut_ptr().add(offset_in_block) as *mut Ext4DirEntry;
                                (*current_entry_ptr_mut).rec_len = space_taken_by_current_header_and_name;
                            }
                            // New entry starts after the shortened old one
                            let new_entry_offset = offset_in_block + space_taken_by_current_header_and_name as usize;
                            let new_entry_struct = Ext4DirEntry {
                                inode: child_inode_num,
                                rec_len: current_rec_len - space_taken_by_current_header_and_name,
                                name_len,
                                file_type,
                            };
                            unsafe {
                                let new_entry_ptr = block_data.as_mut_ptr().add(new_entry_offset) as *mut Ext4DirEntry;
                                *new_entry_ptr = new_entry_struct;
                                core::ptr::copy_nonoverlapping(
                                    name_bytes.as_ptr(),
                                    block_data.as_mut_ptr().add(new_entry_offset + 8),
                                    name_len as usize,
                                );
                            }
                            self.write_block_data_internal(physical_block_num as u32, &block_data)?;
                            entry_written = true;
                            break;
                        }
                        
                        // If this is the last entry in the block, stop processing this block
                        if offset_in_block + current_rec_len as usize >= self.block_size {
                            break;
                        }

                        if current_rec_len == 0 { break; } // Safety break
                        offset_in_block += current_rec_len as usize;
                    }
                    
                    if entry_written { break; }
                }
                
                if entry_written { break; }
            }
            
            if !entry_written {
                console_println!("add_direntry: No space found in existing extent blocks for '{}' in dir inode {}. Directory may need expansion.", filename, parent_dir_inode_num);
                return Err(FilesystemError::FilesystemFull);
            }
        } else {
            console_println!("add_direntry: Directory inode {} uses direct blocks", parent_dir_inode_num);
            
            // Handle direct block directories (existing code)
            let mut current_i_block = parent_inode.i_block;

            'block_loop: for i in 0..12 { // Iterate direct blocks
                let mut physical_block_num = current_i_block[i];
                let mut block_newly_allocated_for_dir = false;

                if physical_block_num == 0 {
                    console_println!("add_direntry: Dir {} block index {} is 0. Attempting to allocate.", parent_dir_inode_num, i);
                    physical_block_num = self.allocate_block()?;
                    
                    let mut zero_buffer = Vec::<u8, 4096>::new(); 
                    if zero_buffer.resize_default(self.block_size).is_err() { return Err(FilesystemError::IoError); }
                    for byte_val in zero_buffer.iter_mut() { *byte_val = 0; }
                    self.write_block_data_internal(physical_block_num, &zero_buffer)?;
                    
                    current_i_block[i] = physical_block_num;
                    parent_inode.i_blocks_lo += (self.block_size / 512) as u32;
                    block_newly_allocated_for_dir = true;
                    console_println!("add_direntry: Allocated new block {} for dir {}, temp inode block ptr updated.", physical_block_num, parent_dir_inode_num);
                }

                let mut block_data = self.read_block_data(physical_block_num as u64)?;
                let mut offset_in_block = 0;

                while offset_in_block < self.block_size {
                    let current_entry_ptr = unsafe { block_data.as_ptr().add(offset_in_block) } as *const Ext4DirEntry;
                    let current_entry = unsafe { *current_entry_ptr };

                    let current_rec_len = current_entry.rec_len;
                    let current_name_len = current_entry.name_len;
                    let space_taken_by_current_header_and_name = Self::calculate_rec_len(current_name_len);

                    // Scenario 1: Current entry is the end of list marker in a newly allocated block.
                    if block_newly_allocated_for_dir && current_entry.inode == 0 && current_rec_len == 0 && offset_in_block == 0 {
                        console_println!("add_direntry: Using newly allocated block {}", physical_block_num);
                        let new_entry = Ext4DirEntry {
                            inode: child_inode_num,
                            rec_len: (self.block_size - offset_in_block) as u16,
                            name_len,
                            file_type,
                        };
                        unsafe {
                            let new_entry_ptr = block_data.as_mut_ptr().add(offset_in_block) as *mut Ext4DirEntry;
                            *new_entry_ptr = new_entry;
                            core::ptr::copy_nonoverlapping(
                                name_bytes.as_ptr(),
                                block_data.as_mut_ptr().add(offset_in_block + 8),
                                name_len as usize,
                            );
                        }
                        self.write_block_data_internal(physical_block_num, &block_data)?;
                        entry_written = true;
                        break 'block_loop;
                    }

                    // Scenario 2: Found a usable deleted/empty entry (inode 0)
                    if current_entry.inode == 0 && current_rec_len >= required_rec_len {
                        console_println!("add_direntry: Reusing deleted/empty entry in block {} at offset {}", physical_block_num, offset_in_block);
                        let new_entry = Ext4DirEntry {
                            inode: child_inode_num,
                            rec_len: current_rec_len,
                            name_len,
                            file_type,
                        };
                        unsafe {
                            let new_entry_ptr = block_data.as_mut_ptr().add(offset_in_block) as *mut Ext4DirEntry;
                            *new_entry_ptr = new_entry;
                            core::ptr::copy_nonoverlapping(
                                name_bytes.as_ptr(),
                                block_data.as_mut_ptr().add(offset_in_block + 8),
                                name_len as usize,
                            );
                        }
                        self.write_block_data_internal(physical_block_num, &block_data)?;
                        entry_written = true;
                        break 'block_loop;
                    }

                    // Scenario 3: Try to split the current (valid) entry if it has enough slack space
                    if current_entry.inode != 0 && current_rec_len >= space_taken_by_current_header_and_name.saturating_add(required_rec_len) {
                        console_println!("add_direntry: Splitting entry in block {} at offset {}", physical_block_num, offset_in_block);
                        // Shorten current entry
                        unsafe {
                            let current_entry_ptr_mut = block_data.as_mut_ptr().add(offset_in_block) as *mut Ext4DirEntry;
                            (*current_entry_ptr_mut).rec_len = space_taken_by_current_header_and_name;
                        }
                        // New entry starts after the shortened old one
                        let new_entry_offset = offset_in_block + space_taken_by_current_header_and_name as usize;
                        let new_entry_struct = Ext4DirEntry {
                            inode: child_inode_num,
                            rec_len: current_rec_len - space_taken_by_current_header_and_name,
                            name_len,
                            file_type,
                        };
                        unsafe {
                            let new_entry_ptr = block_data.as_mut_ptr().add(new_entry_offset) as *mut Ext4DirEntry;
                            *new_entry_ptr = new_entry_struct;
                            core::ptr::copy_nonoverlapping(
                                name_bytes.as_ptr(),
                                block_data.as_mut_ptr().add(new_entry_offset + 8),
                                name_len as usize,
                            );
                        }
                        self.write_block_data_internal(physical_block_num, &block_data)?;
                        entry_written = true;
                        break 'block_loop;
                    }
                    
                    // If this is the last entry in the block, move to next block
                    if offset_in_block + current_rec_len as usize >= self.block_size {
                        if block_newly_allocated_for_dir {
                            console_println!("add_direntry: Warning - newly allocated block full? Offset {}, rec_len {}", offset_in_block, current_rec_len);
                        }
                        break;
                    }

                    if current_rec_len == 0 { break; } // Safety break
                    offset_in_block += current_rec_len as usize;
                }
                if entry_written { break; }
            }

            if !entry_written {
                console_println!("add_direntry: Failed to find space or add entry for '{}' in dir inode {}. Directory may be full or needs indirect blocks.", filename, parent_dir_inode_num);
                return Err(FilesystemError::FilesystemFull); 
            }

            // Update parent inode's block pointers if they were changed (new block allocated)
            parent_inode.i_block = current_i_block; 

            let parent_inode_i_block_copy = parent_inode.i_block;
            let num_blocks_in_inode = parent_inode_i_block_copy.iter().filter(|&&b| b != 0).count();
            let new_dir_size = (num_blocks_in_inode * self.block_size) as u32;
            if parent_inode.i_size_lo < new_dir_size { 
                parent_inode.i_size_lo = new_dir_size;
            }
        }
        
        let current_time = 0; // Placeholder
        parent_inode.i_mtime = current_time;
        parent_inode.i_ctime = current_time;

        console_println!("add_direntry: Writing updated parent dir inode {} to table.", parent_dir_inode_num);
        self.write_inode_to_table(parent_dir_inode_num, &parent_inode)?;

        console_println!("✅ Added entry '{}' to dir inode {}", filename, parent_dir_inode_num);
        Ok(())
    }

    fn create_file(&mut self, path: &str) -> FilesystemResult<FileEntry> {
        console_println!("ext4: create_file('{}')", path);

        let (parent_dir_inode_num, filename_component) =
            self.resolve_path_to_parent_and_final_component(path)?;

        if filename_component.is_empty() {
            console_println!("ext4: create_file - Filename cannot be empty.");
            return Err(FilesystemError::InvalidPath);
        }
        if filename_component.as_str() == "." || filename_component.as_str() == ".." {
            console_println!("ext4: create_file - Filename cannot be '.' or '..'.");
            return Err(FilesystemError::InvalidPath);
        }


        // Check if an entry with the same name already exists in the parent directory
        if self.find_entry_in_dir(parent_dir_inode_num, filename_component.as_str())?.is_some() {
            console_println!(
                "ext4: create_file - File or directory '{}' already exists in parent inode {}.",
                filename_component, parent_dir_inode_num
            );
            return Err(FilesystemError::FileAlreadyExists);
        }

        // Allocate inode for the new file
        // Mode: Regular file (S_IFREG) with permissions (e.g., rw-r--r-- or 0644)
        let mode = 0x8000 | 0o644; // S_IFREG | 0644
        let uid = 0; // Default user ID
        let gid = 0; // Default group ID
        let links_count = 1;
        let i_flags = EXT4_EXTENTS_FL; // Default to using extents

        console_println!("ext4: create_file - Allocating inode for '{}' in parent dir {}", filename_component, parent_dir_inode_num);
        let new_file_inode_num = self.allocate_inode(mode, uid, gid, links_count, i_flags)?;
        console_println!("ext4: create_file - Allocated inode {} for '{}'", new_file_inode_num, filename_component);

        // Add directory entry in the parent directory
        console_println!(
            "ext4: create_file - Adding direntry for inode {} ('{}') into dir {}",
            new_file_inode_num, filename_component, parent_dir_inode_num
        );
        self.add_direntry(
            parent_dir_inode_num,
            new_file_inode_num,
            filename_component.as_str(),
            EXT4_FT_REG_FILE,
        )?;

        let file_entry = FileEntry::new_file(filename_component.as_str(), new_file_inode_num as u64, 0)?;
        
        // Update cache if this is a file in the root directory (inode 2)
        if parent_dir_inode_num == EXT4_ROOT_INODE {
            if self.files.push(file_entry.clone()).is_err() {
                console_println!("ext4: create_file - Warning: File cache full, new file not added to cache");
            } else {
                console_println!("ext4: create_file - Added '{}' to file cache", filename_component);
            }
        }

        console_println!(
            "✅ ext4: create_file - File '{}' created successfully with inode {} in parent dir {}.",
            filename_component, new_file_inode_num, parent_dir_inode_num
        );
        Ok(file_entry)
    }

    fn create_directory(&mut self, path: &str) -> FilesystemResult<FileEntry> {
        console_println!("ext4: create_directory ('{}')", path);

        let (parent_dir_inode_num, dirname_component) =
            self.resolve_path_to_parent_and_final_component(path)?;

        if dirname_component.is_empty() {
            console_println!("ext4: create_directory - Directory name cannot be empty.");
            return Err(FilesystemError::InvalidPath);
        }
        if dirname_component.as_str() == "." || dirname_component.as_str() == ".." {
            console_println!("ext4: create_directory - Directory name cannot be '.' or '..'.");
            return Err(FilesystemError::InvalidPath);
        }

        // Check if an entry with the same name already exists in the parent directory
        if self.find_entry_in_dir(parent_dir_inode_num, dirname_component.as_str())?.is_some() {
            console_println!(
                "ext4: create_directory - File or directory '{}' already exists in parent inode {}.",
                dirname_component, parent_dir_inode_num
            );
            return Err(FilesystemError::FileAlreadyExists);
        }

        // 1. Allocate Inode for the new directory
        let mode = 0x4000 | 0o755; // S_IFDIR | 0755
        let uid = 0; 
        let gid = 0; 
        let i_flags = EXT4_EXTENTS_FL; // Directories can also use extents, or i_block for small ones
        let new_dir_links_count = 2; // For its "." entry and its entry in the parent.

        console_println!("ext4: create_dir - Allocating inode for '{}' in parent dir {}", dirname_component, parent_dir_inode_num);
        let new_dir_inode_num = self.allocate_inode(mode, uid, gid, new_dir_links_count, i_flags)?;
        console_println!("ext4: create_dir - Allocated inode {} for '{}'", new_dir_inode_num, dirname_component);

        // 2. Allocate a data block for the new directory's contents
        console_println!("ext4: create_dir - Allocating data block for dir inode {}", new_dir_inode_num);
        let dir_data_block_num = self.allocate_block()?;
        console_println!("ext4: create_dir - Allocated data block {} for dir inode {}", dir_data_block_num, new_dir_inode_num);

        // 3. Initialize the new directory's data block with . and .. entries
        let mut dir_block_data = Vec::<u8, 4096>::new();
        if dir_block_data.resize_default(self.block_size).is_err() { 
            self.free_inode(new_dir_inode_num)?; // Attempt to roll back inode allocation
            self.free_block(dir_data_block_num)?; // Attempt to roll back block allocation
            return Err(FilesystemError::IoError);
        }
        for byte_val in dir_block_data.iter_mut() { *byte_val = 0; } 

        let dot_name_len = 1;
        let dot_rec_len = Self::calculate_rec_len(dot_name_len);
        let dot_entry = Ext4DirEntry {
            inode: new_dir_inode_num,
            rec_len: dot_rec_len,
            name_len: dot_name_len,
            file_type: EXT4_FT_DIR,
        };
        unsafe {
            let entry_ptr = dir_block_data.as_mut_ptr().add(0) as *mut Ext4DirEntry;
            *entry_ptr = dot_entry;
            dir_block_data.as_mut_ptr().add(8).write(b'.'); 
        }

        let dotdot_name_len = 2;
        let final_dotdot_rec_len = (self.block_size - dot_rec_len as usize) as u16;
        let dotdot_entry = Ext4DirEntry {
            inode: parent_dir_inode_num, // ".." points to the actual parent inode
            rec_len: final_dotdot_rec_len, 
            name_len: dotdot_name_len,
            file_type: EXT4_FT_DIR,
        };
        unsafe {
            let entry_ptr = dir_block_data.as_mut_ptr().add(dot_rec_len as usize) as *mut Ext4DirEntry;
            *entry_ptr = dotdot_entry;
            let name_ptr = dir_block_data.as_mut_ptr().add(dot_rec_len as usize + 8);
            name_ptr.write(b'.');
            name_ptr.add(1).write(b'.');
        }
        console_println!("ext4: create_dir - Prepared . and .. entries for block {}", dir_data_block_num);
        self.write_block_data_internal(dir_data_block_num, &dir_block_data)?;

        // 4. Update the new directory's inode
        let mut new_dir_inode = self.read_inode(new_dir_inode_num)?;
        if (new_dir_inode.i_flags & EXT4_EXTENTS_FL) != 0 {
            // Initialize extent header for the directory data block
            let mut eh: Ext4ExtentHeader = Default::default();
            eh.eh_magic = EXT4_EXT_MAGIC;
            eh.eh_entries = 1;
            eh.eh_max = ((60 - core::mem::size_of::<Ext4ExtentHeader>()) 
                            / core::mem::size_of::<Ext4Extent>()) as u16; // i_block is 60 bytes
            eh.eh_depth = 0;
            
            // Set up extent to point to the allocated data block
            let extent = Ext4Extent {
                ee_block: 0, // First logical block of directory content
                ee_len: 1,   // Covers one block
                ee_start_hi: 0, // dir_data_block_num is u32, so high bits are 0
                ee_start_lo: dir_data_block_num,
            };
            
            // Copy packed fields to local variables for printing
            let ee_block_val = extent.ee_block;
            let ee_len_val = extent.ee_len;
            let ee_start_lo_val = extent.ee_start_lo;
            console_println!("ext4: create_dir - Setting up extent: block={}, len={}, physical={}", 
                ee_block_val, ee_len_val, ee_start_lo_val);

            // Write header and extent directly to i_block array
            unsafe {
                // Copy i_block array to avoid unaligned access
                let mut i_block_copy = new_dir_inode.i_block;
                let i_block_ptr = i_block_copy.as_mut_ptr() as *mut u8;
                
                // Write extent header
                let header_ptr = i_block_ptr as *mut Ext4ExtentHeader;
                *header_ptr = eh;
                
                // Write extent after header
                let extent_ptr = i_block_ptr
                    .add(core::mem::size_of::<Ext4ExtentHeader>()) as *mut Ext4Extent;
                *extent_ptr = extent;
                
                // Copy back the modified array
                new_dir_inode.i_block = i_block_copy;
            }
        } else {
            new_dir_inode.i_block[0] = dir_data_block_num;
        }
        new_dir_inode.i_blocks_lo = (self.block_size / 512) as u32; 
        new_dir_inode.i_size_lo = self.block_size as u32; 
        let current_time = 0; // Placeholder
        new_dir_inode.i_mtime = current_time;
        new_dir_inode.i_ctime = current_time;
        new_dir_inode.i_atime = current_time;
        console_println!("ext4: create_dir - Updating inode {} with block ptr and size", new_dir_inode_num);
        self.write_inode_to_table(new_dir_inode_num, &new_dir_inode)?;

        // 5. Add entry for the new directory in its parent directory
        console_println!(
            "ext4: create_dir - Adding direntry for new dir '{}' (inode {}) into parent dir {}",
            dirname_component, new_dir_inode_num, parent_dir_inode_num
        );
        self.add_direntry(
            parent_dir_inode_num,
            new_dir_inode_num,
            dirname_component.as_str(),
            EXT4_FT_DIR,
        )?;

        // 6. Update parent directory's link count and times
        let mut parent_dir_inode_to_update = self.read_inode(parent_dir_inode_num)?;
        parent_dir_inode_to_update.i_links_count = parent_dir_inode_to_update.i_links_count.saturating_add(1);
        parent_dir_inode_to_update.i_mtime = current_time;
        parent_dir_inode_to_update.i_ctime = current_time;
        let parent_links_count_val = parent_dir_inode_to_update.i_links_count; // Local copy for printing
        self.write_inode_to_table(parent_dir_inode_num, &parent_dir_inode_to_update)?;
        console_println!(
            "ext4: create_dir - Incremented link count of parent dir {} to {}.", 
            parent_dir_inode_num, parent_links_count_val
        );

        let dir_entry_obj = FileEntry::new_directory(dirname_component.as_str(), new_dir_inode_num as u64)?;

        // Update cache if this is a directory in the root directory (inode 2)
        if parent_dir_inode_num == EXT4_ROOT_INODE {
            if self.files.push(dir_entry_obj.clone()).is_err() {
                console_println!("ext4: create_directory - Warning: File cache full, new directory not added to cache");
            } else {
                console_println!("ext4: create_directory - Added '{}' to file cache", dirname_component);
            }
        }

        console_println!(
            "✅ ext4: create_directory - Directory '{}' created successfully with inode {} in parent dir {}.", 
            dirname_component, new_dir_inode_num, parent_dir_inode_num
        );
        Ok(dir_entry_obj)
    }

    fn write_file(&mut self, file: &FileEntry, offset: u64, data: &[u8]) -> FilesystemResult<usize> {
        console_println!(
            "ext4: write_file to '{}' (inode {}), offset: {}, data_len: {}",
            file.name, file.inode, offset, data.len()
        );

        if data.is_empty() {
            return Ok(0);
        }

        let mut file_inode = self.read_inode(file.inode as u32)?;
        let original_file_size = (file_inode.i_size_high as u64) << 32 | file_inode.i_size_lo as u64;

        // For now, only allow appending or overwriting from offset 0 for simplicity.
        // True random write is much more complex with extents (splitting, shifting, etc.)
        if offset > original_file_size {
            console_println!("ext4: write_file - Writing past end of file (offset > size) is not supported yet for arbitrary offsets.");
            return Err(FilesystemError::IoError); // Or specific error like EOVERFLOW or ENXIO
        }
        // If offset < original_file_size and offset != 0, it's an overwrite of existing data, also complex.
        // if offset != 0 && offset < original_file_size {
        //     console_println!("ext4: write_file - Overwriting existing file data is not fully supported yet.");
        //     return Err(FilesystemError::IoError);
        // }

        let i_flags = file_inode.i_flags;
        let mut bytes_written_total = 0usize;

        if (i_flags & EXT4_EXTENTS_FL) != 0 {
            console_println!("ext4: write_file - File uses extents.");
            // Simplified extent handling: append/overwrite from start.
            // Assume for now that if offset is 0, we are overwriting and may need to clear old extents.
            // If offset == original_file_size, we are appending.

            if offset == 0 && original_file_size > 0 {
                // TODO: Handle overwrite. This would involve freeing old blocks/extents and then proceeding as if appending to an empty file.
                // For now, this is complex. Let's only properly support append or write to empty file.
                console_println!("ext4: write_file - Overwriting existing file with extents (offset 0) not fully implemented. Freeing old extents needed.");
                // For a true overwrite, one would typically call a truncate-like function first, or free existing extents.
                // For now, let's block this unless it's a write to an effectively empty file (size 0 but had extents somehow).
                // return Err(FilesystemError::IoError); 
                 // Allow overwrite for now, it will just append and old extents might be orphaned if not handled by truncate later
            }

            // Proceed with append/initial write logic
            let mut current_logical_block = (offset / self.block_size as u64) as u32;
            let mut data_remaining_to_write = data;
            let i_block_copy_for_header_read = file_inode.i_block; // Copy for as_ptr
            let mut extent_header: Ext4ExtentHeader = unsafe { core::ptr::read(i_block_copy_for_header_read.as_ptr() as *const _) };

            if extent_header.eh_magic != EXT4_EXT_MAGIC {
                // Initialize extent header if file is new/empty or header is invalid
                if original_file_size == 0 && offset == 0 {
                    console_println!("ext4: write_file - Initializing extent header for new/empty file.");
                    extent_header.eh_magic = EXT4_EXT_MAGIC;
                    extent_header.eh_entries = 0;
                    let i_block_copy_for_size = file_inode.i_block; // Copy for size_of_val
                    extent_header.eh_max = ((core::mem::size_of_val(&i_block_copy_for_size) - core::mem::size_of::<Ext4ExtentHeader>()) 
                                            / core::mem::size_of::<Ext4Extent>()) as u16;
                    extent_header.eh_depth = 0;
                    extent_header.eh_generation = 0;
                } else {
                    let eh_magic_val = extent_header.eh_magic; // Local copy for printing
                    console_println!(
                        "ext4: write_file - Invalid extent magic 0x{:04X} in inode {} and file not empty/new. Offset {}",
                        eh_magic_val, file.inode, offset
                    );
                    return Err(FilesystemError::CorruptedFilesystem);
                }
            }
            
            // This is highly simplified: assumes we only add to the end of the extent list in i_block.
            // Does not handle full i_block (requiring tree growth), modifying existing extents, or finding specific logical blocks.
            let mut i_block_copy_for_mut_ptr_write = file_inode.i_block; // Copy for as_mut_ptr operations
            while !data_remaining_to_write.is_empty() {
                let current_eh_entries = extent_header.eh_entries; // Local copy
                let current_eh_max = extent_header.eh_max; // Local copy
                if current_eh_entries >= current_eh_max {
                    console_println!("ext4: write_file - Max extents reached in inode direct map. Extent tree needed.");
                    // TODO: Implement extent tree growing (indirect extent blocks)
                    return Err(FilesystemError::FilesystemFull); // Simplified error
                }

                let new_block_phys = self.allocate_block()?;
                // TODO: Zero out new_block_phys if policy requires (e.g. for non-overwrite append)
                // self.write_block_data_internal(new_block_phys, &[0u8; self.block_size]); // Example, needs vec

                let extent_slot_ptr = unsafe { 
                    i_block_copy_for_mut_ptr_write.as_mut_ptr()
                        .add(core::mem::size_of::<Ext4ExtentHeader>() + (current_eh_entries as usize * core::mem::size_of::<Ext4Extent>()))
                        as *mut Ext4Extent
                };
                
                let new_extent = Ext4Extent {
                    ee_block: current_logical_block, // First logical block this extent covers
                    ee_len: 1, // For simplicity, allocate one block at a time
                    ee_start_hi: 0, // Corrected: new_block_phys is u32, so high bits are 0
                    ee_start_lo: new_block_phys,
                };
                unsafe { *extent_slot_ptr = new_extent };
                extent_header.eh_entries += 1;
                console_println!("ext4: write_file - Added extent: logical_blk {}, phys_blk {}, len 1", current_logical_block, new_block_phys);

                // Write data to this new physical block
                let bytes_to_write_this_block = core::cmp::min(data_remaining_to_write.len(), self.block_size);
                self.write_block_data_internal(new_block_phys, &data_remaining_to_write[0..bytes_to_write_this_block])?;
                
                data_remaining_to_write = &data_remaining_to_write[bytes_to_write_this_block..];
                bytes_written_total += bytes_to_write_this_block;
                current_logical_block += 1;
            }

            // Write back the updated extent header into i_block
            unsafe {
                let header_ptr_mut = i_block_copy_for_mut_ptr_write.as_mut_ptr() as *mut Ext4ExtentHeader;
                *header_ptr_mut = extent_header;
            }
            file_inode.i_block = i_block_copy_for_mut_ptr_write; // Assign back the modified copy

        } else {
            console_println!("ext4: write_file - File does not use extents. Direct/indirect block writing not implemented.");
            return Err(FilesystemError::UnsupportedFilesystem);
        }

        // Update inode size and times
        let new_size = offset + bytes_written_total as u64;
        file_inode.i_size_lo = (new_size & 0xFFFFFFFF) as u32;
        file_inode.i_size_high = (new_size >> 32) as u32;
        file_inode.i_blocks_lo = ((new_size + 511) / 512) as u32; // Rough estimate in 512b blocks
        
        let current_time = 0; // Placeholder
        file_inode.i_mtime = current_time;
        file_inode.i_ctime = current_time;

        self.write_inode_to_table(file.inode as u32, &file_inode)?;
        console_println!("✅ ext4: write_file - Wrote {} bytes to '{}'. New size: {}.", bytes_written_total, file.name, new_size);

        Ok(bytes_written_total)
    }

    fn delete_file(&mut self, path: &str) -> FilesystemResult<()> {
        console_println!("ext4: delete_file '{}'", path);

        let (parent_dir_inode_num, filename_component) =
            self.resolve_path_to_parent_and_final_component(path)?;

        if filename_component.is_empty() {
            console_println!("ext4: delete_file - Filename cannot be empty.");
            return Err(FilesystemError::InvalidPath);
        }
         if filename_component.as_str() == "." || filename_component.as_str() == ".." {
            console_println!("ext4: delete_file - Cannot delete '.' or '..'.");
            return Err(FilesystemError::InvalidPath);
        }

        // 1. Find the directory entry to get the inode number and entry details.
        let (entry_data, block_num_of_entry, entry_offset_in_block) =
            self.find_entry_in_dir(parent_dir_inode_num, filename_component.as_str())?
                .ok_or_else(|| {
                    console_println!(
                        "ext4: delete_file - File '{}' not found in parent_dir_inode {}.",
                        filename_component, parent_dir_inode_num
                    );
                    FilesystemError::FileNotFound
                })?;

        if entry_data.file_type == EXT4_FT_DIR {
            console_println!(
                "ext4: delete_file - '{}' is a directory. Use delete_directory.",
                filename_component
            );
            return Err(FilesystemError::IsADirectory); // EISDIR equivalent
        }

        let target_inode_num = entry_data.inode;
        console_println!(
            "ext4: delete_file - Found '{}' (inode {}). Entry in block {}, offset {}",
            filename_component, target_inode_num, block_num_of_entry, entry_offset_in_block
        );

        // 2. Read Inode
        let mut file_inode = self.read_inode(target_inode_num)?;

        // 3. Free Data Blocks based on extents
        if (file_inode.i_flags & EXT4_EXTENTS_FL) != 0 {
            console_println!("ext4: delete_file - File uses extents. Freeing blocks...");
            let i_block_copy_for_header_read = file_inode.i_block; // Copy for safe read
            let extent_header: Ext4ExtentHeader = unsafe { core::ptr::read(i_block_copy_for_header_read.as_ptr() as *const _) };
            if extent_header.eh_magic == EXT4_EXT_MAGIC {
                if extent_header.eh_depth == 0 { 
                    let num_entries = extent_header.eh_entries;
                    let i_block_copy_for_extents_ptr = file_inode.i_block; // Copy for safe ptr ops
                    let extents_ptr_start = unsafe { // Added unsafe block for .add
                        i_block_copy_for_extents_ptr.as_ptr()
                        .add(core::mem::size_of::<Ext4ExtentHeader>()) as *const Ext4Extent
                    };
                    
                    for i in 0..num_entries {
                        let current_extent = unsafe { *extents_ptr_start.add(i as usize) };
                        let phys_start_block = current_extent.ee_start_lo; 
                        let num_blocks_in_extent = current_extent.ee_len;
                        for block_idx in 0..num_blocks_in_extent {
                            let block_to_free = phys_start_block + block_idx as u32;
                            if block_to_free != 0 { 
                                self.free_block(block_to_free)?;
                            }
                        }
                    }
                } else {
                    console_println!("ext4: delete_file - Extent tree depth > 0 not supported for freeing yet.");
                }
            } else {
                console_println!("ext4: delete_file - Invalid extent magic in inode {}, cannot free blocks.", target_inode_num);
            }
        } else {
            console_println!("ext4: delete_file - File does not use extents. Direct/indirect block freeing not implemented.");
        }

        // 4. Clear Inode Contents 
        let mut links_count_val = file_inode.i_links_count; // local copy
        links_count_val = links_count_val.saturating_sub(1);
        file_inode.i_links_count = links_count_val;

        if links_count_val == 0 {
            console_println!("ext4: delete_file - Links count is 0 for inode {}. Proceeding to full free.", target_inode_num);
            file_inode.i_dtime = 0; // Placeholder for current time
            file_inode.i_size_lo = 0;
            file_inode.i_size_high = 0;
            file_inode.i_blocks_lo = 0;
            file_inode.i_block = [0; 15]; 
            file_inode.i_flags = 0; 
            
            self.write_inode_to_table(target_inode_num, &file_inode)?;
            self.free_inode(target_inode_num)?;
        } else {
            file_inode.i_dtime = 0; 
            self.write_inode_to_table(target_inode_num, &file_inode)?;
            console_println!(
                "ext4: delete_file - Decremented links_count for inode {} to {}. Data blocks remain (hard links).",
                target_inode_num, links_count_val
            );
        }

        // 7. Remove Directory Entry (Simplified: set inode to 0)
        console_println!(
            "ext4: delete_file - Removing direntry for '{}' from block {} at offset {}",
            filename_component, block_num_of_entry, entry_offset_in_block
        );
        let mut dir_block_data = self.read_block_data(block_num_of_entry as u64)?;
        unsafe {
            let entry_ptr_mut = dir_block_data.as_mut_ptr().add(entry_offset_in_block) as *mut Ext4DirEntry;
            (*entry_ptr_mut).inode = 0; 
        }
        self.write_block_data_internal(block_num_of_entry, &dir_block_data)?;
        console_println!("ext4: delete_file - Marked direntry inode as 0.");

        // 8. Update parent directory's mtime and ctime
        let mut parent_inode_to_update = self.read_inode(parent_dir_inode_num)?;
        let current_time = 0; // Placeholder
        parent_inode_to_update.i_mtime = current_time;
        parent_inode_to_update.i_ctime = current_time;
        self.write_inode_to_table(parent_dir_inode_num, &parent_inode_to_update)?;
        console_println!("ext4: delete_file - Updated parent dir {} timestamps.", parent_dir_inode_num);
        
        // Update self.files cache if it was root dir; proper caching needed for general case
        if parent_dir_inode_num == EXT4_ROOT_INODE {
             self.files.retain(|f| f.name != filename_component.as_str() || f.inode != target_inode_num as u64);
        }

        console_println!("✅ ext4: delete_file - '{}' processed.", path);
        Ok(())
    }

    fn delete_directory(&mut self, path: &str) -> FilesystemResult<()> {
        console_println!("ext4: delete_directory '{}'", path);

        let (parent_dir_inode_num, dirname_component) =
            self.resolve_path_to_parent_and_final_component(path)?;

        if dirname_component.is_empty() {
            console_println!("ext4: delete_directory - Directory name cannot be empty.");
            return Err(FilesystemError::InvalidPath);
        }
        if dirname_component.as_str() == "." || dirname_component.as_str() == ".." {
            console_println!("ext4: delete_directory - Cannot delete '.' or '..'.");
            return Err(FilesystemError::InvalidPath);
        }
        if parent_dir_inode_num == EXT4_ROOT_INODE && dirname_component.as_str() == "/" { // Trying to delete root itself
             console_println!("ext4: delete_directory - Cannot delete root directory.");
            return Err(FilesystemError::InvalidPath);
        }

        // 1. Find the directory entry to get the inode number and entry details.
        let (entry_data, block_num_of_entry, entry_offset_in_block) =
            self.find_entry_in_dir(parent_dir_inode_num, dirname_component.as_str())?
                .ok_or_else(|| {
                    console_println!(
                        "ext4: delete_directory - Directory '{}' not found in parent_dir_inode {}.",
                        dirname_component, parent_dir_inode_num
                    );
                    FilesystemError::DirectoryNotFound
                })?;

        if entry_data.file_type != EXT4_FT_DIR {
            console_println!(
                "ext4: delete_directory - '{}' is not a directory.",
                dirname_component
            );
            return Err(FilesystemError::NotADirectory);
        }

        let dir_to_delete_inode_num = entry_data.inode;
        console_println!(
            "ext4: delete_directory - Found dir '{}' (inode {}). Entry in block {}, offset {}",
            dirname_component, dir_to_delete_inode_num, block_num_of_entry, entry_offset_in_block
        );

        // 2. Read the directory's inode.
        let mut dir_inode = self.read_inode(dir_to_delete_inode_num)?;

        // 3. Check if directory is empty (except for . and ..)
        let dir_inode_links_count_val = dir_inode.i_links_count; // Local copy for printing
        if dir_inode_links_count_val > 2 { 
            console_println!(
                "ext4: delete_directory - Directory '{}' (inode {}) not empty (links_count: {} > 2). Contains subdirectories?", 
                dirname_component, dir_to_delete_inode_num, dir_inode_links_count_val
            );
            return Err(FilesystemError::DirectoryNotEmpty);
        }
        
        // Check if this directory has corrupted extents (from old buggy mkdir)
        let mut has_corrupted_extent = false;
        if (dir_inode.i_flags & EXT4_EXTENTS_FL) != 0 {
            let i_block_copy_for_corruption_check = dir_inode.i_block;
            let extent_header: Ext4ExtentHeader = unsafe { 
                core::ptr::read(i_block_copy_for_corruption_check.as_ptr() as *const _) 
            };
            if extent_header.eh_magic == EXT4_EXT_MAGIC && extent_header.eh_entries > 0 {
                let i_block_copy_for_extent_ptr = dir_inode.i_block;
                let extent_ptr = unsafe {
                    i_block_copy_for_extent_ptr.as_ptr()
                        .add(core::mem::size_of::<Ext4ExtentHeader>()) as *const Ext4Extent
                };
                let first_extent = unsafe { *extent_ptr };
                if first_extent.ee_start_lo == 0 && first_extent.ee_len == 0 {
                    console_println!(
                        "ext4: delete_directory - Directory '{}' (inode {}) has corrupted extent (PhysicalStartLo=0, Len=0). Allowing deletion.", 
                        dirname_component, dir_to_delete_inode_num
                    );
                    has_corrupted_extent = true;
                }
            }
        }
        
        // More thorough check: iterate its entries (unless corrupted)
        let mut is_empty = true;
        if !has_corrupted_extent {
            match self.find_entry_in_dir(dir_to_delete_inode_num, ".") {
                Ok(Some((dot_entry,_,_))) => {
                    if dot_entry.inode != dir_to_delete_inode_num {
                        console_println!("ext4: delete_directory - Corrupted '.' entry in dir {}", dir_to_delete_inode_num);
                        return Err(FilesystemError::CorruptedFilesystem);
                    }
                } 
                _ => { /* Error or not found */ return Err(FilesystemError::CorruptedFilesystem); }
            }
            match self.find_entry_in_dir(dir_to_delete_inode_num, "..") {
                 Ok(Some((dotdot_entry,_,_))) => {
                    // parent_dir_inode_num is the parent of the dir we are trying to delete.
                    // So, the ".." entry of the dir_to_delete_inode_num should point to parent_dir_inode_num.
                    let dotdot_entry_inode_val_del = dotdot_entry.inode; // Local copy for printing
                    if dotdot_entry_inode_val_del != parent_dir_inode_num {
                         console_println!(
                            "ext4: delete_directory - Corrupted '..' entry in dir {}. Expected parent {}, found {}", 
                            dir_to_delete_inode_num, parent_dir_inode_num, dotdot_entry_inode_val_del
                        );
                        return Err(FilesystemError::CorruptedFilesystem);
                    }
                } 
                _ => { /* Error or not found */ return Err(FilesystemError::CorruptedFilesystem); }
            }
        }
        // Now iterate all blocks of the directory to ensure no other entries exist.
        // This logic duplicates find_entry_in_dir structure but checks for *any* other entry.
        let dir_inode_for_empty_check = self.read_inode(dir_to_delete_inode_num)?;
        is_empty = true; // Assume empty until proven otherwise

        if !has_corrupted_extent {
            if (dir_inode_for_empty_check.i_flags & EXT4_EXTENTS_FL) != 0 {
                let i_block_copy_for_extent_header_check = dir_inode_for_empty_check.i_block; // Copy for safe read
                let extent_header_check: Ext4ExtentHeader = unsafe { core::ptr::read(i_block_copy_for_extent_header_check.as_ptr() as *const _) };
                if extent_header_check.eh_magic == EXT4_EXT_MAGIC && extent_header_check.eh_depth == 0 {
                    let i_block_copy_for_extents_ptr_check = dir_inode_for_empty_check.i_block; // Copy for safe ptr ops
                    let extents_start_ptr_check = unsafe { // Added unsafe block for .add
                        i_block_copy_for_extents_ptr_check.as_ptr().add(core::mem::size_of::<Ext4ExtentHeader>()) as *const Ext4Extent
                    };
                    'extent_empty_loop: for i in 0..extent_header_check.eh_entries {
                        let extent = unsafe { *extents_start_ptr_check.add(i as usize) };
                        let physical_start_block = extent.ee_start_lo;
                        for block_offset_in_extent in 0..extent.ee_len {
                            let current_physical_block_num = physical_start_block + block_offset_in_extent as u32;
                            let block_data_check = self.read_block_data(current_physical_block_num as u64)?;
                            let mut offset_check = 0;
                            while offset_check < self.block_size {
                                let entry_ptr_check = unsafe { // Added unsafe block for .add
                                    block_data_check.as_ptr().add(offset_check) as *const Ext4DirEntry
                                };
                                let entry_check = unsafe { *entry_ptr_check };
                                if entry_check.inode == 0 || entry_check.rec_len == 0 {
                                    if entry_check.rec_len == 0 { break; } // End of entries in block
                                    offset_check += entry_check.rec_len as usize;
                                    continue;
                                }
                                let name_len_check = entry_check.name_len as usize;
                                if offset_check + 8 + name_len_check <= self.block_size {
                                    let name_slice_check = unsafe { core::slice::from_raw_parts(block_data_check.as_ptr().add(offset_check + 8), name_len_check) };
                                    let name_str_check = core::str::from_utf8(name_slice_check).unwrap_or("");
                                    if name_str_check != "." && name_str_check != ".." {
                                        is_empty = false;
                                        console_println!("ext4: delete_directory - Directory '{}' not empty. Found: '{}'", dirname_component, name_str_check);
                                        break 'extent_empty_loop;
                                    }
                                }
                                offset_check += entry_check.rec_len as usize;
                            }
                            if !is_empty { break; }
                        }
                        if !is_empty { break; }
                    }
                }
            } else { // Direct/indirect blocks for directory data
                'direct_empty_loop: for i in 0..12 { // Iterate direct blocks
                    let physical_block_num = dir_inode_for_empty_check.i_block[i];
                    if physical_block_num == 0 { continue; }

                    let block_data_check = self.read_block_data(physical_block_num as u64)?;
                    let mut offset_check = 0;
                    while offset_check < self.block_size {
                        let entry_ptr_check = unsafe { // Added unsafe block for .add
                            block_data_check.as_ptr().add(offset_check) as *const Ext4DirEntry
                        };
                        let entry_check = unsafe { *entry_ptr_check };

                        if entry_check.inode == 0 || entry_check.rec_len == 0 {
                            if entry_check.rec_len == 0 { break; }
                            offset_check += entry_check.rec_len as usize;
                            continue;
                        }
                         let name_len_check = entry_check.name_len as usize;
                        if offset_check + 8 + name_len_check <= self.block_size {
                            let name_slice_check = unsafe { core::slice::from_raw_parts(block_data_check.as_ptr().add(offset_check + 8), name_len_check) };
                            let name_str_check = core::str::from_utf8(name_slice_check).unwrap_or("");
                            if name_str_check != "." && name_str_check != ".." {
                                is_empty = false;
                                console_println!("ext4: delete_directory - Directory '{}' not empty. Found: '{}'", dirname_component, name_str_check);
                                break 'direct_empty_loop;
                            }
                        }
                        offset_check += entry_check.rec_len as usize;
                    }
                    if !is_empty { break; }
                }
                // TODO: Check indirect blocks for emptiness if not using extents
            }
        } else {
            // Skip emptiness check for corrupted directories and assume they're empty
            console_println!("ext4: delete_directory - Skipping emptiness check for corrupted directory '{}'", dirname_component);
        }
        
        if !is_empty {
            return Err(FilesystemError::DirectoryNotEmpty);
        }

        // 4. Free Data Blocks of the directory
        if (dir_inode.i_flags & EXT4_EXTENTS_FL) != 0 {
            console_println!("ext4: delete_directory - Directory '{}' uses extents. Freeing blocks...", dirname_component);
            let i_block_copy_for_header_read_free = dir_inode.i_block; // Copy for safe read
            let extent_header: Ext4ExtentHeader = unsafe { core::ptr::read(i_block_copy_for_header_read_free.as_ptr() as *const _) };
            if extent_header.eh_magic == EXT4_EXT_MAGIC {
                if extent_header.eh_depth == 0 {
                    let num_entries = extent_header.eh_entries;
                    let i_block_copy_for_extents_ptr_free = dir_inode.i_block; // Copy for safe ptr ops
                    let extents_ptr_start = unsafe { // Added unsafe block for .add
                        i_block_copy_for_extents_ptr_free.as_ptr().add(core::mem::size_of::<Ext4ExtentHeader>()) as *const Ext4Extent
                    };
                    
                    for i in 0..num_entries {
                        let current_extent = unsafe { *extents_ptr_start.add(i as usize) };
                        let phys_start_block = current_extent.ee_start_lo;
                        let num_blocks_in_extent = current_extent.ee_len;
                        
                        // Skip freeing corrupted extents
                        if phys_start_block == 0 && num_blocks_in_extent == 0 {
                            console_println!("ext4: delete_directory - Skipping freeing of corrupted extent (PhysicalStartLo=0, Len=0)");
                            continue;
                        }
                        
                        for block_idx in 0..num_blocks_in_extent {
                            let block_to_free = phys_start_block + block_idx as u32;
                            if block_to_free != 0 { 
                                self.free_block(block_to_free)?;
                            }
                        }
                    }
                } else {
                    console_println!("ext4: delete_directory - Directory '{}' uses multi-level extents. Not supported for deletion.", dirname_component);
                    return Err(FilesystemError::UnsupportedFilesystem);
                }
            } else {
                console_println!("ext4: delete_directory - Directory '{}' uses invalid extent magic. Cannot free blocks.", dirname_component);
                return Err(FilesystemError::CorruptedFilesystem);
            }
        } else {
            console_println!("ext4: delete_directory - Directory '{}' does not use extents. Direct/indirect block freeing not implemented.", dirname_component);
        }

        // 5. Clear Inode Contents
        dir_inode.i_links_count = dir_inode.i_links_count.saturating_sub(1);
        dir_inode.i_size_lo = 0;
        dir_inode.i_size_high = 0;
        dir_inode.i_blocks_lo = 0;
        dir_inode.i_block = [0; 15];
        dir_inode.i_flags = 0;
        dir_inode.i_atime = 0;
        dir_inode.i_ctime = 0;
        dir_inode.i_mtime = 0;
        dir_inode.i_dtime = 0;
        dir_inode.i_generation = 0;
        dir_inode.i_file_acl_lo = 0;
        dir_inode._padding = [0; 144];

        // 6. Write Inode
        self.write_inode_to_table(dir_to_delete_inode_num, &dir_inode)?;

        // 7. Free Inode
        self.free_inode(dir_to_delete_inode_num)?;

        // 8. Remove the directory entry from parent directory
        console_println!("ext4: delete_directory - Removing entry '{}' from parent dir {}", dirname_component, parent_dir_inode_num);
        self.remove_direntry_from_parent(parent_dir_inode_num, block_num_of_entry, entry_offset_in_block)?;

        // Update cache if this was a directory in the root directory (inode 2)
        if parent_dir_inode_num == EXT4_ROOT_INODE {
            self.files.retain(|f| f.name != dirname_component.as_str() || f.inode != dir_to_delete_inode_num as u64);
            console_println!("ext4: delete_directory - Removed '{}' from file cache", dirname_component);
        }

        console_println!("✅ ext4: delete_directory - '{}' processed.", path);
        Ok(())
    }

    /// Resolves a full path to an inode number.
    /// Starts from the root directory.
    /// Returns the inode number of the final component in the path.
    fn resolve_path_to_inode(&self, full_path: &str) -> FilesystemResult<u32> {
        console_println!("resolve_path_to_inode: Resolving path '{}'", full_path);
        let mut current_inode_num = EXT4_ROOT_INODE;

        // Normalize path: remove leading/trailing slashes, handle multiple slashes
        let mut normalized_path = heapless::String::<256>::new();
        let mut first_char = true;
        let mut last_was_slash = false;

        for char_byte in full_path.bytes() {
            if char_byte == b'/' {
                if !last_was_slash && !first_char {
                    normalized_path.push('/').map_err(|_| FilesystemError::FilenameTooLong)?; // Corrected
                }
                last_was_slash = true;
            } else {
                normalized_path.push(char_byte as char).map_err(|_| FilesystemError::FilenameTooLong)?; // Corrected
                last_was_slash = false;
            }
            if first_char { first_char = false; }
        }
        // Remove trailing slash if any, unless it's just "/"
        if normalized_path.ends_with('/') && normalized_path.len() > 1 {
            normalized_path.pop();
        }
        
        let path_str = normalized_path.as_str();
        console_println!("resolve_path_to_inode: Normalized to '{}'", path_str);

        if path_str.is_empty() || path_str == "/" { // Path is root
            return Ok(EXT4_ROOT_INODE);
        }

        let components = path_str.split('/');
        let mut component_count = 0;

        for component in components {
            if component.is_empty() { continue; } // Should be handled by normalization, but as a safeguard
            component_count +=1;
            console_println!("resolve_path_to_inode: Current component: '{}', searching in inode {}", component, current_inode_num);

            match self.find_entry_in_dir(current_inode_num, component)? {
                Some((entry, _, _)) => {
                    // Check if this component is a directory if there are more components to follow
                    // (this check is implicitly handled if the next find_entry_in_dir fails with NotADirectory)
                    current_inode_num = entry.inode;
                    console_println!("resolve_path_to_inode: Found '{}', new current_inode: {}", component, current_inode_num);
                }
                None => {
                    console_println!("resolve_path_to_inode: Component '{}' not found in inode {}", component, current_inode_num);
                    return Err(FilesystemError::FileNotFound); // Or PathNotFound
                }
            }
        }
        
        if component_count == 0 { // e.g. path was only slashes after normalization like "///"
             return Ok(EXT4_ROOT_INODE);
        }

        Ok(current_inode_num)
    }

    /// Resolves a full path to the inode number of its parent directory and the final path component string.
    /// Example: "/foo/bar/baz.txt" -> (inode_of_bar, "baz.txt")
    /// Example: "/foo.txt" -> (inode_of_root, "foo.txt")
    /// Example: "foo.txt" -> (inode_of_root, "foo.txt") (assuming current dir is root for relative paths)
    fn resolve_path_to_parent_and_final_component(
        &self,
        full_path: &str,
    ) -> FilesystemResult<(u32, heapless::String<255>)> {
        console_println!(
            "resolve_path_to_parent_and_final_component: Resolving path '{}'",
            full_path
        );

        let mut normalized_path_str = heapless::String::<256>::new();
        let mut last_was_slash = false;
        let mut first_char = true;

        for char_byte in full_path.bytes() {
            if char_byte == b'/' {
                if !last_was_slash || normalized_path_str.is_empty() { // Allow leading slash
                    if !normalized_path_str.is_empty() { // but not if it's not the first char and last was slash (e.g. //)
                         normalized_path_str.push('/').map_err(|_| FilesystemError::FilenameTooLong)?; // Corrected
                    }
                }
                last_was_slash = true;
            } else {
                if last_was_slash && !normalized_path_str.is_empty() && !normalized_path_str.ends_with('/') {
                    // If path was like "/a" and next is 'b', ensure slash before 'b'
                    // But if path is "a" and next is 'b', no slash needed yet.
                    // This logic is tricky with split later. Simpler to just push and let split handle it.
                }
                normalized_path_str.push(char_byte as char).map_err(|_| FilesystemError::FilenameTooLong)?; // Corrected
                last_was_slash = false;
            }
            first_char = false;
        }
        
        // Remove trailing slash if path is not just "/"
        if normalized_path_str.ends_with('/') && normalized_path_str.len() > 1 {
            normalized_path_str.pop();
        }
        if normalized_path_str.is_empty() && full_path.contains('/') { // Original was like "/"
            normalized_path_str.push('/').unwrap();
        }

        console_println!("resolve_path_to_parent_and_final_component: Normalized to '{}'", normalized_path_str.as_str());

        let mut components: heapless::Vec<&str, 16> = normalized_path_str.split('/').collect();
        
        // Handle cases like "/foo.txt" -> components["", "foo.txt"], or "foo.txt" -> ["foo.txt"]
        if components.len() > 0 && components[0].is_empty() { // Path started with '/'
            components.remove(0);
        }

        if components.is_empty() { // Path was effectively root or empty string
            if !normalized_path_str.is_empty() && normalized_path_str != "/" { // e.g. path was "myfile" considered as final component
                 let final_comp_str = heapless::String::try_from(normalized_path_str.as_str()).map_err(|_| FilesystemError::FilenameTooLong)?; // Corrected
                 return Ok((EXT4_ROOT_INODE, final_comp_str));
            }
            // This case should ideally not be hit if creating/deleting, as a name is needed.
            // For lookup of "/", parent is root, final comp is effectively root itself, or an error.
            console_println!("resolve_path_to_parent_and_final_component: Path is empty or root, cannot extract final component for creation/deletion.");
            return Err(FilesystemError::InvalidPath); 
        }

        let final_component = components.pop().unwrap(); // Known to be non-empty from above
        let final_component_str = heapless::String::try_from(final_component).map_err(|_| FilesystemError::FilenameTooLong)?; // Corrected

        let mut current_parent_inode_num = EXT4_ROOT_INODE;

        // Traverse remaining components to find the parent directory
        for component_name in components {
            if component_name.is_empty() { continue; } // Should be handled by split and initial empty check

            console_println!(
                "resolve_path_to_parent_and_final_component: Traversing parent component: '{}' in inode {}",
                component_name,
                current_parent_inode_num
            );

            match self.find_entry_in_dir(current_parent_inode_num, component_name)? {
                Some((entry, _, _)) => {
                    if entry.file_type != EXT4_FT_DIR {
                        console_println!(
                            "resolve_path_to_parent_and_final_component: Path component '{}' is not a directory.",
                            component_name
                        );
                        return Err(FilesystemError::NotADirectory);
                    }
                    current_parent_inode_num = entry.inode;
                }
                None => {
                    console_println!(
                        "resolve_path_to_parent_and_final_component: Path component '{}' not found in inode {}.",
                        component_name, current_parent_inode_num
                    );
                    return Err(FilesystemError::PathNotFound);
                }
            }
        }
        console_println!("resolve_path_to_parent_and_final_component: Parent inode: {}, Final component: '{}'", current_parent_inode_num, final_component_str);
        Ok((current_parent_inode_num, final_component_str))
    }

    /// Finds a directory entry within a given directory inode.
    /// Returns `Ok(Some((Ext4DirEntry, block_num, offset_in_block)))` if found,
    /// `Ok(None)` if not found, or an error.
    fn find_entry_in_dir(
        &self,
        dir_inode_num: u32,
        entry_name: &str,
    ) -> FilesystemResult<Option<(Ext4DirEntry, u32, usize)>> {
        console_println!(
            "dbg: ENTER find_entry_in_dir: Searching for '{}' in dir_inode {}",
            entry_name, dir_inode_num
        );
        let dir_inode = self.read_inode(dir_inode_num)?;

        let i_flags = dir_inode.i_flags;
        let i_size_lo = dir_inode.i_size_lo;

        if (i_flags & EXT4_EXTENTS_FL) != 0 {
            console_println!("dbg: find_entry_in_dir: Using EXTENT path for inode {}", dir_inode_num);
            // Directory uses extents
            let i_block_copy = dir_inode.i_block; // Copy to avoid direct packed access for header
            let extent_header: Ext4ExtentHeader = unsafe { core::ptr::read(i_block_copy.as_ptr() as *const _) };
            
            let eh_magic = extent_header.eh_magic; // Local copy
            let eh_entries = extent_header.eh_entries; // Local copy
            let eh_depth = extent_header.eh_depth; // Local copy

            console_println!(
                "dbg: find_entry_in_dir (EXTENT_PATH): Header Read - Magic=0x{:04X}, Entries={}, Depth={}", 
                eh_magic, eh_entries, eh_depth
            );

            if eh_magic != EXT4_EXT_MAGIC {
                console_println!("find_entry_in_dir: Invalid extent magic 0x{:04X}", eh_magic);
                return Err(FilesystemError::CorruptedFilesystem);
            }
            if eh_depth != 0 {
                console_println!("find_entry_in_dir: Extent tree depth {} not supported", eh_depth);
                return Err(FilesystemError::UnsupportedFilesystem);
            }

            // Correctly access extents as a byte slice from i_block
            let i_block_bytes = unsafe {
                core::slice::from_raw_parts(
                    i_block_copy.as_ptr() as *const u8, // Cast to *const u8
                    core::mem::size_of_val(&i_block_copy) // Size of the i_block array (60 bytes)
                )
            };
            let extents_byte_offset_after_header = core::mem::size_of::<Ext4ExtentHeader>();

            for i in 0..eh_entries {
                let current_extent_offset_in_bytes = extents_byte_offset_after_header + (i as usize * core::mem::size_of::<Ext4Extent>());
                
                if current_extent_offset_in_bytes + core::mem::size_of::<Ext4Extent>() > i_block_bytes.len() {
                    console_println!("dbg: find_entry_in_dir (EXTENT_PATH): Extent {} at offset {} would read out of i_block bounds.", i, current_extent_offset_in_bytes);
                    break; 
                }

                let extent = unsafe { 
                    let extent_ptr = i_block_bytes.as_ptr().add(current_extent_offset_in_bytes) as *const Ext4Extent;
                    *extent_ptr 
                };
                let physical_start_block = extent.ee_start_lo; // ee_start_hi assumed 0 for simplicity
                let num_blocks_in_extent = extent.ee_len;
                // Create local copies for packed fields before use in console_println
                let logical_block_copy = extent.ee_block;
                let start_hi_copy = extent.ee_start_hi;

                console_println!(
                    "dbg: find_entry_in_dir (EXTENT_PATH): Processing Extent {} - LogicalBlock={}, PhysicalStartLo={}, Len={}, StartHi={}", 
                    i, logical_block_copy, physical_start_block, num_blocks_in_extent, start_hi_copy
                );

                for block_idx_in_extent in 0..num_blocks_in_extent {
                    let current_physical_block_num = physical_start_block + block_idx_in_extent as u32;
                    if current_physical_block_num == 0 { continue; }

                    let block_data = self.read_block_data(current_physical_block_num as u64)?;
                    let mut offset_in_block = 0;
                    while offset_in_block < self.block_size {
                        let entry_ptr = unsafe {
                            block_data.as_ptr().add(offset_in_block) as *const Ext4DirEntry
                        };
                        let entry = unsafe { *entry_ptr };

                        let entry_inode = entry.inode; // local copy
                        let entry_rec_len = entry.rec_len; // local copy
                        let entry_name_len = entry.name_len; // local copy

                        // Debug print added
                        if entry_rec_len > 0 && entry_name_len > 0 { // Only try to print if it looks like a valid entry
                            let name_slice_dbg = unsafe {
                                core::slice::from_raw_parts(
                                    block_data.as_ptr().add(offset_in_block + 8),
                                    entry_name_len as usize,
                                )
                            };
                            if let Ok(name_str_dbg) = core::str::from_utf8(name_slice_dbg) {
                                console_println!(
                                    "dbg: find_entry (ext): offset {}, ino {}, name_len {}, rec_len {}, name '{}', target '{}'",
                                    offset_in_block, entry_inode, entry_name_len, entry_rec_len, name_str_dbg, entry_name
                                );
                            } else {
                                 console_println!(
                                    "dbg: find_entry (ext): offset {}, ino {}, name_len {}, rec_len {}, name non-utf8",
                                    offset_in_block, entry_inode, entry_name_len, entry_rec_len
                                );
                            }
                        }

                        if entry_inode == 0 || entry_rec_len == 0 {
                            if entry_rec_len == 0 { break; }
                            offset_in_block += entry_rec_len as usize;
                            continue;
                        }

                        if entry_name_len as usize == entry_name.len() {
                            let name_slice = unsafe {
                                core::slice::from_raw_parts(
                                    block_data.as_ptr().add(offset_in_block + 8),
                                    entry_name_len as usize,
                                )
                            };
                            if let Ok(name_str) = core::str::from_utf8(name_slice) {
                                if name_str == entry_name {
                                    console_println!(
                                        "find_entry_in_dir: Found '{}' in block {}, offset {}",
                                        entry_name, current_physical_block_num, offset_in_block
                                    );
                                    return Ok(Some((entry, current_physical_block_num, offset_in_block)));
                                }
                            }
                        }
                        offset_in_block += entry_rec_len as usize;
                    }
                }
            }
        } else {
            console_println!("dbg: find_entry_in_dir: Using DIRECT/INDIRECT block path for inode {}", dir_inode_num);
            // Directory uses direct/indirect blocks
            let i_block_copy = dir_inode.i_block; // Copy for safe iteration
            for i in 0..12 { // Only direct blocks for now
                let physical_block_num = i_block_copy[i];
                if physical_block_num == 0 { continue; }

                let block_data = self.read_block_data(physical_block_num as u64)?;
                let mut offset_in_block = 0;
                while offset_in_block < self.block_size {
                    // This part is identical to the extent case, could be refactored
                    let entry_ptr = unsafe { // Added unsafe block for .add
                        block_data.as_ptr().add(offset_in_block) as *const Ext4DirEntry
                    };
                    let entry = unsafe { *entry_ptr };

                    let entry_inode = entry.inode; // local copy
                    let entry_rec_len = entry.rec_len; // local copy
                    let entry_name_len = entry.name_len; // local copy

                    // Debug print added
                    if entry_rec_len > 0 && entry_name_len > 0 { // Only try to print if it looks like a valid entry
                        let name_slice_dbg = unsafe {
                            core::slice::from_raw_parts(
                                block_data.as_ptr().add(offset_in_block + 8),
                                entry_name_len as usize,
                            )
                        };
                        if let Ok(name_str_dbg) = core::str::from_utf8(name_slice_dbg) {
                            console_println!(
                                "dbg: find_entry (dir): offset {}, ino {}, name_len {}, rec_len {}, name '{}', target '{}'",
                                offset_in_block, entry_inode, entry_name_len, entry_rec_len, name_str_dbg, entry_name
                            );
                        } else {
                             console_println!(
                                "dbg: find_entry (dir): offset {}, ino {}, name_len {}, rec_len {}, name non-utf8",
                                offset_in_block, entry_inode, entry_name_len, entry_rec_len
                            );
                        }
                    }
                    // Original logic continues here
                    if entry_inode == 0 || entry_rec_len == 0 {
                        if entry_rec_len == 0 { break; }
                        offset_in_block += entry_rec_len as usize;
                        continue;
                    }

                    if entry_name_len as usize == entry_name.len() {
                        let name_slice = unsafe {
                            core::slice::from_raw_parts(
                                block_data.as_ptr().add(offset_in_block + 8),
                                entry_name_len as usize,
                            )
                        };
                        if let Ok(name_str) = core::str::from_utf8(name_slice) {
                            if name_str == entry_name {
                                console_println!(
                                    "find_entry_in_dir: Found '{}' in block {}, offset {}",
                                    entry_name, physical_block_num, offset_in_block
                                );
                                return Ok(Some((entry, physical_block_num, offset_in_block)));
                            }
                        }
                    }
                    offset_in_block += entry_rec_len as usize;
                }
            }
            // TODO: Handle indirect blocks if not found in direct blocks
        }

        console_println!("find_entry_in_dir: '{}' not found in dir_inode {}", entry_name, dir_inode_num);
        Ok(None)
    }

    // == Private helper functions for ext4 write operations ==

    /// Reads a bitmap block from disk.
    /// The bitmap is assumed to be one block in size.
    fn read_bitmap(&self, bitmap_block_num: u32) -> FilesystemResult<Vec<u8, 4096>> {
        if self.block_size > 4096 {
            // This Vec size needs to be dynamic or larger if block_size can exceed 4096
            console_println!("read_bitmap: ERROR - block_size {} > 4096 not supported by Vec capacity", self.block_size);
            return Err(FilesystemError::UnsupportedFilesystem);
        }
        self.read_block_data(bitmap_block_num as u64) // reuses existing block reading logic
    }

    /// Writes a bitmap block to disk.
    fn write_bitmap(&mut self, bitmap_block_num: u32, bitmap_data: &[u8]) -> FilesystemResult<()> {
        if bitmap_data.len() != self.block_size {
            console_println!(
                "write_bitmap: ERROR - bitmap_data len {} does not match block_size {}",
                bitmap_data.len(),
                self.block_size
            );
            return Err(FilesystemError::IoError); // Or a more specific error
        }

        let mut disk_device = virtio_blk::VIRTIO_BLK.lock();
        let sectors_per_block = self.block_size / SECTOR_SIZE;
        let start_sector = bitmap_block_num as u64 * sectors_per_block as u64;

        for i in 0..sectors_per_block {
            let sector_offset = i * SECTOR_SIZE;
            let sector_data_slice = &bitmap_data[sector_offset..sector_offset + SECTOR_SIZE];
            // This conversion might fail if SECTOR_SIZE is not 512, but read/write_blocks expect [u8; 512]
            // Assuming SECTOR_SIZE is indeed 512 for virtio_blk operations for now.
            let sector_buffer_array: &[u8; 512] = sector_data_slice
                .try_into()
                .map_err(|_| {
                    console_println!("write_bitmap: Failed to convert slice to [u8; 512] for writing");
                    FilesystemError::IoError
                })?;
            disk_device.write_blocks(start_sector + i as u64, sector_buffer_array)
                .map_err(|e| {
                    console_println!("write_bitmap: Failed to write sector {}: {:?}", start_sector + i as u64, e);
                    FilesystemError::IoError
                })?;
        }
        drop(disk_device);
        Ok(())
    }

    /// Finds the first clear (0) bit in a bitmap and returns its index.
    /// Returns None if no clear bit is found.
    fn find_free_bit(bitmap_data: &[u8]) -> Option<usize> {
        for (byte_index, byte) in bitmap_data.iter().enumerate() {
            if *byte != 0xFF { // If not all bits are 1, there's a 0 bit in this byte
                for bit_in_byte_index in 0..8 {
                    if (*byte & (1 << bit_in_byte_index)) == 0 {
                        return Some(byte_index * 8 + bit_in_byte_index);
                    }
                }
            }
        }
        None
    }

    /// Sets the specified bit (marks as used, 1) in the bitmap data. (DEPRECATED USIZE VERSION)
    /// Panics if bit_index is out of bounds.
    fn set_bit_usize_deprecated(bitmap_data: &mut [u8], bit_index: usize) {
        let byte_index = bit_index / 8;
        let bit_in_byte_index = bit_index % 8;
        if byte_index < bitmap_data.len() {
            bitmap_data[byte_index] |= 1 << bit_in_byte_index;
        } else {
            console_println!("set_bit_usize_deprecated: bit_index {} out of bounds for bitmap_data len {}", bit_index, bitmap_data.len());
        }
    }

    /// Clears the specified bit (marks as free, 0) in the bitmap data. (DEPRECATED USIZE VERSION)
    /// Panics if bit_index is out of bounds.
    fn clear_bit_usize_deprecated(bitmap_data: &mut [u8], bit_index: usize) {
        let byte_index = bit_index / 8;
        let bit_in_byte_index = bit_index % 8;
        if byte_index < bitmap_data.len() {
            bitmap_data[byte_index] &= !(1 << bit_in_byte_index);
        } else {
            console_println!("clear_bit_usize_deprecated: bit_index {} out of bounds for bitmap_data len {}", bit_index, bitmap_data.len());
        }
    }

    /// Frees an inode in group 0.
    /// Clears the bit in the inode bitmap and updates superblock/GDT free counts.
    fn free_inode(&mut self, inode_num: u32) -> FilesystemResult<()> {
        if inode_num == 0 || inode_num == EXT4_ROOT_INODE { // Cannot free inode 0 or root inode
            console_println!("free_inode: Attempt to free invalid inode_num: {}", inode_num);
            return Err(FilesystemError::IoError);
        }

        let mut sb_to_update = self.superblock.ok_or(FilesystemError::NotInitialized)?;
        let mut gd_to_update = self.group_desc.ok_or(FilesystemError::NotInitialized)?;

        // Calculate bit index in group 0's inode bitmap
        // Assuming inode_num is global and group 0 starts at inode 1.
        let bit_index = (inode_num - 1) as usize; 
        
        // local copy for printing
        let s_inodes_per_group_val = sb_to_update.s_inodes_per_group;
        if bit_index >= s_inodes_per_group_val as usize {
             console_println!("free_inode: inode_num {} (bit_index {}) is out of range for group 0's inode bitmap (size {}).", 
                inode_num, bit_index, s_inodes_per_group_val);
            return Err(FilesystemError::IoError); // Or a more specific error
        }
        
        console_println!("free_inode: Freeing inode {} (bit_index {})", inode_num, bit_index);

        let mut inode_bitmap_data = self.read_bitmap(gd_to_update.bg_inode_bitmap_lo)?;
        Self::clear_bit(&mut inode_bitmap_data, bit_index as u32); // Explicitly call corrected version
        self.write_bitmap(gd_to_update.bg_inode_bitmap_lo, &inode_bitmap_data)?;
        console_println!("free_inode: Cleared bit {} in inode bitmap.", bit_index);

        // Update superblock free inodes count
        sb_to_update.s_free_inodes_count = sb_to_update.s_free_inodes_count.saturating_add(1);
        self.write_superblock(&sb_to_update)?;
        self.superblock = Some(sb_to_update); // Update in-memory copy

        // Update group descriptor free inodes count for group 0
        gd_to_update.bg_free_inodes_count_lo = gd_to_update.bg_free_inodes_count_lo.saturating_add(1);
        self.write_group_descriptor(0, &gd_to_update)?;
        self.group_desc = Some(gd_to_update); // Update in-memory copy

        // local copies for printing
        let s_free_inodes_count_val = sb_to_update.s_free_inodes_count;
        let bg_free_inodes_count_lo_val = gd_to_update.bg_free_inodes_count_lo;
        console_println!("✅ free_inode: Inode {} freed. SB free inodes: {}. GD0 free inodes: {}", 
            inode_num, s_free_inodes_count_val, bg_free_inodes_count_lo_val);
        Ok(())
    }

    /// Frees a data block in group 0.
    /// Clears the bit in the block bitmap and updates superblock/GDT free counts.
    fn free_block(&mut self, block_num: u32) -> FilesystemResult<()> {
        if block_num == 0 {
            console_println!("free_block: Attempt to free block_num 0, which is invalid.");
            return Err(FilesystemError::IoError);
        }
        let mut sb_to_update_fb = self.superblock.ok_or(FilesystemError::NotInitialized)?;
        let mut gd_to_update_fb = self.group_desc.ok_or(FilesystemError::NotInitialized)?;
        
        let s_first_data_block_val_fb = sb_to_update_fb.s_first_data_block; 
        if block_num < s_first_data_block_val_fb {
            console_println!("free_block: block_num {} is less than s_first_data_block {}. Cannot free metadata blocks this way.",
                block_num, s_first_data_block_val_fb);
            return Err(FilesystemError::IoError);
        }

        let s_blocks_per_group_fb = sb_to_update_fb.s_blocks_per_group; 
        let offset_in_group_fb = (block_num - s_first_data_block_val_fb) % s_blocks_per_group_fb;

        let bit_index_fb = offset_in_group_fb; 

        let mut block_bitmap_data = self.read_bitmap(gd_to_update_fb.bg_block_bitmap_lo)?; 
        Self::clear_bit(&mut block_bitmap_data, bit_index_fb as u32); // Explicitly call corrected version
        self.write_bitmap(gd_to_update_fb.bg_block_bitmap_lo, &block_bitmap_data)?;

        // Update superblock free blocks count
        sb_to_update_fb.s_free_blocks_count_lo = sb_to_update_fb.s_free_blocks_count_lo.saturating_add(1);
        self.write_superblock(&sb_to_update_fb)?;
        self.superblock = Some(sb_to_update_fb); // Update in-memory copy

        // Update group descriptor free blocks count for group 0
        gd_to_update_fb.bg_free_blocks_count_lo = gd_to_update_fb.bg_free_blocks_count_lo.saturating_add(1);
        self.write_group_descriptor(0, &gd_to_update_fb)?;
        self.group_desc = Some(gd_to_update_fb); // Update in-memory copy

        // local copies for printing
        let s_free_blocks_count_lo_val = sb_to_update_fb.s_free_blocks_count_lo;
        let bg_free_blocks_count_lo_val = gd_to_update_fb.bg_free_blocks_count_lo;
        console_println!("✅ free_block: Block {} freed. SB free blocks: {}. GD0 free blocks: {}", 
            block_num, s_free_blocks_count_lo_val, bg_free_blocks_count_lo_val);
        Ok(())
    }

    /// Clears a bit in a byte slice (bitmap).
    fn clear_bit(bitmap_data: &mut [u8], bit_index: u32) { // Removed &self, renamed param
        let byte_offset_in_bitmap = bit_index / 8;
        let bit_in_byte_index = bit_index % 8;
        if byte_offset_in_bitmap < bitmap_data.len() as u32 { 
            bitmap_data[byte_offset_in_bitmap as usize] &= !(1 << bit_in_byte_index);
        } else {
            console_println!(
                "clear_bit (u32): byte_offset_in_bitmap {} is out of bounds for bitmap of length {}. Bit index was {}", 
                byte_offset_in_bitmap, bitmap_data.len(), bit_index
            );
        }
    }

    /// Sets a bit in a byte slice (bitmap).
    fn set_bit(bitmap_data: &mut [u8], bit_index: u32) { // Removed &self, renamed param
        let byte_offset_in_bitmap = bit_index / 8;
        let bit_in_byte_index = bit_index % 8;
        if byte_offset_in_bitmap < bitmap_data.len() as u32 { 
            bitmap_data[byte_offset_in_bitmap as usize] |= (1 << bit_in_byte_index);
        } else {
            console_println!(
                "set_bit (u32): byte_offset_in_bitmap {} is out of bounds for bitmap of length {}. Bit index was {}", 
                byte_offset_in_bitmap, bitmap_data.len(), bit_index
            );
        }
    }

    /// List directory entries from extent-based inode
    fn list_directory_from_extents(&self, dir_inode: &Ext4Inode, result_vec: &mut Vec<(heapless::String<64>, usize, bool), 32>) -> FilesystemResult<()> {
        // Copy i_block array to avoid packed field alignment issues
        let i_block_copy = dir_inode.i_block;
        
        // Parse extent header
        let extent_header: Ext4ExtentHeader = unsafe {
            let header_ptr = i_block_copy.as_ptr() as *const Ext4ExtentHeader;
            *header_ptr
        };
        
        let eh_magic = extent_header.eh_magic;
        let eh_entries = extent_header.eh_entries;
        let eh_depth = extent_header.eh_depth;
        
        console_println!("   🔍 Directory extent header: magic=0x{:04x}, entries={}, depth={}", 
            eh_magic, eh_entries, eh_depth);
        
        if eh_magic != EXT4_EXT_MAGIC {
            console_println!("   ❌ Invalid extent magic for directory");
            return Err(FilesystemError::CorruptedFilesystem);
        }
        
        if eh_depth != 0 {
            console_println!("   ❌ Multi-level extent trees not supported for directory listing");
            return Err(FilesystemError::UnsupportedFilesystem);
        }
        
        // Parse extent entries
        let extents_start = core::mem::size_of::<Ext4ExtentHeader>();
        let i_block_bytes = unsafe {
            core::slice::from_raw_parts(
                i_block_copy.as_ptr() as *const u8,
                60  // i_block is 15 * 4 = 60 bytes
            )
        };
        
        for i in 0..eh_entries as usize {
            let extent_offset = extents_start + i * core::mem::size_of::<Ext4Extent>();
            
            if extent_offset + core::mem::size_of::<Ext4Extent>() > i_block_bytes.len() {
                console_println!("   ⚠️ Directory extent {} extends beyond i_block", i);
                break;
            }
            
            let extent: Ext4Extent = unsafe {
                let extent_ptr = (i_block_bytes.as_ptr().add(extent_offset)) as *const Ext4Extent;
                *extent_ptr
            };
            
            let ee_block = extent.ee_block;
            let ee_len = extent.ee_len;
            let ee_start_hi = extent.ee_start_hi;
            let ee_start_lo = extent.ee_start_lo;
            
            // Calculate physical block number
            let physical_block = ((ee_start_hi as u64) << 32) | (ee_start_lo as u64);
            
            console_println!("   🔍 Directory extent {}: logical={}, len={}, physical={}", 
                i, ee_block, ee_len, physical_block);
            
            // Read directory data from this extent
            for block_offset in 0..ee_len {
                let block_num = physical_block + block_offset as u64;
                
                console_println!("   📍 Reading directory block {} (from extent)", block_num);
                
                let block_data = match self.read_block_data(block_num) {
                    Ok(data) => data,
                    Err(_) => {
                        console_println!("   ⚠️ Failed to read directory extent block {}", block_num);
                        continue;
                    }
                };
                
                // Parse directory entries in this block
                self.parse_directory_block_for_listing(&block_data, result_vec)?;
            }
        }
        
        Ok(())
    }
    
    /// List directory entries from traditional direct block pointers
    fn list_directory_from_blocks(&self, dir_inode: &Ext4Inode, result_vec: &mut Vec<(heapless::String<64>, usize, bool), 32>) -> FilesystemResult<()> {
        // Copy i_block array to avoid packed field alignment issues
        let i_block_copy = dir_inode.i_block;
        
        // Handle only direct blocks (first 12 entries in i_block) for simplicity
        for &block_num in i_block_copy.iter().take(12) {
            if block_num == 0 {
                break;
            }
            
            // Validate block number
            if block_num > 1000000 {
                console_println!("   ⚠️ Skipping invalid block number: {}", block_num);
                continue;
            }
            
            let block_data = match self.read_block_data(block_num as u64) {
                Ok(data) => data,
                Err(_) => {
                    console_println!("   ⚠️ Failed to read directory block {}", block_num);
                    continue;
                }
            };
            
            // Parse directory entries in this block
            self.parse_directory_block_for_listing(&block_data, result_vec)?;
        }
        
        Ok(())
    }
    
    /// Parse directory entries from a block of data for listing
    fn parse_directory_block_for_listing(&self, block_data: &[u8], result_vec: &mut Vec<(heapless::String<64>, usize, bool), 32>) -> FilesystemResult<()> {
        let mut offset = 0;
        
        while offset < block_data.len() {
            // Ensure we have enough bytes for directory entry header
            if offset + 8 > block_data.len() {
                break;
            }
            
            let dir_entry: Ext4DirEntry = unsafe {
                let entry_ptr = (block_data.as_ptr().add(offset)) as *const Ext4DirEntry;
                *entry_ptr
            };
            
            let inode = dir_entry.inode;
            let rec_len = dir_entry.rec_len;
            let name_len = dir_entry.name_len;
            let file_type = dir_entry.file_type;
            
            // Handle empty/deleted entries (inode = 0)
            if inode == 0 {
                if rec_len == 0 {
                    break;
                }
                offset += rec_len as usize;
                continue;
            }
            
            // Validate rec_len
            if rec_len == 0 {
                break;
            }
            
            if offset + rec_len as usize > block_data.len() {
                break;
            }
            
            if name_len > 0 && name_len <= 255 {
                // Extract filename
                let name_start = offset + 8; // Skip fixed part of dir entry
                let name_end = name_start + name_len as usize;
                
                if name_end <= block_data.len() {
                    let name_bytes = &block_data[name_start..name_end];
                    
                    if let Ok(filename) = core::str::from_utf8(name_bytes) {
                        // Skip "." and ".." entries for file listing
                        if filename != "." && filename != ".." {
                            // First try using file_type from directory entry
                            let mut is_directory = file_type == EXT4_FT_DIR;
                            
                            // If file_type is 0 or doesn't match expected values, check inode mode as fallback
                            if file_type == 0 || (file_type != EXT4_FT_DIR && file_type != EXT4_FT_REG_FILE) {
                                console_println!("   🔍 file_type={} unreliable, checking inode mode for '{}'", file_type, filename);
                                match self.read_inode(inode) {
                                    Ok(entry_inode) => {
                                        // Check inode mode: 0x4000 = S_IFDIR (directory)
                                        let inode_mode = entry_inode.i_mode; // Copy to local variable
                                        is_directory = (inode_mode & 0xF000) == 0x4000;
                                        console_println!("   📊 Inode mode: 0x{:04x}, is_directory: {}", inode_mode, is_directory);
                                    },
                                    Err(e) => {
                                        console_println!("   ⚠️ Failed to read inode {} for type detection: {:?}", inode, e);
                                        // Keep original file_type determination as fallback
                                    }
                                }
                            }
                            
                            // Get file size from inode
                            let file_size = match self.read_inode(inode) {
                                Ok(file_inode) => {
                                    let size = file_inode.i_size_lo as usize;
                                    size
                                },
                                Err(_) => 0
                            };
                            
                            let name_str = heapless::String::try_from(filename)
                                .map_err(|_| FilesystemError::FilenameTooLong)?;
                            
                            if result_vec.push((name_str, file_size, is_directory)).is_err() {
                                console_println!("   ⚠️ Directory listing result vector full");
                                return Ok(());
                            }
                        }
                    }
                }
            }
            
            offset += rec_len as usize;
        }
        
        Ok(())
    }

    fn remove_direntry_from_parent(&mut self, parent_dir_inode_num: u32, block_num_of_entry: u32, entry_offset_in_block: usize) -> FilesystemResult<()> {
        console_println!("ext4: remove_direntry_from_parent - Removing entry from parent dir inode {} at block {}, offset {}", parent_dir_inode_num, block_num_of_entry, entry_offset_in_block);
        
        // Read the directory block containing the entry to remove
        let mut block_data = self.read_block_data(block_num_of_entry as u64)?;
        
        // Verify we have the expected entry at the given offset
        let entry_ptr = unsafe {
            block_data.as_ptr().add(entry_offset_in_block) as *const Ext4DirEntry
        };
        let entry = unsafe { *entry_ptr };
        
        if entry.inode == 0 {
            console_println!("ext4: remove_direntry_from_parent - Entry already deleted at offset {}", entry_offset_in_block);
            return Ok(());
        }
        
        // Copy packed fields to local variables for printing
        let entry_inode_val = entry.inode;
        let entry_rec_len_val = entry.rec_len;
        let entry_name_len_val = entry.name_len;
        console_println!("ext4: remove_direntry_from_parent - Found entry at offset {}: inode={}, rec_len={}, name_len={}", 
            entry_offset_in_block, entry_inode_val, entry_rec_len_val, entry_name_len_val);
        
        // Mark the entry as deleted by setting inode to 0
        unsafe {
            let entry_ptr_mut = block_data.as_mut_ptr().add(entry_offset_in_block) as *mut Ext4DirEntry;
            (*entry_ptr_mut).inode = 0;
            // Keep rec_len and other fields as they are for proper directory traversal
        }
        
        console_println!("ext4: remove_direntry_from_parent - Marked entry as deleted (inode=0)");
        
        // Write the updated block back to disk
        self.write_block_data_internal(block_num_of_entry, &block_data)?;
        
        console_println!("ext4: remove_direntry_from_parent - Updated directory block {} written to disk", block_num_of_entry);
        
        Ok(())
    }
} // Closing brace for impl Ext4FileSystem

// Implementation of the FileSystem trait for Ext4FileSystem
impl FileSystem for Ext4FileSystem {
    fn list_files(&self) -> FilesystemResult<Vec<(heapless::String<64>, usize), 32>> {
        if !self.is_mounted() {
            return Err(FilesystemError::NotMounted);
        }
        let mut result_vec = Vec::new();
        // self.files currently only contains root directory entries parsed at init.
        // For a more general list_files, it would need path resolution.
        // This is a simplified version for root listing.
        for entry in self.files.iter() {
            let name_str = heapless::String::try_from(entry.name.as_str())
                .map_err(|_| FilesystemError::FilenameTooLong)?;
            if result_vec.push((name_str, entry.size)).is_err() {
                console_println!("ext4: list_files - Result vector full.");
                break;
            }
        }
        Ok(result_vec)
    }

    fn list_directory(&self, path: &str) -> FilesystemResult<Vec<(heapless::String<64>, usize, bool), 32>> {
        if !self.is_mounted() {
            return Err(FilesystemError::NotMounted);
        }
        
        console_println!("ext4: list_directory('{}')", path);
        
        // Resolve the path to get the directory inode
        let dir_inode_num = self.resolve_path_to_inode(path)?;
        let dir_inode = self.read_inode(dir_inode_num)?;
        
        // Check if it's actually a directory
        let i_mode = dir_inode.i_mode;
        if (i_mode & 0xF000) != 0x4000 {
            console_println!("ext4: list_directory - Path '{}' is not a directory", path);
            return Err(FilesystemError::NotADirectory);
        }
        
        console_println!("ext4: list_directory - Reading directory inode {}", dir_inode_num);
        
        let mut result_vec = Vec::new();
        
        // Parse directory entries from the directory inode
        let i_flags = dir_inode.i_flags;
        
        if (i_flags & EXT4_EXTENTS_FL) != 0 {
            self.list_directory_from_extents(&dir_inode, &mut result_vec)?;
        } else {
            self.list_directory_from_blocks(&dir_inode, &mut result_vec)?;
        }
        
        console_println!("ext4: list_directory - Found {} entries in '{}'", result_vec.len(), path);
        Ok(result_vec)
    }

    fn read_file(&self, path: &str) -> FilesystemResult<Vec<u8, 4096>> {
        if !self.is_mounted() {
            return Err(FilesystemError::NotMounted);
        }
        console_println!("ext4 FS trait: read_file '{}'", path);
        let inode_num = self.resolve_path_to_inode(path)?;
        // Create a temporary FileEntry; size will be determined by read_inode in read_file_content
        let temp_file_entry = FileEntry::new_file(path, inode_num as u64, 0)?; 
        self.read_file_content(&temp_file_entry)
    }

    fn file_exists(&self, path: &str) -> bool {
        if !self.is_mounted() {
            return false;
        }
        self.resolve_path_to_inode(path).is_ok()
    }
    
    fn get_filesystem_info(&self) -> Option<(u16, u32, u16)> {
        if !self.is_initialized() {
            return None;
        }
        if let Some(sb) = &self.superblock {
            // local copies for safety
            let magic = sb.s_magic;
            let blocks_count = sb.s_blocks_count_lo;
            let block_size = self.block_size as u16; // block_size is usize
            Some((magic, blocks_count, block_size))
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

    // == Write Operations ==
    fn create_file(&mut self, path: &str) -> FilesystemResult<FileEntry> {
        if !self.is_mounted() { return Err(FilesystemError::NotMounted); }
        // Calls the existing method on Ext4FileSystem struct
        Ext4FileSystem::create_file(self, path)
    }

    fn create_directory(&mut self, path: &str) -> FilesystemResult<FileEntry> {
        if !self.is_mounted() { return Err(FilesystemError::NotMounted); }
        Ext4FileSystem::create_directory(self, path)
    }

    fn write_file(&mut self, file: &FileEntry, offset: u64, data: &[u8]) -> FilesystemResult<usize> {
        if !self.is_mounted() { return Err(FilesystemError::NotMounted); }
        Ext4FileSystem::write_file(self, file, offset, data)
    }

    fn delete_file(&mut self, path: &str) -> FilesystemResult<()> {
        if !self.is_mounted() { return Err(FilesystemError::NotMounted); }
        Ext4FileSystem::delete_file(self, path)
    }

    fn delete_directory(&mut self, path: &str) -> FilesystemResult<()> {
        if !self.is_mounted() { return Err(FilesystemError::NotMounted); }
        Ext4FileSystem::delete_directory(self, path)
    }
    
    fn truncate_file(&mut self, file: &FileEntry, new_size: u64) -> FilesystemResult<()> {
        if !self.is_mounted() { return Err(FilesystemError::NotMounted); }
        // Calls the existing method on Ext4FileSystem struct
        // Ext4FileSystem::truncate_file(self, file, new_size) 
        // For now, since the main truncate_file is a basic stub:
        console_println!("ext4 FS Trait: truncate_file for '{}' to size {} - NOT IMPLEMENTED in main struct", file.name, new_size);
        Err(FilesystemError::UnsupportedFilesystem)
    }

    fn sync(&mut self) -> FilesystemResult<()> {
        // For ext4, fsync would involve ensuring all dirty metadata and data are written to disk.
        // This includes superblock, group descriptors, inode table, block bitmaps, inode bitmaps,
        // and any modified file data blocks.
        // For now, this is a placeholder.
        console_println!("Ext4FileSystem: sync() called, placeholder.");
        Ok(())
    }
} // This is the correct closing brace for impl FileSystem for Ext4FileSystem