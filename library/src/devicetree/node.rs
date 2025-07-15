// Device Tree Node implementation
// Represents nodes in the device tree structure

use core::str;
use heapless::Vec;
use super::{DeviceTreeError, DeviceTreeResult};
use super::parser::DeviceTreeParser;
use super::property::DeviceTreeProperty;

pub struct DeviceTreeNode<'a> {
    parser: &'a DeviceTreeParser,
    offset: usize,
    name: Option<&'a str>,
}

impl<'a> DeviceTreeNode<'a> {
    pub(crate) fn new(parser: &'a DeviceTreeParser, offset: usize) -> DeviceTreeResult<Self> {
        let mut node = DeviceTreeNode {
            parser,
            offset,
            name: None,
        };
        
        // Parse the node name
        node.parse_name()?;
        
        Ok(node)
    }
    
    fn parse_name(&mut self) -> DeviceTreeResult<()> {
        // Read FDT_BEGIN_NODE token
        let token = self.parser.read_u32_be(self.offset)?;
        if token != 0x00000001 {  // FDT_BEGIN_NODE
            return Err(DeviceTreeError::NodeNotFound);
        }
        
        // Node name follows the token
        let name_offset = self.offset + 4;
        let struct_block = self.parser.struct_block();
        
        if name_offset >= struct_block.len() {
            return Err(DeviceTreeError::InvalidOffset);
        }
        
        // Find null terminator for the name
        let mut end = name_offset;
        while end < struct_block.len() && struct_block[end] != 0 {
            end += 1;
        }
        
        if end >= struct_block.len() {
            return Err(DeviceTreeError::InvalidString);
        }
        
        let name_bytes = &struct_block[name_offset..end];
        self.name = Some(str::from_utf8(name_bytes).map_err(|_| DeviceTreeError::InvalidString)?);
        
        Ok(())
    }
    
    /// Get the node name
    pub fn name(&self) -> DeviceTreeResult<&str> {
        self.name.ok_or(DeviceTreeError::NodeNotFound)
    }
    
    /// Get the node name without unit address (part after @)
    pub fn unit_name(&self) -> DeviceTreeResult<&str> {
        let name = self.name()?;
        if let Some(at_pos) = name.find('@') {
            Ok(&name[..at_pos])
        } else {
            Ok(name)
        }
    }
    
    /// Get a property by name
    pub fn get_property(&self, name: &str) -> DeviceTreeResult<DeviceTreeProperty> {
        let mut offset = self.offset;
        let struct_block = self.parser.struct_block();
        
        // Skip FDT_BEGIN_NODE token
        offset += 4;
        
        // Skip node name (null-terminated string)
        while offset < struct_block.len() && struct_block[offset] != 0 {
            offset += 1;
        }
        offset += 1; // Skip null terminator
        
        // Align to 4-byte boundary
        offset = DeviceTreeParser::align_offset(offset);
        
        // Parse properties until we find the one we want or hit FDT_END_NODE
        while offset < struct_block.len() {
            let token = self.parser.read_u32_be(offset)?;
            
            match token {
                0x00000003 => {  // FDT_PROP
                    // Property structure: token(4) + len(4) + nameoff(4) + data(len)
                    let prop_len = self.parser.read_u32_be(offset + 4)?;
                    let name_offset = self.parser.read_u32_be(offset + 8)?;
                    
                    let prop_name = self.parser.get_string(name_offset)?;
                    
                    if prop_name == name {
                        return DeviceTreeProperty::new(self.parser, offset);
                    }
                    
                    // Skip to next property
                    offset += 12 + DeviceTreeParser::align_offset(prop_len as usize);
                }
                0x00000001 => {  // FDT_BEGIN_NODE (child node)
                    // Skip entire child node
                    offset = self.skip_node(offset)?;
                }
                0x00000002 => {  // FDT_END_NODE
                    break;
                }
                0x00000004 => {  // FDT_NOP
                    offset += 4;
                }
                _ => {
                    return Err(DeviceTreeError::InvalidOffset);
                }
            }
        }
        
        Err(DeviceTreeError::PropertyNotFound)
    }
    
    /// Get all properties of this node
    pub fn properties(&self) -> DeviceTreeResult<Vec<DeviceTreeProperty, 16>> {
        let mut properties = Vec::new();
        let mut offset = self.offset;
        let struct_block = self.parser.struct_block();
        
        // Skip FDT_BEGIN_NODE token
        offset += 4;
        
        // Skip node name
        while offset < struct_block.len() && struct_block[offset] != 0 {
            offset += 1;
        }
        offset += 1;
        
        // Align to 4-byte boundary
        offset = DeviceTreeParser::align_offset(offset);
        
        // Parse all properties
        while offset < struct_block.len() {
            let token = self.parser.read_u32_be(offset)?;
            
            match token {
                0x00000003 => {  // FDT_PROP
                    let property = DeviceTreeProperty::new(self.parser, offset)?;
                    properties.push(property);
                    
                    let prop_len = self.parser.read_u32_be(offset + 4)?;
                    offset += 12 + DeviceTreeParser::align_offset(prop_len as usize);
                }
                0x00000001 => {  // FDT_BEGIN_NODE (child node)
                    break;
                }
                0x00000002 => {  // FDT_END_NODE
                    break;
                }
                0x00000004 => {  // FDT_NOP
                    offset += 4;
                }
                _ => {
                    return Err(DeviceTreeError::InvalidOffset);
                }
            }
        }
        
        Ok(properties)
    }
    
    /// Get child nodes
    pub fn children(&self) -> DeviceTreeResult<NodeIterator<'a>> {
        NodeIterator::new(self.parser, self.offset)
    }
    
    /// Skip over a node and all its children
    fn skip_node(&self, start_offset: usize) -> DeviceTreeResult<usize> {
        let mut offset = start_offset;
        let mut depth = 0;
        
        while offset < self.parser.struct_block().len() {
            let token = self.parser.read_u32_be(offset)?;
            
            match token {
                0x00000001 => {  // FDT_BEGIN_NODE
                    depth += 1;
                    offset += 4;
                    
                    // Skip node name
                    while offset < self.parser.struct_block().len() && 
                          self.parser.struct_block()[offset] != 0 {
                        offset += 1;
                    }
                    offset += 1;
                    offset = DeviceTreeParser::align_offset(offset);
                }
                0x00000002 => {  // FDT_END_NODE
                    depth -= 1;
                    offset += 4;
                    if depth == 0 {
                        return Ok(offset);
                    }
                }
                0x00000003 => {  // FDT_PROP
                    let prop_len = self.parser.read_u32_be(offset + 4)?;
                    offset += 12 + DeviceTreeParser::align_offset(prop_len as usize);
                }
                0x00000004 => {  // FDT_NOP
                    offset += 4;
                }
                _ => {
                    return Err(DeviceTreeError::InvalidOffset);
                }
            }
        }
        
        Err(DeviceTreeError::InvalidOffset)
    }
}

pub struct NodeIterator<'a> {
    parser: &'a DeviceTreeParser,
    offset: usize,
    done: bool,
}

impl<'a> NodeIterator<'a> {
    pub(crate) fn new(parser: &'a DeviceTreeParser, parent_offset: usize) -> DeviceTreeResult<Self> {
        let mut offset = parent_offset;
        let struct_block = parser.struct_block();
        
        // Skip FDT_BEGIN_NODE token
        offset += 4;
        
        // Skip node name
        while offset < struct_block.len() && struct_block[offset] != 0 {
            offset += 1;
        }
        offset += 1;
        offset = DeviceTreeParser::align_offset(offset);
        
        // Skip all properties to get to first child
        while offset < struct_block.len() {
            let token = parser.read_u32_be(offset)?;
            
            match token {
                0x00000003 => {  // FDT_PROP
                    let prop_len = parser.read_u32_be(offset + 4)?;
                    offset += 12 + DeviceTreeParser::align_offset(prop_len as usize);
                }
                0x00000001 => {  // FDT_BEGIN_NODE (first child)
                    break;
                }
                0x00000002 => {  // FDT_END_NODE (no children)
                    return Ok(NodeIterator {
                        parser,
                        offset,
                        done: true,
                    });
                }
                0x00000004 => {  // FDT_NOP
                    offset += 4;
                }
                _ => {
                    return Err(DeviceTreeError::InvalidOffset);
                }
            }
        }
        
        Ok(NodeIterator {
            parser,
            offset,
            done: false,
        })
    }
}

impl<'a> Iterator for NodeIterator<'a> {
    type Item = DeviceTreeNode<'a>;
    
    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }
        
        // Check if we're at a BEGIN_NODE token
        if let Ok(token) = self.parser.read_u32_be(self.offset) {
            if token == 0x00000001 {  // FDT_BEGIN_NODE
                let current_offset = self.offset;
                
                // Create node for current position
                if let Ok(node) = DeviceTreeNode::new(self.parser, current_offset) {
                    // Skip to next sibling
                    if let Ok(next_offset) = node.skip_node(current_offset) {
                        self.offset = next_offset;
                        
                        // Check if we're at END_NODE (no more siblings)
                        if let Ok(token) = self.parser.read_u32_be(self.offset) {
                            if token == 0x00000002 {  // FDT_END_NODE
                                self.done = true;
                            }
                        }
                        
                        return Some(node);
                    }
                }
            }
        }
        
        self.done = true;
        None
    }
}