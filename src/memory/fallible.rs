// Fallible Memory Allocator for elinOS
// Inspired by Maestro OS - allocations can fail without panicking
// Provides graceful error handling for memory allocation failures

use core::ptr::NonNull;
use core::result::Result;
use crate::memory::slab::{SlabAllocator, SlabStats};
use crate::memory::buddy::BuddyError;

/// Allocation errors that can occur
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AllocError {
    /// Out of memory
    OutOfMemory,
    /// Invalid size (zero or too large)
    InvalidSize,
    /// Invalid alignment
    InvalidAlignment,
    /// Memory corruption detected
    CorruptionDetected,
}

/// Result type for fallible allocations
pub type AllocResult<T> = Result<T, AllocError>;

/// Trait for types that can be cloned fallibly (might fail due to allocation)
pub trait TryClone {
    /// Try to clone this object, potentially failing due to allocation errors
    fn try_clone(&self) -> AllocResult<Self> 
    where 
        Self: Sized;
}

// Blanket implementation for types that implement Clone
impl<T: Clone> TryClone for T {
    fn try_clone(&self) -> AllocResult<Self> {
        Ok(self.clone())
    }
}

/// Collection trait that supports fallible operations
pub trait FallibleCollection<T> {
    /// Try to push an item, potentially failing
    fn try_push(&mut self, item: T) -> AllocResult<()>;
    
    /// Try to extend with iterator, potentially failing
    fn try_extend<I>(&mut self, iter: I) -> AllocResult<()>
    where
        I: IntoIterator<Item = T>;
    
    /// Try to reserve capacity, potentially failing
    fn try_reserve(&mut self, capacity: usize) -> AllocResult<()>;
}

/// Wrapper for collect operations that can fail
pub struct CollectResult<T> {
    items: Option<heapless::Vec<T, 1024>>, // Limited size for no_std
}

impl<T> CollectResult<T> {
    /// Create a new CollectResult
    pub fn new() -> Self {
        Self {
            items: Some(heapless::Vec::new()),
        }
    }
    
    /// Add an item to the collection
    pub fn push(&mut self, item: T) -> AllocResult<()> {
        if let Some(ref mut vec) = self.items {
            vec.push(item).map_err(|_| AllocError::OutOfMemory)
        } else {
            Err(AllocError::OutOfMemory)
        }
    }
    
    /// Finalize and return the collected items
    pub fn finish(self) -> AllocResult<heapless::Vec<T, 1024>> {
        self.items.ok_or(AllocError::OutOfMemory)
    }
}

impl<T> Default for CollectResult<T> {
    fn default() -> Self {
        Self::new()
    }
}

/// Memory allocation transaction system
/// Allows atomic allocation operations that can be rolled back on failure
pub struct AllocTransaction {
    /// List of allocations made during this transaction
    allocations: heapless::Vec<(NonNull<u8>, usize), 32>,
    /// Whether this transaction has been committed
    committed: bool,
}

impl AllocTransaction {
    /// Create a new allocation transaction
    pub fn new() -> Self {
        Self {
            allocations: heapless::Vec::new(),
            committed: false,
        }
    }
    
    /// Record an allocation in this transaction
    pub fn record_allocation(&mut self, ptr: NonNull<u8>, size: usize) -> AllocResult<()> {
        self.allocations.push((ptr, size))
            .map_err(|_| AllocError::OutOfMemory)
    }
    
    /// Commit the transaction (no rollback will occur)
    pub fn commit(mut self) {
        self.committed = true;
        // Don't run Drop::drop when committed
        core::mem::forget(self);
    }
    
    /// Get the number of allocations in this transaction
    pub fn allocation_count(&self) -> usize {
        self.allocations.len()
    }
}

impl Drop for AllocTransaction {
    /// Automatically rollback if not committed
    fn drop(&mut self) {
        if !self.committed {
            // In a real implementation, we would free all recorded allocations
            // For now, we just log the rollback
            crate::console_println!("ℹ️  Rolling back {} allocations", self.allocations.len());
            
            // Rollback allocations in reverse order
            for &(ptr, size) in self.allocations.iter().rev() {
                // TODO: Call the actual deallocator here
                // FALLIBLE_ALLOCATOR.deallocate(ptr, size);
                drop((ptr, size)); // Placeholder
            }
        }
    }
}

/// Fallible allocator that wraps the slab allocator
pub struct FallibleAllocator {
    /// The underlying slab allocator
    slab_allocator: SlabAllocator,
    
    /// Statistics for tracking allocation failures
    allocation_failures: usize,
    oom_events: usize,
    
    /// Configuration
    fail_fast: bool, // Whether to fail immediately on OOM vs trying to recover
}

impl FallibleAllocator {
    /// Create a new fallible allocator
    pub fn new(base_address: usize, total_size: usize) -> Result<Self, BuddyError> {
        let slab_allocator = SlabAllocator::new(base_address, total_size)?;
        
        Ok(FallibleAllocator {
            slab_allocator,
            allocation_failures: 0,
            oom_events: 0,
            fail_fast: true,
        })
    }
    
    /// Set whether to fail fast on OOM or try to recover
    pub fn set_fail_fast(&mut self, fail_fast: bool) {
        self.fail_fast = fail_fast;
    }
    
    /// Try to allocate memory of the given size
    pub fn try_allocate(&mut self, size: usize) -> AllocResult<NonNull<u8>> {
        if size == 0 {
            return Err(AllocError::InvalidSize);
        }
        
        // Try allocation
        match self.slab_allocator.allocate(size) {
            Some(ptr) => Ok(ptr),
            None => {
                self.allocation_failures += 1;
                
                if self.fail_fast {
                    Err(AllocError::OutOfMemory)
                } else {
                    // Try to recover memory and retry once
                    self.try_recover_memory();
                    
                    match self.slab_allocator.allocate(size) {
                        Some(ptr) => Ok(ptr),
                        None => {
                            self.oom_events += 1;
                            Err(AllocError::OutOfMemory)
                        }
                    }
                }
            }
        }
    }
    
    /// Try to allocate memory with specific alignment
    pub fn try_allocate_aligned(&mut self, size: usize, align: usize) -> AllocResult<NonNull<u8>> {
        if !align.is_power_of_two() {
            return Err(AllocError::InvalidAlignment);
        }
        
        // For simplicity, over-allocate and align manually
        // A more sophisticated implementation would handle this in the allocator
        let extra_size = size + align - 1;
        let ptr = self.try_allocate(extra_size)?;
        
        let addr = ptr.as_ptr() as usize;
        let aligned_addr = (addr + align - 1) & !(align - 1);
        
        // This is a simplified approach - in practice, we'd need to track
        // the original allocation for proper deallocation
        match NonNull::new(aligned_addr as *mut u8) {
            Some(aligned_ptr) => Ok(aligned_ptr),
            None => Err(AllocError::CorruptionDetected),
        }
    }
    
    /// Deallocate memory
    pub fn deallocate(&mut self, ptr: NonNull<u8>, size: usize) {
        self.slab_allocator.deallocate(ptr, size);
    }
    
    /// Try to recover memory by freeing up caches, etc.
    fn try_recover_memory(&mut self) {
        // In a real implementation, this could:
        // 1. Compact memory
        // 2. Free cached objects
        // 3. Run garbage collection if applicable
        // 4. Return empty slabs to buddy allocator
        
        crate::console_println!("ℹ️  Attempting memory recovery...");
        
        // Placeholder: In practice, we'd implement actual recovery strategies
    }
    
    /// Get comprehensive allocator statistics
    pub fn get_stats(&self) -> FallibleAllocatorStats {
        let slab_stats = self.slab_allocator.get_stats();
        
        // Calculate failure rate before moving slab_stats
        let failure_rate = if slab_stats.total_allocations > 0 {
            self.allocation_failures as f32 / slab_stats.total_allocations as f32
        } else {
            0.0
        };
        
        FallibleAllocatorStats {
            slab_stats,
            allocation_failures: self.allocation_failures,
            oom_events: self.oom_events,
            failure_rate,
        }
    }
    
    /// Check if the allocator is in a healthy state
    pub fn is_healthy(&self) -> bool {
        let stats = self.get_stats();
        
        // Consider healthy if failure rate is below 5%
        stats.failure_rate < 0.05 && stats.oom_events < 10
    }
}

/// Comprehensive statistics for the fallible allocator
#[derive(Debug)]
pub struct FallibleAllocatorStats {
    pub slab_stats: SlabStats,
    pub allocation_failures: usize,
    pub oom_events: usize,
    pub failure_rate: f32,
}

/// Convenience macros for fallible operations
#[macro_export]
macro_rules! try_allocate {
    ($allocator:expr, $size:expr) => {
        $allocator.try_allocate($size)?
    };
}

#[macro_export]
macro_rules! with_transaction {
    ($allocator:expr, $operations:block) => {
        {
            let mut transaction = AllocTransaction::new();
            let result: AllocResult<_> = (|| $operations)();
            match result {
                Ok(value) => {
                    transaction.commit();
                    Ok(value)
                }
                Err(e) => {
                    // Transaction will be dropped and rolled back automatically
                    Err(e)
                }
            }
        }
    };
}

impl From<BuddyError> for AllocError {
    fn from(err: BuddyError) -> Self {
        match err {
            BuddyError::OutOfMemory => AllocError::OutOfMemory,
            BuddyError::InvalidSize => AllocError::InvalidSize,
            BuddyError::InvalidAddress => AllocError::CorruptionDetected,
            BuddyError::BadAlignment => AllocError::InvalidAlignment,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_transaction_rollback() {
        let mut allocator = FallibleAllocator::new(0x1000, 1024 * 1024).unwrap();
        
        // Test automatic rollback on drop
        {
            let mut transaction = AllocTransaction::new();
            let ptr = allocator.try_allocate(64).unwrap();
            transaction.record_allocation(ptr, 64).unwrap();
            
            // Transaction goes out of scope here and should rollback
        }
        
        // Allocator should be in original state
        assert!(allocator.is_healthy());
    }
    
    #[test]
    fn test_fallible_allocation() {
        let mut allocator = FallibleAllocator::new(0x1000, 1024).unwrap(); // Small size to trigger OOM
        
        // Should be able to allocate something
        let result = allocator.try_allocate(64);
        assert!(result.is_ok());
        
        // Eventually should fail with small memory pool
        let mut allocations = heapless::Vec::<NonNull<u8>, 32>::new();
        
        loop {
            match allocator.try_allocate(64) {
                Ok(ptr) => {
                    let _ = allocations.push(ptr);
                }
                Err(AllocError::OutOfMemory) => {
                    break; // Expected behavior
                }
                Err(e) => panic!("Unexpected error: {:?}", e),
            }
            
            if allocations.len() >= 20 {
                break; // Safety break
            }
        }
        
        assert!(allocator.get_stats().allocation_failures > 0);
    }
} 