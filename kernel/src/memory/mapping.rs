// Memory Mapping Management for elinOS
// Centralized memory mapping API to prevent overlapping and provide clean interface
// Inspired by Qiling framework memory management

use heapless::{FnvIndexMap, Vec, String};
use spin::Mutex;
use lazy_static::lazy_static;
use crate::console_println;

/// Memory mapping permissions
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MemoryPermissions {
    pub read: bool,
    pub write: bool,
    pub execute: bool,
}

impl MemoryPermissions {
    pub const READ: Self = MemoryPermissions { read: true, write: false, execute: false };
    pub const WRITE: Self = MemoryPermissions { read: false, write: true, execute: false };
    pub const EXECUTE: Self = MemoryPermissions { read: false, write: false, execute: true };
    pub const READ_WRITE: Self = MemoryPermissions { read: true, write: true, execute: false };
    pub const READ_EXECUTE: Self = MemoryPermissions { read: true, write: false, execute: true };
    pub const READ_WRITE_EXECUTE: Self = MemoryPermissions { read: true, write: true, execute: true };
    pub const NONE: Self = MemoryPermissions { read: false, write: false, execute: false };
}

/// Memory mapping type
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MappingType {
    /// Physical memory mapping (direct hardware access)
    Physical,
    /// Virtual memory mapping (allocated from heap)
    Virtual,
    /// Device memory mapping (MMIO)
    Device,
    /// Framebuffer memory
    Framebuffer,
    /// DMA buffer
    DmaBuffer,
}

/// Memory mapping entry
#[derive(Debug, Clone)]
pub struct MemoryMapping {
    pub start_addr: usize,
    pub size: usize,
    pub permissions: MemoryPermissions,
    pub mapping_type: MappingType,
    pub name: String<64>,
    pub physical_addr: Option<usize>, // For virtual mappings
}

impl MemoryMapping {
    pub fn end_addr(&self) -> usize {
        self.start_addr + self.size
    }

    pub fn overlaps_with(&self, other: &MemoryMapping) -> bool {
        !(self.end_addr() <= other.start_addr || other.end_addr() <= self.start_addr)
    }

    pub fn contains_address(&self, addr: usize) -> bool {
        addr >= self.start_addr && addr < self.end_addr()
    }
}

/// Memory mapping manager
pub struct MemoryMappingManager {
    mappings: FnvIndexMap<usize, MemoryMapping, 64>,
    next_virtual_addr: usize,
    total_mapped: usize,
}

impl MemoryMappingManager {
    pub fn new() -> Self {
        Self {
            mappings: FnvIndexMap::new(),
            next_virtual_addr: 0x40000000, // Start virtual mappings at 1GB
            total_mapped: 0,
        }
    }

    /// Map memory region with overlap checking
    pub fn map_memory(
        &mut self,
        addr: usize,
        size: usize,
        permissions: MemoryPermissions,
        mapping_type: MappingType,
        name: &str,
    ) -> Result<usize, &'static str> {
        if size == 0 {
            return Err("Cannot map zero-sized region");
        }

        // Align size to page boundary (4KB)
        let aligned_size = (size + 4095) & !4095;

        let new_mapping = MemoryMapping {
            start_addr: addr,
            size: aligned_size,
            permissions,
            mapping_type,
            name: String::try_from(name).unwrap_or_default(),
            physical_addr: None,
        };

        // Check for overlaps with existing mappings
        for (_, existing) in &self.mappings {
            if new_mapping.overlaps_with(existing) {
                return Err("Memory region overlaps with existing mapping");
            }
        }

        // Insert the mapping
        let _ = self.mappings.insert(addr, new_mapping);
        self.total_mapped += aligned_size;

        Ok(addr)
    }

    /// Allocate and map virtual memory
    pub fn map_virtual_memory(
        &mut self,
        size: usize,
        permissions: MemoryPermissions,
        mapping_type: MappingType,
        name: &str,
    ) -> Result<usize, &'static str> {
        if size == 0 {
            return Err("Cannot map zero-sized region");
        }

        // Align size to page boundary (4KB)
        let aligned_size = (size + 4095) & !4095;

        // Find a suitable virtual address
        let virtual_addr = self.find_free_virtual_address(aligned_size)?;

        // Try to allocate physical memory
        let physical_addr = match crate::memory::allocate_aligned_memory(aligned_size, 4096) {
            Some(addr) => addr,
            None => return Err("Failed to allocate physical memory"),
        };

        // CRITICAL: Zero out the allocated physical memory
        unsafe {
            core::ptr::write_bytes(physical_addr as *mut u8, 0, aligned_size);
        }

        // For VirtIO DMA operations, we need to use physical addresses directly
        // Return the physical address as the "virtual" address for now
        // This is a temporary workaround until proper virtual memory is implemented
        let mapped_addr = if name.contains("VirtIO") {
            physical_addr  // Use physical address directly for VirtIO
        } else {
            virtual_addr   // Use virtual address for other mappings
        };

        let new_mapping = MemoryMapping {
            start_addr: mapped_addr,
            size: aligned_size,
            permissions,
            mapping_type,
            name: String::try_from(name).unwrap_or_default(),
            physical_addr: Some(physical_addr),
        };

        // Insert the mapping
        let _ = self.mappings.insert(mapped_addr, new_mapping);
        self.total_mapped += aligned_size;

        Ok(mapped_addr)
    }

    /// Unmap memory region
    pub fn unmap_memory(&mut self, addr: usize) -> Result<(), &'static str> {
        if let Some(mapping) = self.mappings.remove(&addr) {
            // If it was a virtual mapping, free the physical memory
            if let Some(physical_addr) = mapping.physical_addr {
                crate::memory::deallocate_memory(physical_addr, mapping.size);
            }
            self.total_mapped -= mapping.size;
            Ok(())
        } else {
            Err("Memory region not found")
        }
    }

    /// Find information about a memory address
    pub fn find_mapping(&self, addr: usize) -> Option<&MemoryMapping> {
        for (_, mapping) in &self.mappings {
            if mapping.contains_address(addr) {
                return Some(mapping);
            }
        }
        None
    }

    /// Check if an address range is valid and has required permissions
    pub fn check_access(&self, addr: usize, size: usize, write: bool, execute: bool) -> bool {
        let end_addr = addr + size;
        
        for check_addr in (addr..end_addr).step_by(4096) {
            if let Some(mapping) = self.find_mapping(check_addr) {
                if write && !mapping.permissions.write {
                    return false;
                }
                if execute && !mapping.permissions.execute {
                    return false;
                }
                if !mapping.permissions.read {
                    return false;
                }
            } else {
                return false; // Address not mapped
            }
        }
        true
    }

    /// Find a free virtual address range
    fn find_free_virtual_address(&mut self, size: usize) -> Result<usize, &'static str> {
        let mut candidate = self.next_virtual_addr;
        
        // Align to page boundary
        candidate = (candidate + 4095) & !4095;

        // Search for a free range
        loop {
            let candidate_end = candidate + size;
            let mut overlaps = false;

            for (_, mapping) in &self.mappings {
                if candidate < mapping.end_addr() && candidate_end > mapping.start_addr {
                    overlaps = true;
                    candidate = mapping.end_addr();
                    break;
                }
            }

            if !overlaps {
                self.next_virtual_addr = candidate_end;
                return Ok(candidate);
            }

            // Prevent infinite loop
            if candidate > 0x80000000 {
                return Err("Virtual address space exhausted");
            }
        }
    }

    /// Get all mappings for debugging
    pub fn get_mappings(&self) -> Vec<&MemoryMapping, 32> {
        let mut result = Vec::new();
        for (_, mapping) in &self.mappings {
            if result.push(mapping).is_err() {
                break;
            }
        }
        result
    }

    /// Get memory mapping statistics
    pub fn get_stats(&self) -> MappingStats {
        let mut stats = MappingStats {
            total_mappings: self.mappings.len(),
            total_mapped_size: self.total_mapped,
            virtual_mappings: 0,
            physical_mappings: 0,
            device_mappings: 0,
            framebuffer_mappings: 0,
        };

        for (_, mapping) in &self.mappings {
            match mapping.mapping_type {
                MappingType::Virtual => stats.virtual_mappings += 1,
                MappingType::Physical => stats.physical_mappings += 1,
                MappingType::Device => stats.device_mappings += 1,
                MappingType::Framebuffer => stats.framebuffer_mappings += 1,
                MappingType::DmaBuffer => stats.virtual_mappings += 1,
            }
        }

        stats
    }

    /// Clear all mappings (for testing/reset)
    pub fn clear_all_mappings(&mut self) {
        for (_, mapping) in &self.mappings {
            if let Some(physical_addr) = mapping.physical_addr {
                crate::memory::deallocate_memory(physical_addr, mapping.size);
            }
        }
        self.mappings.clear();
        self.total_mapped = 0;
        self.next_virtual_addr = 0x40000000;
    }
}

/// Memory mapping statistics
#[derive(Debug, Clone, Copy)]
pub struct MappingStats {
    pub total_mappings: usize,
    pub total_mapped_size: usize,
    pub virtual_mappings: usize,
    pub physical_mappings: usize,
    pub device_mappings: usize,
    pub framebuffer_mappings: usize,
}

// Global memory mapping manager
lazy_static! {
    pub static ref MEMORY_MAPPER: Mutex<MemoryMappingManager> = 
        Mutex::new(MemoryMappingManager::new());
}

/// High-level API functions for memory mapping

/// Map a physical memory region
pub fn map_physical_memory(
    physical_addr: usize,
    size: usize,
    permissions: MemoryPermissions,
    name: &str,
) -> Result<usize, &'static str> {
    let mut mapper = MEMORY_MAPPER.lock();
    mapper.map_memory(physical_addr, size, permissions, MappingType::Physical, name)
}

/// Map device memory (MMIO)
pub fn map_device_memory(
    device_addr: usize,
    size: usize,
    name: &str,
) -> Result<usize, &'static str> {
    let mut mapper = MEMORY_MAPPER.lock();
    mapper.map_memory(device_addr, size, MemoryPermissions::READ_WRITE, MappingType::Device, name)
}

/// Allocate and map virtual memory
pub fn map_virtual_memory(
    size: usize,
    permissions: MemoryPermissions,
    name: &str,
) -> Result<usize, &'static str> {
    let mut mapper = MEMORY_MAPPER.lock();
    mapper.map_virtual_memory(size, permissions, MappingType::Virtual, name)
}

/// Map framebuffer memory
pub fn map_framebuffer_memory(
    physical_addr: usize,
    size: usize,
    name: &str,
) -> Result<usize, &'static str> {
    let mut mapper = MEMORY_MAPPER.lock();
    mapper.map_memory(physical_addr, size, MemoryPermissions::READ_WRITE, MappingType::Framebuffer, name)
}

/// Unmap memory region
pub fn unmap_memory(addr: usize) -> Result<(), &'static str> {
    let mut mapper = MEMORY_MAPPER.lock();
    mapper.unmap_memory(addr)
}

/// Check if memory access is valid
pub fn check_memory_access(addr: usize, size: usize, write: bool, execute: bool) -> bool {
    let mapper = MEMORY_MAPPER.lock();
    mapper.check_access(addr, size, write, execute)
}

/// Find mapping information for an address
pub fn find_memory_mapping(addr: usize) -> Option<MemoryMapping> {
    let mapper = MEMORY_MAPPER.lock();
    mapper.find_mapping(addr).cloned()
}

/// Get memory mapping statistics
pub fn get_mapping_stats() -> MappingStats {
    let mapper = MEMORY_MAPPER.lock();
    mapper.get_stats()
}

/// Show all memory mappings (for debugging)
pub fn show_memory_mappings() {
    let mapper = MEMORY_MAPPER.lock();
    let mappings = mapper.get_mappings();
    
    console_println!("=== Memory Mappings ===");
    console_println!("Total mappings: {}", mappings.len());
    
    for mapping in mappings.iter() {
        console_println!(
            "{}: 0x{:08x}-0x{:08x} ({} KB) {:?} {:?}",
            mapping.name,
            mapping.start_addr,
            mapping.end_addr(),
            mapping.size / 1024,
            mapping.mapping_type,
            mapping.permissions
        );
    }
    
    let stats = mapper.get_stats();
    console_println!("Total mapped: {} KB", stats.total_mapped_size / 1024);
    console_println!("Virtual: {}, Physical: {}, Device: {}, FB: {}", 
        stats.virtual_mappings, stats.physical_mappings, 
        stats.device_mappings, stats.framebuffer_mappings);
} 