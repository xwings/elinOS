// ext2 data structures

/// ext2 constants
pub const SECTOR_SIZE: usize = 512;
pub const EXT2_SUPERBLOCK_OFFSET: usize = 1024;
pub const EXT2_MAGIC: u16 = 0xEF53;
pub const EXT2_ROOT_INODE: u32 = 2;
pub const EXT2_FT_REG_FILE: u8 = 1;
pub const EXT2_FT_DIR: u8 = 2;
pub const EXT2_EXTENTS_FL: u32 = 0x00080000;
pub const EXT2_EXT_MAGIC: u16 = 0xF30A;

/// Simplified ext2 Superblock - only essential fields
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct Ext2Superblock {
    pub s_inodes_count: u32,        // 0x00
    pub s_blocks_count_lo: u32,     // 0x04
    pub s_r_blocks_count_lo: u32,   // 0x08
    pub s_free_blocks_count_lo: u32, // 0x0C
    pub s_free_inodes_count: u32,   // 0x10
    pub s_first_data_block: u32,    // 0x14
    pub s_log_block_size: u32,      // 0x18
    pub s_log_cluster_size: u32,    // 0x1C
    pub s_blocks_per_group: u32,    // 0x20
    pub s_clusters_per_group: u32,  // 0x24
    pub s_inodes_per_group: u32,    // 0x28
    pub s_mtime: u32,              // 0x2C
    pub s_wtime: u32,              // 0x30
    pub s_mnt_count: u16,          // 0x34
    pub s_max_mnt_count: u16,      // 0x36
    pub s_magic: u16,              // 0x38 - Magic signature (0xEF53)
    pub s_state: u16,              // 0x3A
    pub s_errors: u16,             // 0x3C
    pub s_minor_rev_level: u16,    // 0x3E
    pub s_lastcheck: u32,          // 0x40
    pub s_checkinterval: u32,      // 0x44
    pub s_creator_os: u32,         // 0x48
    pub s_rev_level: u32,          // 0x4C
    pub s_def_resuid: u16,         // 0x50
    pub s_def_resgid: u16,         // 0x52
    // Extended fields
    pub s_first_ino: u32,          // 0x54
    pub s_inode_size: u16,         // 0x58
    pub s_block_group_nr: u16,     // 0x5A
    pub _reserved: [u8; 932],          // Padding to 1024 bytes
}

/// Simplified Group Descriptor
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct Ext2GroupDesc {
    pub bg_block_bitmap_lo: u32,      // 0x00
    pub bg_inode_bitmap_lo: u32,      // 0x04
    pub bg_inode_table_lo: u32,       // 0x08
    pub bg_free_blocks_count_lo: u16, // 0x0C
    pub bg_free_inodes_count_lo: u16, // 0x0E
    pub bg_used_dirs_count_lo: u16,   // 0x10
    pub bg_flags: u16,                // 0x12
    pub _reserved: [u8; 16],              // Padding to 32 bytes
}

/// Simplified Inode - focusing on basic fields
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct Ext2Inode {
    pub i_mode: u16,        // 0x00
    pub i_uid: u16,         // 0x02
    pub i_size_lo: u32,     // 0x04
    pub i_atime: u32,       // 0x08
    pub i_ctime: u32,       // 0x0C
    pub i_mtime: u32,       // 0x10
    pub i_dtime: u32,       // 0x14
    pub i_gid: u16,         // 0x18
    pub i_links_count: u16, // 0x1A
    pub i_blocks_lo: u32,   // 0x1C
    pub i_flags: u32,       // 0x20
    pub i_osd1: u32,        // 0x24
    pub i_block: [u32; 15], // 0x28 - Block pointers (60 bytes)
    pub i_generation: u32,  // 0x64
    pub i_file_acl_lo: u32, // 0x68
    pub i_size_high: u32,   // 0x6C
    pub _padding: [u8; 144],    // Padding to 256 bytes total
}

/// Directory Entry
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct Ext2DirEntry {
    pub inode: u32,       // Inode number
    pub rec_len: u16,     // Directory entry length
    pub name_len: u8,     // Name length
    pub file_type: u8,    // File type
    // name follows here
}

/// Extent Header
#[repr(C, packed)]
#[derive(Debug, Clone, Copy, Default)]
pub struct Ext2ExtentHeader {
    pub eh_magic: u16,          // Magic number (0xF30A)
    pub eh_entries: u16,        // Number of valid entries following the header
    pub eh_max: u16,            // Maximum number of entries that could follow
    pub eh_depth: u16,          // Depth of tree (0 = leaf node)
    pub eh_generation: u32,     // Generation
}

/// Extent Entry
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct Ext2Extent {
    pub ee_block: u32,          // First logical block extent covers
    pub ee_len: u16,            // Number of blocks covered by extent
    pub ee_start_hi: u16,       // High 16 bits of physical block
    pub ee_start_lo: u32,       // Low 32 bits of physical block
}

impl Ext2Inode {
    /// Create a new inode with default values
    pub fn new(mode: u16, uid: u16, gid: u16, links_count: u16, flags: u32) -> Self {
        Self {
            i_mode: mode,
            i_uid: uid,
            i_size_lo: 0,
            i_atime: 0, // TODO: Use current time
            i_ctime: 0,
            i_mtime: 0,
            i_dtime: 0,
            i_gid: gid,
            i_links_count: links_count,
            i_blocks_lo: 0,
            i_flags: flags,
            i_osd1: 0,
            i_block: [0; 15],
            i_generation: 1,
            i_file_acl_lo: 0,
            i_size_high: 0,
            _padding: [0; 144],
        }
    }
    
    /// Check if this inode is a directory
    pub fn is_directory(&self) -> bool {
        (self.i_mode & 0o170000) == 0o040000
    }
    
    /// Check if this inode is a regular file
    pub fn is_regular_file(&self) -> bool {
        (self.i_mode & 0o170000) == 0o100000
    }
    
    /// Get file size (combining low and high parts)
    pub fn get_size(&self) -> u64 {
        (self.i_size_high as u64) << 32 | (self.i_size_lo as u64)
    }
    
    /// Set file size (splitting into low and high parts)
    pub fn set_size(&mut self, size: u64) {
        self.i_size_lo = size as u32;
        self.i_size_high = (size >> 32) as u32;
    }
    
    /// Check if inode uses extents
    pub fn uses_extents(&self) -> bool {
        (self.i_flags & EXT2_EXTENTS_FL) != 0
    }
}

impl Ext2Extent {
    /// Get the full physical block number
    pub fn get_start_block(&self) -> u64 {
        ((self.ee_start_hi as u64) << 32) | (self.ee_start_lo as u64)
    }
    
    /// Set the physical block number
    pub fn set_start_block(&mut self, block: u64) {
        self.ee_start_lo = block as u32;
        self.ee_start_hi = (block >> 32) as u16;
    }
} 