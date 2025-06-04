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
#[derive(Debug, Clone, Copy)]
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
        let mut sb_buffer = [0u8; 1024];
        
        // Read 2 sectors to get full superblock
        for i in 0..2 {
            let mut sector_buf = [0u8; SECTOR_SIZE];
            disk_device.read_blocks((start_sector + i) as u64, &mut sector_buf)
                .map_err(|_| FilesystemError::IoError)?;
            sb_buffer[i * SECTOR_SIZE..(i + 1) * SECTOR_SIZE].copy_from_slice(&sector_buf);
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
        
        let eh_magic = extent_header.eh_magic;
        let eh_entries = extent_header.eh_entries;
        let eh_depth = extent_header.eh_depth;
        
        console_println!("   🔍 Extent header: magic=0x{:04x}, entries={}, depth={}", 
            eh_magic, eh_entries, eh_depth);
        
        if eh_magic != EXT4_EXT_MAGIC {
            console_println!("   ❌ Invalid extent magic: 0x{:04x} (expected: 0x{:04x})", 
                eh_magic, EXT4_EXT_MAGIC);
            return Err(FilesystemError::CorruptedFilesystem);
        }
        
        if eh_depth != 0 {
            console_println!("   ❌ Multi-level extent trees not supported (depth={})", eh_depth);
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
        
        console_println!("   📊 Processing {} extent entries", eh_entries);
        
        for i in 0..eh_entries as usize {
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
                            let is_directory = file_type == EXT4_FT_DIR;
                            
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
                            
                            if self.files.push(file_entry).is_err() {
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
}

impl FileSystem for Ext4FileSystem {
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
        console_println!("📖 Reading real file: {}", filename);
        
        // Find the file in our parsed directory entries
        for file in &self.files {
            if file.name.as_str() == filename && !file.is_directory {
                return self.read_file_content(file);
            }
        }
        
        console_println!("❌ File '{}' not found in directory", filename);
        Err(FilesystemError::FileNotFound)
    }
    
    fn file_exists(&self, filename: &str) -> bool {
        self.files.iter().any(|f| f.name.as_str() == filename)
    }
    
    fn get_filesystem_info(&self) -> Option<(u16, u32, u16)> {
        if let Some(superblock) = &self.superblock {
            let s_blocks_count_lo = superblock.s_blocks_count_lo;
            Some((EXT4_MAGIC, s_blocks_count_lo, self.block_size as u16))
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