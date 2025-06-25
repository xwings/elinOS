// Slab Allocator Implementation for elinOS
// High-level allocator working on top of the buddy allocator
// Inspired by Maestro OS and Linux kernel slab allocator

use core::ptr::NonNull;
use crate::memory::buddy::{BuddyAllocator, BuddyError};
use heapless::Vec;

/// Size classes for the slab allocator (powers of 2)
const SIZE_CLASSES: &[usize] = &[
    8, 16, 32, 64, 128, 256, 512, 1024, 2048, 4096
];

/// Maximum number of slabs per size class
const MAX_SLABS_PER_CLASS: usize = 16;

/// A slab contains multiple objects of the same size
#[derive(Debug)]
struct Slab {
    /// Base address of this slab
    base_addr: usize,
    /// Size of each object in this slab
    object_size: usize,
    /// Number of objects that can fit in this slab
    capacity: usize,
    /// Bitmap tracking which objects are allocated
    allocated_bitmap: u64, // Supports up to 64 objects per slab
    /// Number of free objects remaining
    free_count: usize,
}

/// Statistics for the slab allocator
#[derive(Debug, Clone, Copy)]
pub struct SlabStats {
    pub total_allocations: usize,
    pub total_deallocations: usize,
    pub slab_allocations: usize,
    pub total_slab_memory: usize,
    pub total_objects_allocated: usize,
    pub total_objects_capacity: usize,
    pub fragmentation_ratio: f32,
}

/// Wrapper type for slab arrays to avoid orphan rules
#[derive(Debug)]
struct SlabArrays {
    arrays: [Vec<Slab, MAX_SLABS_PER_CLASS>; SIZE_CLASSES.len()],
}

impl Default for SlabArrays {
    fn default() -> Self {
        Self {
            arrays: [
                Vec::new(), Vec::new(), Vec::new(), Vec::new(), Vec::new(),
                Vec::new(), Vec::new(), Vec::new(), Vec::new(), Vec::new(),
            ]
        }
    }
}

impl core::ops::Index<usize> for SlabArrays {
    type Output = Vec<Slab, MAX_SLABS_PER_CLASS>;
    
    fn index(&self, index: usize) -> &Self::Output {
        &self.arrays[index]
    }
}

impl core::ops::IndexMut<usize> for SlabArrays {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.arrays[index]
    }
}

/// Slab allocator that works on top of buddy allocator
pub struct SlabAllocator {
    /// The underlying buddy allocator
    buddy: BuddyAllocator,
    /// Slabs for each size class
    slabs: SlabArrays,
    /// Statistics
    total_allocations: usize,
    total_deallocations: usize,
    slab_allocations: usize,
}

impl SlabAllocator {
    /// Create a new slab allocator on top of buddy allocator
    pub fn new(base_address: usize, total_size: usize) -> Result<Self, BuddyError> {
        let buddy = BuddyAllocator::new(base_address, total_size)?;
        
        let mut allocator = SlabAllocator {
            buddy,
            slabs: SlabArrays::default(),
            total_allocations: 0,
            total_deallocations: 0,
            slab_allocations: 0,
        };
        
        // Pre-allocate some slabs for common sizes
        allocator.preallocate_common_slabs()?;
        
        Ok(allocator)
    }
    
    /// Allocate memory of the given size
    pub fn allocate(&mut self, size: usize) -> Option<NonNull<u8>> {
        self.total_allocations += 1;
        
        // For large allocations, use buddy allocator directly
        if size > SIZE_CLASSES[SIZE_CLASSES.len() - 1] {
            let addr = self.buddy.allocate(size)?;
            return NonNull::new(addr as *mut u8);
        }
        
        // Find appropriate size class
        let size_class_idx = self.find_size_class(size)?;
        
        // Try to allocate from existing slab
        if let Some(addr) = self.allocate_from_slab(size_class_idx) {
            return NonNull::new(addr as *mut u8);
        }
        
        // No suitable slab found, create a new one
        if self.create_new_slab(size_class_idx).is_ok() {
            if let Some(addr) = self.allocate_from_slab(size_class_idx) {
                return NonNull::new(addr as *mut u8);
            }
        }
        
        // Fallback to buddy allocator for the actual size requested
        let addr = self.buddy.allocate(SIZE_CLASSES[size_class_idx])?;
        NonNull::new(addr as *mut u8)
    }
    
    /// Deallocate memory at the given address
    pub fn deallocate(&mut self, ptr: NonNull<u8>, size: usize) {
        self.total_deallocations += 1;
        let addr = ptr.as_ptr() as usize;
        
        // For large allocations, use buddy allocator directly
        if size > SIZE_CLASSES[SIZE_CLASSES.len() - 1] {
            self.buddy.deallocate(addr, size);
            return;
        }
        
        // Find the size class and deallocate from slab
        if let Some(size_class_idx) = self.find_size_class(size) {
            if self.deallocate_from_slab(size_class_idx, addr) {
                return;
            }
        }
        
        // Fallback to buddy allocator
        self.buddy.deallocate(addr, size);
    }
    
    /// Pre-allocate slabs for commonly used sizes
    fn preallocate_common_slabs(&mut self) -> Result<(), BuddyError> {
        // Pre-allocate slabs for 32, 64, 128, 256 byte objects
        let common_sizes = [2, 3, 4, 5]; // Indices in SIZE_CLASSES
        
        for &size_idx in &common_sizes {
            let _ = self.create_new_slab(size_idx);
        }
        
        Ok(())
    }
    
    /// Find the appropriate size class for the given size
    fn find_size_class(&self, size: usize) -> Option<usize> {
        SIZE_CLASSES.iter().position(|&class_size| class_size >= size)
    }
    
    /// Allocate an object from an existing slab
    fn allocate_from_slab(&mut self, size_class_idx: usize) -> Option<usize> {
        let slabs = &mut self.slabs.arrays[size_class_idx];
        
        for slab in slabs.iter_mut() {
            if slab.free_count > 0 {
                if let Some(object_addr) = slab.allocate_object() {
                    return Some(object_addr);
                }
            }
        }
        
        None
    }
    
    /// Deallocate an object from a slab
    fn deallocate_from_slab(&mut self, size_class_idx: usize, addr: usize) -> bool {
        let slabs = &mut self.slabs.arrays[size_class_idx];
        
        for slab in slabs.iter_mut() {
            if slab.owns_address(addr) {
                slab.deallocate_object(addr);
                
                // If slab becomes empty, consider returning it to buddy allocator
                if slab.is_empty() && slabs.len() > 1 {
                    // Keep at least one empty slab for future allocations
                    // This is a simple heuristic
                }
                
                return true;
            }
        }
        
        false
    }
    
    /// Create a new slab for the given size class
    fn create_new_slab(&mut self, size_class_idx: usize) -> Result<(), BuddyError> {
        let object_size = SIZE_CLASSES[size_class_idx];
        
        // Calculate slab size (aim for ~4KB slabs)
        let slab_size = if object_size <= 256 {
            4096
        } else {
            // For larger objects, use smaller slabs
            core::cmp::max(object_size * 8, 4096)
        };
        
        // Allocate memory from buddy allocator
        let base_addr = self.buddy.allocate(slab_size)
            .ok_or(BuddyError::OutOfMemory)?;
        
        let capacity = slab_size / object_size;
        if capacity == 0 || capacity > 64 {
            // Return the memory if we can't use it effectively
            self.buddy.deallocate(base_addr, slab_size);
            return Err(BuddyError::InvalidSize);
        }
        
        let slab = Slab {
            base_addr,
            object_size,
            capacity,
            allocated_bitmap: 0,
            free_count: capacity,
        };
        
        self.slabs.arrays[size_class_idx].push(slab)
            .map_err(|_| BuddyError::OutOfMemory)?;
        
        self.slab_allocations += 1;
        
        Ok(())
    }
    
    /// Get allocator statistics
    pub fn get_stats(&self) -> SlabStats {
        let mut total_slab_memory = 0;
        let mut total_objects_allocated = 0;
        let mut total_objects_capacity = 0;
        
        for size_class_slabs in &self.slabs.arrays {
            for slab in size_class_slabs {
                total_slab_memory += slab.capacity * slab.object_size;
                total_objects_allocated += slab.capacity - slab.free_count;
                total_objects_capacity += slab.capacity;
            }
        }
        
        SlabStats {
            total_allocations: self.total_allocations,
            total_deallocations: self.total_deallocations,
            slab_allocations: self.slab_allocations,
            total_slab_memory,
            total_objects_allocated,
            total_objects_capacity,
            fragmentation_ratio: if total_objects_capacity > 0 {
                (total_objects_capacity - total_objects_allocated) as f32 / total_objects_capacity as f32
            } else {
                0.0
            },
        }
    }
}

impl Slab {
    /// Allocate an object from this slab
    fn allocate_object(&mut self) -> Option<usize> {
        if self.free_count == 0 {
            return None;
        }
        
        // Find first free bit
        for i in 0..self.capacity {
            if (self.allocated_bitmap & (1 << i)) == 0 {
                // Mark as allocated
                self.allocated_bitmap |= 1 << i;
                self.free_count -= 1;
                
                return Some(self.base_addr + i * self.object_size);
            }
        }
        
        None
    }
    
    /// Deallocate an object from this slab
    fn deallocate_object(&mut self, addr: usize) {
        if !self.owns_address(addr) {
            return;
        }
        
        let offset = addr - self.base_addr;
        let object_index = offset / self.object_size;
        
        if object_index < self.capacity {
            let mask = 1 << object_index;
            if (self.allocated_bitmap & mask) != 0 {
                self.allocated_bitmap &= !mask;
                self.free_count += 1;
            }
        }
    }
    
    /// Check if this slab owns the given address
    fn owns_address(&self, addr: usize) -> bool {
        let slab_end = self.base_addr + self.capacity * self.object_size;
        addr >= self.base_addr && addr < slab_end
    }
    
    /// Check if this slab is completely empty
    fn is_empty(&self) -> bool {
        self.free_count == self.capacity
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_size_class_selection() {
        let allocator = SlabAllocator::new(0x1000, 1024 * 1024).unwrap();
        
        assert_eq!(allocator.find_size_class(1), Some(0));  // 8 bytes
        assert_eq!(allocator.find_size_class(8), Some(0));  // 8 bytes
        assert_eq!(allocator.find_size_class(9), Some(1));  // 16 bytes
        assert_eq!(allocator.find_size_class(32), Some(2)); // 32 bytes
        assert_eq!(allocator.find_size_class(4096), Some(9)); // 4096 bytes
        assert_eq!(allocator.find_size_class(8192), None);  // Too large
    }
} 