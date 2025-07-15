// Device Tree integration with SBI for elinOS
// Provides device tree parsing capability to SBI memory detection

use crate::sbi::{SbiMemoryInfo, SbiMemoryRegion};
use super::DeviceTreeParser;

/// Parse device tree to extract memory information for SBI compatibility
pub fn parse_device_tree_memory(dtb_address: usize) -> Option<SbiMemoryInfo> {
    // Safety: We assume the DTB address is valid - this should be validated by caller
    let parser = unsafe { DeviceTreeParser::new(dtb_address) };
    
    let parser = match parser {
        Ok(p) => p,
        Err(_) => return None,
    };
    
    // Get memory regions from device tree
    let dt_regions = match parser.get_memory_regions() {
        Ok(regions) => regions,
        Err(_) => return None,
    };
    
    // Convert to SBI memory info format
    let mut info = SbiMemoryInfo {
        regions: [SbiMemoryRegion { start: 0, size: 0, flags: 0 }; 8],
        count: 0,
    };
    
    // Add memory regions (RAM)
    for (i, region) in dt_regions.iter().enumerate() {
        if i >= 8 { break; } // SBI info has max 8 regions
        
        if region.is_valid() {
            info.regions[i] = SbiMemoryRegion {
                start: region.start as usize,
                size: region.size as usize,
                flags: 1, // RAM
            };
            info.count += 1;
        }
    }
    
    // Add standard RISC-V MMIO regions if we have space
    if info.count < 8 {
        add_standard_mmio_regions(&mut info);
    }
    
    Some(info)
}

/// Extract basic memory information (base, size) from device tree
pub fn get_memory_info_from_dt(dtb_address: usize) -> Option<(usize, usize)> {
    // Safety: We assume the DTB address is valid - this should be validated by caller
    let parser = unsafe { DeviceTreeParser::new(dtb_address) };
    
    let parser = match parser {
        Ok(p) => p,
        Err(_) => return None,
    };
    
    // Get memory regions
    let regions = match parser.get_memory_regions() {
        Ok(regions) => regions,
        Err(_) => return None,
    };
    
    // Find the first valid memory region
    for region in regions {
        if region.is_valid() {
            return Some(region.as_tuple());
        }
    }
    
    None
}

/// Add standard RISC-V MMIO regions to SBI memory info
fn add_standard_mmio_regions(info: &mut SbiMemoryInfo) {
    let mmio_regions = [
        (0x10000000, 0x1000),    // UART
        (0x02000000, 0x10000),   // CLINT  
        (0x0c000000, 0x400000),  // PLIC
    ];
    
    for (start, size) in mmio_regions {
        if info.count >= 8 { break; }
        
        info.regions[info.count] = SbiMemoryRegion {
            start,
            size,
            flags: 0, // MMIO
        };
        info.count += 1;
    }
}

/// Get CPU information from device tree
pub fn get_cpu_info_from_dt(dtb_address: usize) -> Option<(u32, u32)> {
    // Safety: We assume the DTB address is valid - this should be validated by caller
    let parser = unsafe { DeviceTreeParser::new(dtb_address) };
    
    let parser = match parser {
        Ok(p) => p,
        Err(_) => return None,
    };
    
    // Look for /cpus node
    let cpus_node = match parser.find_node("/cpus") {
        Ok(node) => node,
        Err(_) => return None,
    };
    
    // Get timebase frequency
    let timebase_freq = if let Ok(prop) = cpus_node.get_property("timebase-frequency") {
        prop.as_u32().unwrap_or(10000000) // Default to 10MHz
    } else {
        10000000 // Default to 10MHz
    };
    
    // Count CPU cores
    let mut cpu_count = 0;
    if let Ok(children) = cpus_node.children() {
        for child in children {
            if let Ok(name) = child.unit_name() {
                if name == "cpu" {
                    cpu_count += 1;
                }
            }
        }
    }
    
    Some((cpu_count, timebase_freq))
}

/// Check if device tree parsing is available
pub fn is_device_tree_available(dtb_address: usize) -> bool {
    // Safety: We assume the DTB address is valid - this should be validated by caller
    if dtb_address == 0 {
        return false;
    }
    
    // Try to create parser - if it succeeds, device tree is available
    let parser = unsafe { DeviceTreeParser::new(dtb_address) };
    parser.is_ok()
}