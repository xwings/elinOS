// Shared memory region types and management for elinOS
// Used by both bootloader and kernel

// Memory region structure
#[derive(Debug, Clone, Copy)]
pub struct MemoryRegion {
    pub start: usize,
    pub size: usize,
    pub is_ram: bool,
    pub zone_type: MemoryZone,
}

// Memory zones similar to Linux
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MemoryZone {
    DMA,        // Direct Memory Access zone (first 16MB)
    Normal,     // Normal memory zone
    High,       // High memory zone (if applicable)
}

impl MemoryRegion {
    /// Create a new memory region
    pub const fn new(start: usize, size: usize, is_ram: bool, zone_type: MemoryZone) -> Self {
        Self {
            start,
            size,
            is_ram,
            zone_type,
        }
    }
    
    /// Get the end address of this region
    pub const fn end(&self) -> usize {
        self.start + self.size
    }
    
    /// Check if an address is within this region
    pub const fn contains(&self, addr: usize) -> bool {
        addr >= self.start && addr < self.end()
    }
    
    /// Check if this region overlaps with another region
    pub const fn overlaps_with(&self, other: &MemoryRegion) -> bool {
        self.start < other.end() && other.start < self.end()
    }
}

impl MemoryZone {
    /// Get the priority of this zone (lower number = higher priority)
    pub const fn priority(&self) -> u8 {
        match self {
            MemoryZone::DMA => 0,
            MemoryZone::Normal => 1,
            MemoryZone::High => 2,
        }
    }
    
    /// Check if this zone is suitable for DMA operations
    pub const fn is_dma_capable(&self) -> bool {
        matches!(self, MemoryZone::DMA)
    }
}