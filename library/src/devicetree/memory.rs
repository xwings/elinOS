// Device Tree Memory Region handling
// Extracts memory information from device tree nodes

use super::{DeviceTreeError, DeviceTreeResult};
use super::node::DeviceTreeNode;

#[derive(Debug, Clone, Copy)]
pub struct DeviceTreeMemoryRegion {
    pub start: u64,
    pub size: u64,
    pub available: bool,
}

impl DeviceTreeMemoryRegion {
    /// Create a new memory region from a device tree memory node
    pub fn from_node(node: &DeviceTreeNode) -> DeviceTreeResult<Self> {
        // Verify this is a memory node
        if let Ok(device_type) = node.get_property("device_type") {
            if device_type.as_str()? != "memory" {
                return Err(DeviceTreeError::NodeNotFound);
            }
        } else {
            return Err(DeviceTreeError::PropertyNotFound);
        }
        
        // Get the reg property which contains address and size
        let reg_prop = node.get_property("reg")?;
        
        // Get address and size cell counts from parent (usually root)
        // Default to 2 if not found (common for RISC-V)
        let address_cells = 2;
        let size_cells = 2;
        
        // Parse reg property as (address, size) pairs
        let regions = reg_prop.as_reg(address_cells, size_cells)?;
        
        if regions.is_empty() {
            return Err(DeviceTreeError::InvalidOffset);
        }
        
        // Take the first region (most memory nodes have only one)
        let (start, size) = regions[0];
        
        // Check if memory is available (status property)
        let available = if let Ok(status) = node.get_property("status") {
            let status_str = status.as_str()?;
            status_str == "okay" || status_str == "ok"
        } else {
            // If no status property, assume available
            true
        };
        
        Ok(DeviceTreeMemoryRegion {
            start,
            size,
            available,
        })
    }
    
    /// Get memory region as (base, size) tuple for compatibility
    pub fn as_tuple(&self) -> (usize, usize) {
        (self.start as usize, self.size as usize)
    }
    
    /// Check if this memory region is valid
    pub fn is_valid(&self) -> bool {
        self.size > 0 && self.available
    }
    
    /// Get the end address of this memory region
    pub fn end(&self) -> u64 {
        self.start + self.size
    }
    
    /// Check if this region contains the given address
    pub fn contains(&self, address: u64) -> bool {
        address >= self.start && address < self.end()
    }
    
    /// Check if this region overlaps with another region
    pub fn overlaps(&self, other: &Self) -> bool {
        self.start < other.end() && other.start < self.end()
    }
}