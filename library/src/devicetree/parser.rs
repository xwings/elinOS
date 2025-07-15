// Device Tree Blob (DTB) parser implementation
// Parses Flattened Device Tree format used by OpenSBI and bootloaders

use core::slice;
use core::str;
use heapless::Vec;
use super::{DeviceTreeError, DeviceTreeResult, FDT_MAGIC, FDT_SUPPORTED_VERSION};
use super::node::DeviceTreeNode;
use super::memory::DeviceTreeMemoryRegion;

// FDT header structure (from device tree specification)
#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct FdtHeader {
    magic: u32,           // FDT_MAGIC
    totalsize: u32,       // Total size of the FDT in bytes
    off_dt_struct: u32,   // Offset to structure block
    off_dt_strings: u32,  // Offset to strings block
    off_mem_rsvmap: u32,  // Offset to memory reservation map
    version: u32,         // Version of the FDT format
    last_comp_version: u32, // Last compatible version
    boot_cpuid_phys: u32, // Physical CPU ID of the boot CPU
    size_dt_strings: u32, // Size of strings block
    size_dt_struct: u32,  // Size of structure block
}

// FDT tokens (from device tree specification)
const FDT_BEGIN_NODE: u32 = 0x00000001;
const FDT_END_NODE: u32 = 0x00000002;
const FDT_PROP: u32 = 0x00000003;
const FDT_NOP: u32 = 0x00000004;
const FDT_END: u32 = 0x00000009;

pub struct DeviceTreeParser {
    data: &'static [u8],
    header: FdtHeader,
    struct_block: &'static [u8],
    strings_block: &'static [u8],
}

impl DeviceTreeParser {
    /// Create a new device tree parser from a DTB blob
    /// 
    /// # Safety
    /// The provided address must point to a valid DTB blob in memory
    pub unsafe fn new(dtb_address: usize) -> DeviceTreeResult<Self> {
        // Read the header first to get the total size
        let header_ptr = dtb_address as *const FdtHeader;
        let header = core::ptr::read_volatile(header_ptr);
        
        // Convert from big-endian
        let magic = u32::from_be(header.magic);
        let totalsize = u32::from_be(header.totalsize);
        let version = u32::from_be(header.version);
        
        // Validate magic number
        if magic != FDT_MAGIC {
            return Err(DeviceTreeError::InvalidMagic);
        }
        
        // Validate version
        if version < FDT_SUPPORTED_VERSION {
            return Err(DeviceTreeError::UnsupportedVersion);
        }
        
        // Create slice for the entire DTB
        let data = slice::from_raw_parts(dtb_address as *const u8, totalsize as usize);
        
        let header = FdtHeader {
            magic,
            totalsize,
            off_dt_struct: u32::from_be(header.off_dt_struct),
            off_dt_strings: u32::from_be(header.off_dt_strings),
            off_mem_rsvmap: u32::from_be(header.off_mem_rsvmap),
            version,
            last_comp_version: u32::from_be(header.last_comp_version),
            boot_cpuid_phys: u32::from_be(header.boot_cpuid_phys),
            size_dt_strings: u32::from_be(header.size_dt_strings),
            size_dt_struct: u32::from_be(header.size_dt_struct),
        };
        
        // Validate offsets
        if header.off_dt_struct as usize >= data.len() ||
           header.off_dt_strings as usize >= data.len() {
            return Err(DeviceTreeError::InvalidOffset);
        }
        
        // Create slices for structure and strings blocks
        let struct_start = header.off_dt_struct as usize;
        let struct_end = struct_start + header.size_dt_struct as usize;
        let strings_start = header.off_dt_strings as usize;
        let strings_end = strings_start + header.size_dt_strings as usize;
        
        if struct_end > data.len() || strings_end > data.len() {
            return Err(DeviceTreeError::InvalidOffset);
        }
        
        let struct_block = &data[struct_start..struct_end];
        let strings_block = &data[strings_start..strings_end];
        
        Ok(DeviceTreeParser {
            data,
            header,
            struct_block,
            strings_block,
        })
    }
    
    /// Get the root node of the device tree
    pub fn root_node(&self) -> DeviceTreeResult<DeviceTreeNode> {
        DeviceTreeNode::new(self, 0)
    }
    
    /// Find a node by path (e.g., "/memory", "/cpus/cpu@0")
    pub fn find_node(&self, path: &str) -> DeviceTreeResult<DeviceTreeNode> {
        let root = self.root_node()?;
        
        if path == "/" {
            return Ok(root);
        }
        
        // Split path and traverse
        let path_parts: Vec<&str, 8> = path.trim_start_matches('/').split('/').collect();
        let mut current_node = root;
        
        for part in path_parts {
            if part.is_empty() {
                continue;
            }
            
            // Find child node with matching name
            let mut found = false;
            for child in current_node.children()? {
                if child.name()? == part {
                    current_node = child;
                    found = true;
                    break;
                }
            }
            
            if !found {
                return Err(DeviceTreeError::NodeNotFound);
            }
        }
        
        Ok(current_node)
    }
    
    /// Get memory regions from the device tree
    pub fn get_memory_regions(&self) -> DeviceTreeResult<Vec<DeviceTreeMemoryRegion, 8>> {
        let mut regions = Vec::new();
        
        // Look for memory nodes
        let root = self.root_node()?;
        for child in root.children()? {
            let name = child.name()?;
            
            // Memory nodes are named "memory" or "memory@address"
            if name == "memory" || name.starts_with("memory@") {
                // Check device_type property
                if let Ok(device_type) = child.get_property("device_type") {
                    if device_type.as_str()? == "memory" {
                        if let Ok(region) = DeviceTreeMemoryRegion::from_node(&child) {
                            regions.push(region);
                        }
                    }
                }
            }
        }
        
        Ok(regions)
    }
    
    /// Get string from strings block by offset
    pub(crate) fn get_string(&self, offset: u32) -> DeviceTreeResult<&str> {
        let offset = offset as usize;
        if offset >= self.strings_block.len() {
            return Err(DeviceTreeError::InvalidOffset);
        }
        
        // Find null terminator
        let mut end = offset;
        while end < self.strings_block.len() && self.strings_block[end] != 0 {
            end += 1;
        }
        
        if end >= self.strings_block.len() {
            return Err(DeviceTreeError::InvalidString);
        }
        
        let string_bytes = &self.strings_block[offset..end];
        str::from_utf8(string_bytes).map_err(|_| DeviceTreeError::InvalidString)
    }
    
    /// Get structure block reference
    pub(crate) fn struct_block(&self) -> &[u8] {
        self.struct_block
    }
    
    /// Read a u32 from structure block at offset (with big-endian conversion)
    pub(crate) fn read_u32_be(&self, offset: usize) -> DeviceTreeResult<u32> {
        if offset + 4 > self.struct_block.len() {
            return Err(DeviceTreeError::InvalidOffset);
        }
        
        let bytes = &self.struct_block[offset..offset + 4];
        let value = u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        Ok(value)
    }
    
    /// Align offset to 4-byte boundary
    pub(crate) fn align_offset(offset: usize) -> usize {
        (offset + 3) & !3
    }
}