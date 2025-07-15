// Device Tree Property implementation
// Represents properties within device tree nodes

use core::str;
use heapless::Vec;
use super::{DeviceTreeError, DeviceTreeResult};
use super::parser::DeviceTreeParser;

pub struct DeviceTreeProperty<'a> {
    parser: &'a DeviceTreeParser,
    offset: usize,
    name: &'a str,
    data: &'a [u8],
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PropertyType {
    String,
    U32,
    U64,
    StringList,
    U32Array,
    U64Array,
    ByteArray,
    Bool,  // Empty property (presence indicates true)
}

impl<'a> DeviceTreeProperty<'a> {
    pub(crate) fn new(parser: &'a DeviceTreeParser, offset: usize) -> DeviceTreeResult<Self> {
        // Property structure: token(4) + len(4) + nameoff(4) + data(len)
        let token = parser.read_u32_be(offset)?;
        if token != 0x00000003 {  // FDT_PROP
            return Err(DeviceTreeError::PropertyNotFound);
        }
        
        let len = parser.read_u32_be(offset + 4)? as usize;
        let name_offset = parser.read_u32_be(offset + 8)?;
        
        let name = parser.get_string(name_offset)?;
        
        let data_offset = offset + 12;
        let struct_block = parser.struct_block();
        
        if data_offset + len > struct_block.len() {
            return Err(DeviceTreeError::InvalidOffset);
        }
        
        let data = &struct_block[data_offset..data_offset + len];
        
        Ok(DeviceTreeProperty {
            parser,
            offset,
            name,
            data,
        })
    }
    
    /// Get property name
    pub fn name(&self) -> &str {
        self.name
    }
    
    /// Get raw property data
    pub fn data(&self) -> &[u8] {
        self.data
    }
    
    /// Get property length
    pub fn len(&self) -> usize {
        self.data.len()
    }
    
    /// Check if property is empty (boolean property)
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }
    
    /// Determine the property type based on common conventions
    pub fn property_type(&self) -> PropertyType {
        match self.name {
            "compatible" | "status" | "device_type" | "model" | "name" => PropertyType::String,
            "reg" | "ranges" => PropertyType::U32Array,
            "#address-cells" | "#size-cells" | "clock-frequency" | "timebase-frequency" => PropertyType::U32,
            _ => {
                if self.data.is_empty() {
                    PropertyType::Bool
                } else if self.data.len() == 4 {
                    PropertyType::U32
                } else if self.data.len() == 8 {
                    PropertyType::U64
                } else if self.data.len() % 4 == 0 {
                    PropertyType::U32Array
                } else {
                    PropertyType::ByteArray
                }
            }
        }
    }
    
    /// Get property as string
    pub fn as_str(&self) -> DeviceTreeResult<&str> {
        if self.data.is_empty() {
            return Err(DeviceTreeError::InvalidString);
        }
        
        // Find the first null terminator
        let mut end = 0;
        while end < self.data.len() && self.data[end] != 0 {
            end += 1;
        }
        
        if end == 0 {
            return Ok("");
        }
        
        let string_bytes = &self.data[..end];
        str::from_utf8(string_bytes).map_err(|_| DeviceTreeError::InvalidString)
    }
    
    /// Get property as string list
    pub fn as_string_list(&self) -> DeviceTreeResult<Vec<&str, 8>> {
        let mut strings = Vec::new();
        let mut start = 0;
        
        while start < self.data.len() {
            let mut end = start;
            while end < self.data.len() && self.data[end] != 0 {
                end += 1;
            }
            
            if end > start {
                let string_bytes = &self.data[start..end];
                let string = str::from_utf8(string_bytes).map_err(|_| DeviceTreeError::InvalidString)?;
                strings.push(string);
            }
            
            start = end + 1;
        }
        
        Ok(strings)
    }
    
    /// Get property as u32 (big-endian)
    pub fn as_u32(&self) -> DeviceTreeResult<u32> {
        if self.data.len() != 4 {
            return Err(DeviceTreeError::InvalidOffset);
        }
        
        let bytes = [self.data[0], self.data[1], self.data[2], self.data[3]];
        Ok(u32::from_be_bytes(bytes))
    }
    
    /// Get property as u64 (big-endian)
    pub fn as_u64(&self) -> DeviceTreeResult<u64> {
        if self.data.len() != 8 {
            return Err(DeviceTreeError::InvalidOffset);
        }
        
        let bytes = [
            self.data[0], self.data[1], self.data[2], self.data[3],
            self.data[4], self.data[5], self.data[6], self.data[7]
        ];
        Ok(u64::from_be_bytes(bytes))
    }
    
    /// Get property as u32 array (big-endian)
    pub fn as_u32_array(&self) -> DeviceTreeResult<Vec<u32, 16>> {
        if self.data.len() % 4 != 0 {
            return Err(DeviceTreeError::InvalidOffset);
        }
        
        let mut values = Vec::new();
        for chunk in self.data.chunks_exact(4) {
            let bytes = [chunk[0], chunk[1], chunk[2], chunk[3]];
            values.push(u32::from_be_bytes(bytes));
        }
        
        Ok(values)
    }
    
    /// Get property as u64 array (big-endian)
    pub fn as_u64_array(&self) -> DeviceTreeResult<Vec<u64, 8>> {
        if self.data.len() % 8 != 0 {
            return Err(DeviceTreeError::InvalidOffset);
        }
        
        let mut values = Vec::new();
        for chunk in self.data.chunks_exact(8) {
            let bytes = [
                chunk[0], chunk[1], chunk[2], chunk[3],
                chunk[4], chunk[5], chunk[6], chunk[7]
            ];
            values.push(u64::from_be_bytes(bytes));
        }
        
        Ok(values)
    }
    
    /// Get property as reg (address, size) pairs
    /// This is a common pattern in device trees
    pub fn as_reg(&self, address_cells: u32, size_cells: u32) -> DeviceTreeResult<Vec<(u64, u64), 8>> {
        let cell_size = (address_cells + size_cells) as usize * 4;
        
        if self.data.len() % cell_size != 0 {
            return Err(DeviceTreeError::InvalidOffset);
        }
        
        let mut regions = Vec::new();
        
        for chunk in self.data.chunks_exact(cell_size) {
            let mut address = 0u64;
            let mut size = 0u64;
            
            // Read address
            for i in 0..address_cells as usize {
                let offset = i * 4;
                let cell_bytes = [chunk[offset], chunk[offset + 1], chunk[offset + 2], chunk[offset + 3]];
                let cell_value = u32::from_be_bytes(cell_bytes) as u64;
                address = (address << 32) | cell_value;
            }
            
            // Read size
            for i in 0..size_cells as usize {
                let offset = (address_cells as usize + i) * 4;
                let cell_bytes = [chunk[offset], chunk[offset + 1], chunk[offset + 2], chunk[offset + 3]];
                let cell_value = u32::from_be_bytes(cell_bytes) as u64;
                size = (size << 32) | cell_value;
            }
            
            regions.push((address, size));
        }
        
        Ok(regions)
    }
    
    /// Check if property matches a compatible string
    pub fn is_compatible(&self, compatible: &str) -> bool {
        if self.name != "compatible" {
            return false;
        }
        
        if let Ok(compatibles) = self.as_string_list() {
            compatibles.iter().any(|&c| c == compatible)
        } else {
            false
        }
    }
}