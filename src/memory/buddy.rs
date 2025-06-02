// Buddy Allocator Implementation for elinKernel
// Based on MIT-licensed references and public domain algorithms
// Inspired by evanw/buddy-malloc and jjyr/buddy-alloc designs

use core::fmt;
use heapless::Vec;

/// Maximum order supported by the buddy allocator
/// This gives us block sizes from 2^0 to 2^MAX_ORDER bytes
pub const MAX_ORDER: usize = 20; // Up to 1MB blocks

/// Minimum block size (2^0 = 1 byte)
pub const MIN_BLOCK_SIZE: usize = 1;

/// Buddy allocator error types
#[derive(Debug)]
pub enum BuddyError {
    InvalidSize,
    OutOfMemory,
    InvalidAddress,
    BadAlignment,
}

/// A free block in the buddy allocator
#[derive(Debug, Clone, Copy)]
struct FreeBlock {
    address: usize,
    next: Option<usize>, // Address of next free block (intrusive linked list)
}

/// Buddy allocator using binary tree structure
/// Based on the algorithm described in MIT-licensed references
pub struct BuddyAllocator {
    /// Base address of the memory region we're managing
    base_address: usize,
    
    /// Total size of the memory region
    total_size: usize,
    
    /// Maximum order that fits in our memory region
    max_order: usize,
    
    /// Free lists for each order
    /// Each list contains free blocks of size 2^order
    free_lists: [Option<usize>; MAX_ORDER + 1],
    
    /// Bitmap to track split status of blocks
    /// Based on the "single bit per node" trick from Linux kernel
    /// Bit = 0: both buddies are free or both are allocated
    /// Bit = 1: exactly one buddy is allocated
    split_bitmap: Vec<u8, 4096>, // Support up to 32KB bitmap
}

impl BuddyAllocator {
    /// Create a new buddy allocator
    /// 
    /// # Arguments
    /// * `base_address` - Starting address of memory region to manage
    /// * `total_size` - Size of memory region (will be rounded down to largest power of 2)
    pub fn new(base_address: usize, total_size: usize) -> Result<Self, BuddyError> {
        if total_size == 0 {
            return Err(BuddyError::InvalidSize);
        }
        
        // Find the largest power of 2 that fits in our memory region
        let max_order = Self::log2_floor(total_size);
        let usable_size = 1 << max_order;
        
        // Calculate bitmap size needed
        let bitmap_bits = usable_size / MIN_BLOCK_SIZE;
        let bitmap_bytes = (bitmap_bits + 7) / 8;
        
        if bitmap_bytes > 4096 {
            return Err(BuddyError::InvalidSize);
        }
        
        let mut allocator = BuddyAllocator {
            base_address,
            total_size: usable_size,
            max_order,
            free_lists: [None; MAX_ORDER + 1],
            split_bitmap: Vec::new(),
        };
        
        // Initialize bitmap with zeros
        for _ in 0..bitmap_bytes {
            allocator.split_bitmap.push(0).map_err(|_| BuddyError::OutOfMemory)?;
        }
        
        // Initially, we have one free block of maximum size
        allocator.free_lists[max_order] = Some(base_address);
        
        // Write the initial free block header
        unsafe {
            let block_ptr = base_address as *mut FreeBlock;
            *block_ptr = FreeBlock {
                address: base_address,
                next: None,
            };
        }
        
        Ok(allocator)
    }
    
    /// Allocate a block of at least `size` bytes
    pub fn allocate(&mut self, size: usize) -> Option<usize> {
        if size == 0 {
            return None;
        }
        
        // Find the order needed for this size
        let order = Self::size_to_order(size);
        
        // Find a free block of sufficient size
        let block_addr = self.allocate_block(order)?;
        
        Some(block_addr)
    }
    
    /// Deallocate a block at the given address with the given size
    pub fn deallocate(&mut self, address: usize, size: usize) {
        if size == 0 || !self.owns_address(address) {
            return;
        }
        
        let order = Self::size_to_order(size);
        self.deallocate_block(address, order);
    }
    
    /// Check if this allocator owns the given address
    pub fn owns_address(&self, address: usize) -> bool {
        address >= self.base_address && address < self.base_address + self.total_size
    }
    
    /// Allocate a block of the specified order
    fn allocate_block(&mut self, order: usize) -> Option<usize> {
        if order > self.max_order {
            return None;
        }
        
        // Try to find a free block of the requested order
        if let Some(block_addr) = self.free_lists[order] {
            // Remove the block from the free list
            self.remove_from_free_list(order, block_addr);
            self.mark_allocated(block_addr, order);
            return Some(block_addr);
        }
        
        // No block of the requested order available, try to split a larger block
        if let Some(larger_block) = self.allocate_block(order + 1) {
            let block_size = 1 << order;
            let buddy_addr = larger_block + block_size;
            
            // Add the buddy to the free list for this order
            self.add_to_free_list(order, buddy_addr);
            
            // Return the first half
            return Some(larger_block);
        }
        
        None
    }
    
    /// Deallocate a block and try to merge with its buddy
    fn deallocate_block(&mut self, address: usize, order: usize) {
        if order >= MAX_ORDER {
            return;
        }
        
        self.mark_free(address, order);
        
        // Try to merge with buddy
        let buddy_addr = self.get_buddy_address(address, order);
        
        if self.is_free(buddy_addr, order) && self.owns_address(buddy_addr) {
            // Both blocks are free, merge them
            self.remove_from_free_list(order, buddy_addr);
            
            // The merged block starts at the lower address
            let merged_addr = if address < buddy_addr { address } else { buddy_addr };
            
            // Recursively try to merge at the next level
            self.deallocate_block(merged_addr, order + 1);
        } else {
            // Can't merge, add to free list
            self.add_to_free_list(order, address);
        }
    }
    
    /// Add a block to the free list for the given order
    fn add_to_free_list(&mut self, order: usize, address: usize) {
        unsafe {
            let block_ptr = address as *mut FreeBlock;
            let old_head = self.free_lists[order];
            
            *block_ptr = FreeBlock {
                address,
                next: old_head,
            };
            
            self.free_lists[order] = Some(address);
        }
    }
    
    /// Remove a block from the free list for the given order
    fn remove_from_free_list(&mut self, order: usize, address: usize) {
        if self.free_lists[order] == Some(address) {
            // Remove from head of list
            unsafe {
                let block_ptr = address as *const FreeBlock;
                self.free_lists[order] = (*block_ptr).next;
            }
        } else {
            // Search through the list to find and remove the block
            let mut current = self.free_lists[order];
            
            while let Some(current_addr) = current {
                unsafe {
                    let current_ptr = current_addr as *mut FreeBlock;
                    let current_block = *current_ptr;
                    
                    if current_block.next == Some(address) {
                        let target_ptr = address as *const FreeBlock;
                        let target_block = *target_ptr;
                        (*current_ptr).next = target_block.next;
                        break;
                    }
                    
                    current = current_block.next;
                }
            }
        }
    }
    
    /// Get the address of the buddy for a given block
    fn get_buddy_address(&self, address: usize, order: usize) -> usize {
        let relative_addr = address - self.base_address;
        let block_size = 1 << order;
        let buddy_relative = relative_addr ^ block_size;
        self.base_address + buddy_relative
    }
    
    /// Mark a block as allocated in the bitmap
    fn mark_allocated(&mut self, address: usize, order: usize) {
        let bit_index = self.get_bitmap_index(address, order);
        self.flip_bit(bit_index);
    }
    
    /// Mark a block as free in the bitmap
    fn mark_free(&mut self, address: usize, order: usize) {
        let bit_index = self.get_bitmap_index(address, order);
        self.flip_bit(bit_index);
    }
    
    /// Check if a block is free (using buddy status)
    fn is_free(&self, address: usize, order: usize) -> bool {
        let bit_index = self.get_bitmap_index(address, order);
        !self.get_bit(bit_index) // Bit = 0 means both buddies have same status
    }
    
    /// Get bitmap index for a block
    fn get_bitmap_index(&self, address: usize, order: usize) -> usize {
        let relative_addr = address - self.base_address;
        let block_size = 1 << order;
        relative_addr / block_size / 2
    }
    
    /// Flip a bit in the bitmap
    fn flip_bit(&mut self, bit_index: usize) {
        let byte_index = bit_index / 8;
        let bit_offset = bit_index % 8;
        
        if byte_index < self.split_bitmap.len() {
            self.split_bitmap[byte_index] ^= 1 << bit_offset;
        }
    }
    
    /// Get a bit from the bitmap
    fn get_bit(&self, bit_index: usize) -> bool {
        let byte_index = bit_index / 8;
        let bit_offset = bit_index % 8;
        
        if byte_index < self.split_bitmap.len() {
            (self.split_bitmap[byte_index] >> bit_offset) & 1 != 0
        } else {
            false
        }
    }
    
    /// Convert size to order (ceiling log2)
    fn size_to_order(size: usize) -> usize {
        if size <= 1 {
            return 0;
        }
        
        let mut order = 0;
        let mut power = 1;
        
        while power < size {
            power <<= 1;
            order += 1;
        }
        
        order
    }
    
    /// Calculate floor log2 of a number
    fn log2_floor(n: usize) -> usize {
        if n == 0 {
            return 0;
        }
        
        let mut result = 0;
        let mut temp = n;
        
        while temp > 1 {
            temp >>= 1;
            result += 1;
        }
        
        result
    }
}

impl fmt::Debug for BuddyAllocator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "BuddyAllocator {{ base: 0x{:x}, size: 0x{:x}, max_order: {} }}", 
               self.base_address, self.total_size, self.max_order)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_size_to_order() {
        assert_eq!(BuddyAllocator::size_to_order(1), 0);
        assert_eq!(BuddyAllocator::size_to_order(2), 1);
        assert_eq!(BuddyAllocator::size_to_order(3), 2);
        assert_eq!(BuddyAllocator::size_to_order(4), 2);
        assert_eq!(BuddyAllocator::size_to_order(5), 3);
        assert_eq!(BuddyAllocator::size_to_order(1024), 10);
    }
    
    #[test]
    fn test_log2_floor() {
        assert_eq!(BuddyAllocator::log2_floor(1), 0);
        assert_eq!(BuddyAllocator::log2_floor(2), 1);
        assert_eq!(BuddyAllocator::log2_floor(3), 1);
        assert_eq!(BuddyAllocator::log2_floor(4), 2);
        assert_eq!(BuddyAllocator::log2_floor(1024), 10);
    }
} 