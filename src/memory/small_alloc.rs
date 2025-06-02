// Small Object Allocator for elinKernel
// dlmalloc-style allocator for small objects < 4KB
// Implements size classes and bins for efficient allocation

use crate::memory::allocate_memory;
use spin::Mutex;
use crate::UART;
use core::fmt::Write;

/// Number of size classes for small objects
const NUM_SIZE_CLASSES: usize = 32;

/// Maximum size for small allocations (4KB threshold)
const MAX_SMALL_SIZE: usize = 4096;

/// Size class structure
#[derive(Debug, Clone, Copy)]
struct SizeClass {
    size: usize,
    chunk_size: usize, // Size of chunks we allocate for this class
}

/// Free chunk header (stored at the beginning of each free chunk)
#[derive(Debug, Clone, Copy)]
struct FreeChunk {
    next: Option<usize>, // Address of next free chunk
    size: usize,         // Size of this chunk
}

/// Bin for managing chunks of a specific size class
#[derive(Debug)]
struct Bin {
    free_list: Option<usize>, // Head of free list
    chunk_count: usize,       // Number of chunks in this bin
    total_allocated: usize,   // Total memory allocated for this bin
}

impl Bin {
    fn new() -> Self {
        Bin {
            free_list: None,
            chunk_count: 0,
            total_allocated: 0,
        }
    }
}

/// Small allocator for objects < 4KB
/// Uses size classes and bins like dlmalloc
pub struct SmallAllocator {
    /// Size classes for different allocation sizes
    size_classes: [SizeClass; NUM_SIZE_CLASSES],
    
    /// Bins for each size class
    bins: [Bin; NUM_SIZE_CLASSES],
    
    /// Base address for small allocations
    base_addr: usize,
    
    /// Current allocation pointer (bump allocator for new chunks)
    current_ptr: usize,
    
    /// End of available memory
    end_addr: usize,
    
    /// Statistics
    total_allocations: usize,
    total_deallocations: usize,
    bytes_allocated: usize,
}

impl SmallAllocator {
    pub fn new(base_addr: usize, size: usize) -> Self {
        let mut allocator = SmallAllocator {
            size_classes: [SizeClass { size: 0, chunk_size: 0 }; NUM_SIZE_CLASSES],
            bins: core::array::from_fn(|_| Bin::new()),
            base_addr,
            current_ptr: base_addr,
            end_addr: base_addr + size,
            total_allocations: 0,
            total_deallocations: 0,
            bytes_allocated: 0,
        };
        
        // Initialize size classes
        allocator.init_size_classes();
        
        allocator
    }
    
    /// Initialize size classes with powers of 2 up to MAX_SMALL_SIZE
    fn init_size_classes(&mut self) {
        let mut size = 8; // Start with 8-byte alignment
        
        for i in 0..NUM_SIZE_CLASSES {
            // Calculate chunk size (allocate multiple objects at once)
            let chunk_size = if size <= 64 {
                4096 // 4KB chunks for very small objects
            } else if size <= 512 {
                8192 // 8KB chunks for medium objects
            } else {
                16384 // 16KB chunks for larger small objects
            };
            
            self.size_classes[i] = SizeClass {
                size,
                chunk_size,
            };
            
            // Next size class (roughly geometric progression)
            if size < 128 {
                size += 8; // 8-byte increments for small sizes
            } else if size < 1024 {
                size += 64; // 64-byte increments for medium sizes
            } else {
                size += 256; // 256-byte increments for larger sizes
            }
            
            if size > MAX_SMALL_SIZE {
                size = MAX_SMALL_SIZE;
            }
        }
    }
    
    /// Find the appropriate size class for a given size
    fn find_size_class(&self, size: usize) -> Option<usize> {
        for (i, sc) in self.size_classes.iter().enumerate() {
            if sc.size >= size && sc.size > 0 {
                return Some(i);
            }
        }
        None
    }
    
    /// Allocate memory for a small object
    pub fn allocate(&mut self, size: usize) -> Option<*mut u8> {
        if size == 0 || size > MAX_SMALL_SIZE {
            return None;
        }
        
        // Find appropriate size class
        let size_class_idx = self.find_size_class(size)?;
        let size_class = self.size_classes[size_class_idx];
        
        // Try to get a chunk from the bin
        if let Some(chunk_addr) = self.bins[size_class_idx].free_list {
            // Remove from free list
            unsafe {
                let chunk_ptr = chunk_addr as *mut FreeChunk;
                let chunk = *chunk_ptr;
                self.bins[size_class_idx].free_list = chunk.next;
                self.bins[size_class_idx].chunk_count -= 1;
            }
            
            self.total_allocations += 1;
            self.bytes_allocated += size_class.size;
            
            return Some(chunk_addr as *mut u8);
        }
        
        // No free chunks available, allocate a new chunk
        self.allocate_new_chunk(size_class_idx)
    }
    
    /// Allocate a new chunk for the given size class
    fn allocate_new_chunk(&mut self, size_class_idx: usize) -> Option<*mut u8> {
        let size_class = self.size_classes[size_class_idx];
        let chunk_size = size_class.chunk_size;
        
        // Check if we have enough space
        if self.current_ptr + chunk_size > self.end_addr {
            return None;
        }
        
        // Allocate the chunk
        let chunk_start = self.current_ptr;
        self.current_ptr += chunk_size;
        
        // Split the chunk into individual objects
        let object_size = size_class.size;
        let num_objects = chunk_size / object_size;
        
        // Add all but the first object to the free list
        for i in 1..num_objects {
            let object_addr = chunk_start + (i * object_size);
            unsafe {
                let free_chunk = FreeChunk {
                    next: self.bins[size_class_idx].free_list,
                    size: object_size,
                };
                let chunk_ptr = object_addr as *mut FreeChunk;
                *chunk_ptr = free_chunk;
                self.bins[size_class_idx].free_list = Some(object_addr);
                self.bins[size_class_idx].chunk_count += 1;
            }
        }
        
        // Update bin statistics
        self.bins[size_class_idx].total_allocated += chunk_size;
        
        // Return the first object
        self.total_allocations += 1;
        self.bytes_allocated += object_size;
        
        Some(chunk_start as *mut u8)
    }
    
    /// Deallocate a small object
    pub fn deallocate(&mut self, ptr: *mut u8, size: usize) {
        if ptr.is_null() || size == 0 || size > MAX_SMALL_SIZE {
            return;
        }
        
        // Find the size class
        let size_class_idx = match self.find_size_class(size) {
            Some(idx) => idx,
            None => return,
        };
        
        let size_class = self.size_classes[size_class_idx];
        let addr = ptr as usize;
        
        // Verify the address is within our managed range
        if addr < self.base_addr || addr >= self.end_addr {
            return;
        }
        
        // Add to free list
        unsafe {
            let free_chunk = FreeChunk {
                next: self.bins[size_class_idx].free_list,
                size: size_class.size,
            };
            let chunk_ptr = addr as *mut FreeChunk;
            *chunk_ptr = free_chunk;
            self.bins[size_class_idx].free_list = Some(addr);
            self.bins[size_class_idx].chunk_count += 1;
        }
        
        self.total_deallocations += 1;
        self.bytes_allocated -= size_class.size;
    }
    
    /// Check if this allocator owns the given address
    pub fn owns_address(&self, addr: usize) -> bool {
        addr >= self.base_addr && addr < self.end_addr
    }
    
    /// Get allocation statistics
    pub fn get_stats(&self) -> SmallAllocatorStats {
        let mut free_chunks = 0;
        let mut total_bin_memory = 0;
        
        for bin in &self.bins {
            free_chunks += bin.chunk_count;
            total_bin_memory += bin.total_allocated;
        }
        
        SmallAllocatorStats {
            total_allocations: self.total_allocations,
            total_deallocations: self.total_deallocations,
            bytes_allocated: self.bytes_allocated,
            free_chunks,
            total_bin_memory,
            memory_usage: self.current_ptr - self.base_addr,
        }
    }
}

/// Statistics for the small allocator
#[derive(Debug)]
pub struct SmallAllocatorStats {
    pub total_allocations: usize,
    pub total_deallocations: usize,
    pub bytes_allocated: usize,
    pub free_chunks: usize,
    pub total_bin_memory: usize,
    pub memory_usage: usize,
} 