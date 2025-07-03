// Modular ext2 Filesystem Implementation

use super::traits::{FileSystem, FileEntry, FilesystemError, FilesystemResult};
use heapless::Vec;

// Re-export modules
pub mod structures;
pub mod superblock;
pub mod inode;
pub mod directory;
pub mod block;
pub mod bitmap;

use structures::*;
use superblock::SuperblockManager;
use inode::InodeManager;
use directory::DirectoryManager;
use block::BlockManager;
use bitmap::BitmapManager;

/// Main ext2 Filesystem implementation
pub struct Ext2FileSystem {
    superblock_mgr: SuperblockManager,
    inode_mgr: InodeManager,
    directory_mgr: DirectoryManager,
    block_mgr: BlockManager,
    bitmap_mgr: BitmapManager,
    files: Vec<FileEntry, 64>,
    initialized: bool,
    mounted: bool,
}

impl Ext2FileSystem {
    pub fn new() -> Self {
        Self {
            superblock_mgr: SuperblockManager::new(),
            inode_mgr: InodeManager::new(),
            directory_mgr: DirectoryManager::new(),
            block_mgr: BlockManager::new(),
            bitmap_mgr: BitmapManager::new(),
            files: Vec::new(),
            initialized: false,
            mounted: false,
        }
    }
    
    /// Initialize the ext2 filesystem
    pub fn init(&mut self) -> FilesystemResult<()> {
        // Initialize all managers in sequence
        self.superblock_mgr.init()?;
        self.inode_mgr.init(&self.superblock_mgr)?;
        self.directory_mgr.init(&self.superblock_mgr, &self.inode_mgr)?;
        self.block_mgr.init(&self.superblock_mgr)?;
        self.bitmap_mgr.init(&self.superblock_mgr)?;
        
        // Parse root directory
        self.parse_root_directory()?;
        
        self.initialized = true;
        self.mounted = true;
        Ok(())
    }
    
    fn parse_root_directory(&mut self) -> FilesystemResult<()> {
        let root_inode = self.inode_mgr.read_inode(EXT2_ROOT_INODE, &self.superblock_mgr)?;
        self.directory_mgr.read_directory_entries(&root_inode, &mut self.files, &self.superblock_mgr, &self.inode_mgr)?;
        Ok(())
    }
    
    fn resolve_path_to_inode(&self, path: &str) -> FilesystemResult<u32> {
        if path == "/" {
            return Ok(EXT2_ROOT_INODE);
        }
        
        let path = path.trim_start_matches('/');
        let components: Vec<&str, 32> = path.split('/').collect();
        
        let mut current_inode = EXT2_ROOT_INODE;
        
        for component in components.iter() {
            if component.is_empty() {
                continue;
            }
            
            let inode = self.inode_mgr.read_inode(current_inode, &self.superblock_mgr)?;
            if !self.directory_mgr.is_directory(&inode) {
                return Err(FilesystemError::NotADirectory);
            }
            
            if let Some((_, child_inode, _)) = self.directory_mgr.find_entry_in_dir(current_inode, component, &self.superblock_mgr, &self.inode_mgr)? {
                current_inode = child_inode;
            } else {
                return Err(FilesystemError::FileNotFound);
            }
        }
        
        Ok(current_inode)
    }
    
    fn resolve_path_to_parent_and_filename(&self, path: &str) -> FilesystemResult<(u32, heapless::String<255>)> {
        let path = path.trim_start_matches('/').trim_end_matches('/');
        
        let last_slash = path.rfind('/');
        
        let (parent_path, filename) = if let Some(pos) = last_slash {
            (&path[..pos], &path[pos + 1..])
        } else {
            ("/", path)
        };
        
        let parent_inode = self.resolve_path_to_inode(if parent_path.is_empty() { "/" } else { parent_path })?;
        let filename = heapless::String::try_from(filename)
            .map_err(|_| FilesystemError::FilenameTooLong)?;
        
        Ok((parent_inode, filename))
    }
    
    /// Get a file entry for an existing file (public method)
    pub fn get_file_entry(&self, path: &str) -> FilesystemResult<FileEntry> {
        let inode_num = self.resolve_path_to_inode(path)?;
        let inode = self.inode_mgr.read_inode(inode_num, &self.superblock_mgr)?;
        
        if self.directory_mgr.is_directory(&inode) {
            FileEntry::new_directory(path, inode_num as u64)
        } else {
            FileEntry::new_file(path, inode_num as u64, self.inode_mgr.get_file_size(&inode))
        }
    }
    
    /// Refresh the in-memory cache by re-reading the root directory
    fn refresh_root_directory_cache(&mut self) -> FilesystemResult<()> {
        // Clear the current cache
        self.files.clear();
        
        // Re-parse the root directory to update the cache
        self.parse_root_directory()?;
        
        Ok(())
    }
}

impl FileSystem for Ext2FileSystem {
    fn list_files(&self) -> FilesystemResult<Vec<(heapless::String<64>, usize), 32>> {
        if !self.is_mounted() {
            return Err(FilesystemError::NotMounted);
        }
        
        let mut result = Vec::new();
        for file in &self.files {
            if !file.is_directory {
                let name = heapless::String::try_from(file.name.as_str())
                    .map_err(|_| FilesystemError::FilenameTooLong)?;
                result.push((name, file.size)).map_err(|_| FilesystemError::FilesystemFull)?;
            }
        }
        Ok(result)
    }
    
    fn list_directory(&self, path: &str) -> FilesystemResult<Vec<(heapless::String<64>, usize, bool), 32>> {
        if !self.is_mounted() {
            return Err(FilesystemError::NotMounted);
        }
        
        let inode_num = self.resolve_path_to_inode(path)?;
        let inode = self.inode_mgr.read_inode(inode_num, &self.superblock_mgr)?;
        
        if !self.directory_mgr.is_directory(&inode) {
            return Err(FilesystemError::NotADirectory);
        }
        
        self.directory_mgr.list_directory(&inode, &self.superblock_mgr, &self.inode_mgr)
    }
    
    fn read_file(&self, path: &str) -> FilesystemResult<Vec<u8, 32768>> {
        if !self.is_mounted() {
            return Err(FilesystemError::NotMounted);
        }
        
        // Resolve the path to get the inode number
        let inode_num = self.resolve_path_to_inode(path)?;
        let inode = self.inode_mgr.read_inode(inode_num, &self.superblock_mgr)?;
        
        // Check if it's a directory
        if self.directory_mgr.is_directory(&inode) {
            return Err(FilesystemError::IsADirectory);
        }
        
        // Get file size from inode
        let file_size = self.inode_mgr.get_file_size(&inode);
        
        // Read file content using block manager (returns Vec<u8, 8192>)
        let small_buffer = self.block_mgr.read_file_content(&inode, file_size, &self.superblock_mgr)?;
        
        // Convert to larger buffer size (Vec<u8, 32768>)
        let mut large_buffer = Vec::<u8, 32768>::new();
        for byte in small_buffer.iter() {
            if large_buffer.push(*byte).is_err() {
                break; // Buffer full
            }
        }
        
        Ok(large_buffer)
    }
    
    fn read_file_to_buffer(&self, filename: &str, buffer: &mut [u8]) -> FilesystemResult<usize> {
        let content = self.read_file(filename)?;
        let bytes_to_copy = content.len().min(buffer.len());
        buffer[..bytes_to_copy].copy_from_slice(&content[..bytes_to_copy]);
        Ok(bytes_to_copy)
    }
    
    fn get_file_size(&self, filename: &str) -> FilesystemResult<usize> {
        if !self.is_mounted() {
            return Err(FilesystemError::NotMounted);
        }
        
        // Resolve the path to get the inode number
        let inode_num = self.resolve_path_to_inode(filename)?;
        let inode = self.inode_mgr.read_inode(inode_num, &self.superblock_mgr)?;
        
        // Get file size from inode
        Ok(self.inode_mgr.get_file_size(&inode))
    }
    
    fn file_exists(&self, path: &str) -> bool {
        self.resolve_path_to_inode(path).is_ok()
    }
    
    fn get_filesystem_info(&self) -> Option<(u16, u32, u16)> {
        if let Some(sb) = self.superblock_mgr.get_superblock() {
            Some((EXT2_MAGIC, sb.s_blocks_count_lo, self.superblock_mgr.get_block_size() as u16))
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
    
    fn create_file(&mut self, path: &str) -> FilesystemResult<FileEntry> {
        if !self.is_mounted() {
            return Err(FilesystemError::NotMounted);
        }
        
        if self.file_exists(path) {
            return Err(FilesystemError::FileAlreadyExists);
        }
        
        let (parent_inode, filename) = self.resolve_path_to_parent_and_filename(path)?;
        let new_inode = self.inode_mgr.allocate_inode(0o100644, 0, 0, 1, 0, &self.superblock_mgr)?;
        
        // Note: We need to pass superblock_mgr by reference only, so create a temp scope
        {
            let sb_mgr = &mut self.superblock_mgr;
            let inode_mgr = &self.inode_mgr;
            self.directory_mgr.add_directory_entry(parent_inode, new_inode, &filename, EXT2_FT_REG_FILE, sb_mgr, inode_mgr)?;
        }
        
        // Refresh the in-memory cache to include the new file
        self.refresh_root_directory_cache()?;
        
        FileEntry::new_file(&filename, new_inode as u64, 0)
    }
    
    fn create_directory(&mut self, path: &str) -> FilesystemResult<FileEntry> {
        if !self.is_mounted() {
            return Err(FilesystemError::NotMounted);
        }
        
        if self.file_exists(path) {
            return Err(FilesystemError::FileAlreadyExists);
        }
        
        let (parent_inode, dirname) = self.resolve_path_to_parent_and_filename(path)?;
        let new_inode = self.inode_mgr.allocate_inode(0o755 | 0o040000, 0, 0, 2, 0, &self.superblock_mgr)?;
        
        // Create directory operations
        {
            let sb_mgr = &mut self.superblock_mgr;
            let inode_mgr = &self.inode_mgr;
            
            self.directory_mgr.add_directory_entry(parent_inode, new_inode, &dirname, EXT2_FT_DIR, sb_mgr, inode_mgr)?;
            self.directory_mgr.create_dot_entries(new_inode, parent_inode, sb_mgr, inode_mgr)?;
        }
        
        // Refresh the in-memory cache to include the new directory
        self.refresh_root_directory_cache()?;
        
        FileEntry::new_directory(&dirname, new_inode as u64)
    }
    
    fn write_file(&mut self, file: &FileEntry, offset: u64, data: &[u8]) -> FilesystemResult<usize> {
        if !self.is_mounted() {
            return Err(FilesystemError::NotMounted);
        }
        
        let inode_num = file.inode as u32;
        let mut inode = self.inode_mgr.read_inode(inode_num, &self.superblock_mgr)?;
        
        let bytes_written = self.block_mgr.write_file_content(&mut inode, offset, data, &mut self.superblock_mgr)?;
        self.inode_mgr.write_inode(inode_num, &inode, &self.superblock_mgr)?;
        
        Ok(bytes_written)
    }
    
    fn delete_file(&mut self, path: &str) -> FilesystemResult<()> {
        if !self.is_mounted() {
            return Err(FilesystemError::NotMounted);
        }
        
        let inode_num = self.resolve_path_to_inode(path)?;
        let inode = self.inode_mgr.read_inode(inode_num, &self.superblock_mgr)?;
        
        if self.directory_mgr.is_directory(&inode) {
            return Err(FilesystemError::IsADirectory);
        }
        
        let (parent_inode, filename) = self.resolve_path_to_parent_and_filename(path)?;
        
                // Remove directory entry
        {
            let sb_mgr = &self.superblock_mgr;
            let inode_mgr = &self.inode_mgr;
            self.directory_mgr.remove_directory_entry(parent_inode, &filename, sb_mgr, inode_mgr)?;
        }

        // Free blocks and inode
        self.block_mgr.free_inode_blocks(&inode, &mut self.superblock_mgr)?;
        self.inode_mgr.free_inode(inode_num, &self.superblock_mgr)?;
        
        // Refresh the in-memory cache to reflect the deletion
        self.refresh_root_directory_cache()?;
        
        Ok(())
    }
    
    fn delete_directory(&mut self, path: &str) -> FilesystemResult<()> {
        if !self.is_mounted() {
            return Err(FilesystemError::NotMounted);
        }
        
        let inode_num = self.resolve_path_to_inode(path)?;
        let inode = self.inode_mgr.read_inode(inode_num, &self.superblock_mgr)?;
        
        if !self.directory_mgr.is_directory(&inode) {
            return Err(FilesystemError::NotADirectory);
        }
        
        // Check if directory is empty (only contains . and ..)
        if !self.directory_mgr.is_empty_directory(&inode, &self.superblock_mgr)? {
            return Err(FilesystemError::DirectoryNotEmpty);
        }
        
        let (parent_inode, dirname) = self.resolve_path_to_parent_and_filename(path)?;
        
                // Remove directory entry
        {
            let sb_mgr = &self.superblock_mgr;
            let inode_mgr = &self.inode_mgr;
            self.directory_mgr.remove_directory_entry(parent_inode, &dirname, sb_mgr, inode_mgr)?;
        }

        // Free blocks and inode
        self.block_mgr.free_inode_blocks(&inode, &mut self.superblock_mgr)?;
        self.inode_mgr.free_inode(inode_num, &self.superblock_mgr)?;
        
        // Refresh the in-memory cache to reflect the deletion
        self.refresh_root_directory_cache()?;
        
        Ok(())
    }
    
    fn truncate_file(&mut self, file: &FileEntry, new_size: u64) -> FilesystemResult<()> {
        if !self.is_mounted() {
            return Err(FilesystemError::NotMounted);
        }
        
        let inode_num = file.inode as u32;
        let mut inode = self.inode_mgr.read_inode(inode_num, &self.superblock_mgr)?;
        
        self.block_mgr.truncate_file(&mut inode, new_size)?;
        self.inode_mgr.write_inode(inode_num, &inode, &self.superblock_mgr)?;
        
        Ok(())
    }
    
    fn sync(&mut self) -> FilesystemResult<()> {
        if !self.is_mounted() {
            return Err(FilesystemError::NotMounted);
        }
        
        self.superblock_mgr.sync()?;
        Ok(())
    }
} 