// Unified Dynamic Memory Manager for elinOS
// Consolidates all memory management functionality with dynamic allocation

// Removed unused imports: GlobalAlloc, Layout
use core::ptr::NonNull;
use spin::Mutex;
use heapless::Vec;
use crate::console_println;
use super::regions::MemoryRegion;
use super::hardware::{detect_main_ram, get_fallback_ram_for_system, get_kernel_boundaries, SystemType};

/// Memory allocation modes based on available system memory
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AllocationMode {
    /// Minimal mode for systems with < 16MB RAM - simple bump allocator only
    Minimal,
    /// Standard mode for 16MB-128MB RAM - buddy + simple allocators
    Standard,
    /// Advanced mode for > 128MB RAM - full multi-tier allocation
    Advanced,
}

/// Memory allocation result
pub type AllocResult<T> = Result<T, AllocationError>;

/// Memory allocation errors
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AllocationError {
    OutOfMemory,
    InvalidSize,
    InvalidAlignment,
    FragmentationError,
    SystemError,
}

/// Buffer usage types for optimal sizing
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BufferUsage {
    SectorIO,   // For disk sector operations
    FileRead,   // For reading files
    Command,    // For command line input
    Network,    // For network operations
}

/// Memory usage statistics
#[derive(Debug, Clone)]
pub struct MemoryStats {
    pub detected_ram_size: usize,
    pub allocated_bytes: usize,
    pub allocation_count: usize,
    pub allocator_mode: AllocationMode,
    pub heap_size: usize,
    pub heap_used: usize,
    pub regions_detected: usize,
}

/// Configuration for dynamic memory allocation
#[derive(Debug, Clone)]
pub struct MemoryConfig {
    /// Total available RAM
    pub total_ram: usize,
    /// Kernel boundaries
    pub kernel_start: usize,
    pub kernel_end: usize,
    /// Allocation mode based on available memory
    pub mode: AllocationMode,
    /// Dynamically calculated heap size (percentage of total RAM)
    pub heap_size: usize,
    /// Dynamically calculated buddy allocator size
    pub buddy_heap_size: usize,
    /// Small object allocator size
    pub small_heap_size: usize,
    /// Device memory size for DMA operations
    pub device_memory_size: usize,
    /// Maximum allocation size for this system
    pub max_allocation_size: usize,
}

impl MemoryConfig {
    /// Create dynamic memory configuration based on detected hardware
    pub fn detect() -> Self {
        console_println!("[i] Detecting memory configuration...");
        
        // Detect available RAM with smart fallback
        let memory_region = detect_main_ram().unwrap_or_else(|| {
            // Try to detect system type for better fallback
            console_println!("[!] RAM detection failed, using intelligent fallback");
            get_fallback_ram_for_system(SystemType::QEMU) // Default to QEMU for testing
        });
        let total_ram = memory_region.size;
        console_println!("[i] Total RAM detected: {} MB", total_ram / (1024 * 1024));
        
        // Get kernel boundaries
        let (kernel_start, kernel_end, _kernel_size) = get_kernel_boundaries();
        
        // Determine allocation mode based on available RAM
        let mode = if total_ram < 16 * 1024 * 1024 {
            AllocationMode::Minimal
        } else if total_ram < 128 * 1024 * 1024 {
            AllocationMode::Standard  
        } else {
            AllocationMode::Advanced
        };
        
        // Calculate dynamic sizes based on total RAM and mode
        let (heap_size, buddy_heap_size, small_heap_size, device_memory_size) = 
            Self::calculate_dynamic_sizes(total_ram, mode);
        
        let max_allocation_size = match mode {
            AllocationMode::Minimal => heap_size / 2,      // Conservative for minimal systems
            AllocationMode::Standard => buddy_heap_size,    // Limited by buddy allocator
            AllocationMode::Advanced => buddy_heap_size,    // Can use full buddy capacity
        };
        
        console_println!("[i] Memory configuration:");
        console_println!("    Mode: {:?}", mode);
        console_println!("    Heap: {} KB", heap_size / 1024);
        console_println!("    Buddy heap: {} KB", buddy_heap_size / 1024);
        console_println!("    Small heap: {} KB", small_heap_size / 1024);
        console_println!("    Device memory: {} KB", device_memory_size / 1024);
        console_println!("    Max allocation: {} KB", max_allocation_size / 1024);
        
        Self {
            total_ram,
            kernel_start,
            kernel_end,
            mode,
            heap_size,
            buddy_heap_size,
            small_heap_size,
            device_memory_size,
            max_allocation_size,
        }
    }
    
    /// Calculate optimal allocation sizes based on total RAM and allocation mode
    fn calculate_dynamic_sizes(total_ram: usize, mode: AllocationMode) -> (usize, usize, usize, usize) {
        match mode {
            AllocationMode::Minimal => {
                // For systems with < 16MB RAM - be very conservative
                let heap_size = (total_ram * 15 / 100).max(64 * 1024);  // 15% of RAM, min 64KB
                let buddy_heap_size = 0; // No buddy allocator
                let small_heap_size = heap_size / 4; // 25% for small objects
                let device_memory_size = (total_ram * 5 / 100).max(32 * 1024); // 5% for devices, min 32KB
                (heap_size, buddy_heap_size, small_heap_size, device_memory_size)
            }
            AllocationMode::Standard => {
                // For systems with 16-128MB RAM - balanced approach
                let heap_size = (total_ram * 25 / 100).max(2 * 1024 * 1024);  // 25% of RAM, min 2MB
                let buddy_heap_size = (total_ram * 15 / 100).max(1 * 1024 * 1024); // 15% for buddy, min 1MB
                let small_heap_size = (total_ram * 5 / 100).max(256 * 1024); // 5% for small objects, min 256KB
                let device_memory_size = (total_ram * 10 / 100).max(1 * 1024 * 1024); // 10% for devices, min 1MB
                (heap_size, buddy_heap_size, small_heap_size, device_memory_size)
            }
            AllocationMode::Advanced => {
                // For systems with > 128MB RAM - aggressive allocation
                let heap_size = (total_ram * 40 / 100).max(8 * 1024 * 1024);  // 40% of RAM, min 8MB
                let buddy_heap_size = (total_ram * 25 / 100).max(4 * 1024 * 1024); // 25% for buddy, min 4MB
                let small_heap_size = (total_ram * 8 / 100).max(1 * 1024 * 1024); // 8% for small objects, min 1MB
                let device_memory_size = (total_ram * 15 / 100).max(2 * 1024 * 1024); // 15% for devices, min 2MB
                (heap_size, buddy_heap_size, small_heap_size, device_memory_size)
            }
        }
    }
}

/// Unified memory manager that coordinates all allocation strategies
pub struct UnifiedMemoryManager {
    config: MemoryConfig,
    
    // Memory regions
    regions: Vec<MemoryRegion, 16>,
    
    // Allocation tracking
    total_allocated: usize,
    allocation_count: usize,
    
    // Memory layout
    heap_start: usize,
    heap_end: usize,
    buddy_start: usize,
    buddy_end: usize,
    small_start: usize,
    small_end: usize,
    device_start: usize,
    device_end: usize,
    
    // Simple bump allocator for minimal systems
    bump_position: usize,
    
    // Buddy allocator state (if enabled)
    buddy_free_lists: Option<Vec<usize, 32>>, // Support up to 32 free lists (2^32 max block size)
    buddy_bitmap: Option<Vec<u8, 65536>>,     // Dynamic bitmap size - up to 512KB
    
    // Small object allocator state
    small_bins: Option<Vec<usize, 64>>,      // Support 64 size classes
    
    // Free range tracking
    free_ranges: Vec<(usize, usize), 256>,   // Track free memory ranges
}

impl UnifiedMemoryManager {
    /// Create new memory manager with detected configuration
    pub fn new() -> Self {
        let config = MemoryConfig::detect();
        
        Self {
            config,
            regions: Vec::new(),
            total_allocated: 0,
            allocation_count: 0,
            heap_start: 0,
            heap_end: 0,
            buddy_start: 0,
            buddy_end: 0,
            small_start: 0,
            small_end: 0,
            device_start: 0,
            device_end: 0,
            bump_position: 0,
            buddy_free_lists: None,
            buddy_bitmap: None,
            small_bins: None,
            free_ranges: Vec::new(),
        }
    }
    
    /// Initialize the memory manager with proper memory layout
    pub fn initialize(&mut self) -> AllocResult<()> {
        console_println!("[i] Initializing unified memory manager...");
        
        // Calculate memory layout based on configuration
        self.calculate_memory_layout()?;
        
        // Initialize allocators based on mode
        match self.config.mode {
            AllocationMode::Minimal => {
                self.init_minimal_allocator()?;
            }
            AllocationMode::Standard => {
                self.init_minimal_allocator()?;
                self.init_buddy_allocator()?;
            }
            AllocationMode::Advanced => {
                self.init_minimal_allocator()?;
                self.init_buddy_allocator()?;
                self.init_small_allocator()?;
            }
        }
        
        console_println!("[o] Unified memory manager initialized successfully!");
        self.display_layout();
        Ok(())
    }
    
    /// Calculate optimal memory layout based on configuration
    fn calculate_memory_layout(&mut self) -> AllocResult<()> {
        // Start heap after kernel with proper alignment
        let heap_start = (self.config.kernel_end + 4096) & !4095; // 4KB aligned
        
        self.heap_start = heap_start;
        self.heap_end = heap_start + self.config.heap_size;
        
        // Buddy allocator region (if enabled)
        if self.config.buddy_heap_size > 0 {
            self.buddy_start = self.heap_end;
            self.buddy_end = self.buddy_start + self.config.buddy_heap_size;
        }
        
        // Small allocator region (if enabled)
        if self.config.small_heap_size > 0 {
            self.small_start = if self.buddy_end > 0 { self.buddy_end } else { self.heap_end };
            self.small_end = self.small_start + self.config.small_heap_size;
        }
        
        // Device memory region
        self.device_start = if self.small_end > 0 { self.small_end } else if self.buddy_end > 0 { self.buddy_end } else { self.heap_end };
        self.device_end = self.device_start + self.config.device_memory_size;
        
        // Validate layout doesn't exceed available memory
        let total_usage = self.device_end - self.config.kernel_start;
        if total_usage > self.config.total_ram {
            console_println!("[x] Memory layout exceeds available RAM!");
            return Err(AllocationError::SystemError);
        }
        
        // Initialize free ranges with the main heap
        let _ = self.free_ranges.push((self.heap_start, self.heap_end));
        
        Ok(())
    }
    
    /// Initialize minimal bump allocator
    fn init_minimal_allocator(&mut self) -> AllocResult<()> {
        self.bump_position = self.heap_start;
        console_println!("[o] Minimal bump allocator initialized: 0x{:x}-0x{:x}", 
                         self.heap_start, self.heap_end);
        Ok(())
    }
    
    /// Initialize buddy allocator with dynamic bitmap
    fn init_buddy_allocator(&mut self) -> AllocResult<()> {
        if self.config.buddy_heap_size == 0 {
            return Ok(());
        }
        
        // Calculate required bitmap size
        let min_block_size = 64; // 64-byte minimum blocks
        let total_blocks = self.config.buddy_heap_size / min_block_size;
        let bitmap_size = (total_blocks + 7) / 8; // Round up to byte boundary
        
        if bitmap_size > 65536 {
            console_println!("[x] Buddy allocator bitmap too large: {} bytes", bitmap_size);
            return Err(AllocationError::SystemError);
        }
        
        // Initialize buddy allocator data structures
        self.buddy_free_lists = Some(Vec::new());
        self.buddy_bitmap = Some(Vec::new());
        
        console_println!("[o] Buddy allocator initialized: 0x{:x}-0x{:x} (bitmap: {} bytes)", 
                         self.buddy_start, self.buddy_end, bitmap_size);
        Ok(())
    }
    
    /// Initialize small object allocator
    fn init_small_allocator(&mut self) -> AllocResult<()> {
        if self.config.small_heap_size == 0 {
            return Ok(());
        }
        
        self.small_bins = Some(Vec::new());
        console_println!("[o] Small object allocator initialized: 0x{:x}-0x{:x}", 
                         self.small_start, self.small_end);
        Ok(())
    }
    
    /// Allocate memory using the most appropriate allocator
    pub fn allocate(&mut self, size: usize, align: usize) -> AllocResult<NonNull<u8>> {
        if size == 0 {
            return Err(AllocationError::InvalidSize);
        }
        
        if size > self.config.max_allocation_size {
            return Err(AllocationError::InvalidSize);
        }
        
        // Choose allocator based on size and mode
        match self.config.mode {
            AllocationMode::Minimal => self.allocate_minimal(size, align),
            AllocationMode::Standard => {
                if size <= 4096 && self.small_bins.is_some() {
                    self.allocate_small(size, align)
                } else if size <= self.config.buddy_heap_size / 4 && self.buddy_free_lists.is_some() {
                    self.allocate_buddy(size, align)
                } else {
                    self.allocate_minimal(size, align)
                }
            }
            AllocationMode::Advanced => {
                if size <= 4096 && self.small_bins.is_some() {
                    self.allocate_small(size, align)
                } else if self.buddy_free_lists.is_some() {
                    self.allocate_buddy(size, align)
                } else {
                    self.allocate_minimal(size, align)
                }
            }
        }
    }
    
    /// Minimal bump allocator implementation
    fn allocate_minimal(&mut self, size: usize, align: usize) -> AllocResult<NonNull<u8>> {
        // Align current position
        let aligned_pos = (self.bump_position + align - 1) & !(align - 1);
        let end_pos = aligned_pos + size;
        
        if end_pos > self.heap_end {
            return Err(AllocationError::OutOfMemory);
        }
        
        self.bump_position = end_pos;
        self.total_allocated += size;
        self.allocation_count += 1;
        
        // Update free ranges
        self.update_free_ranges_after_allocation(aligned_pos, size);
        
        unsafe {
            Ok(NonNull::new_unchecked(aligned_pos as *mut u8))
        }
    }
    
    /// Buddy allocator implementation (simplified)
    fn allocate_buddy(&mut self, size: usize, align: usize) -> AllocResult<NonNull<u8>> {
        // For now, fall back to minimal allocator
        // TODO: Implement full buddy allocator
        self.allocate_minimal(size, align)
    }
    
    /// Small object allocator implementation
    fn allocate_small(&mut self, size: usize, align: usize) -> AllocResult<NonNull<u8>> {
        // For now, fall back to minimal allocator
        // TODO: Implement full small object allocator
        self.allocate_minimal(size, align)
    }
    
    /// Deallocate memory and update free ranges
    pub fn deallocate(&mut self, ptr: NonNull<u8>, size: usize) {
        let addr = ptr.as_ptr() as usize;
        
        // Update statistics
        self.total_allocated = self.total_allocated.saturating_sub(size);
        
        // Add to free ranges
        self.add_free_range(addr, size);
        
        // TODO: Implement proper deallocation for buddy and small allocators
    }
    
    /// Update free ranges after allocation
    fn update_free_ranges_after_allocation(&mut self, addr: usize, size: usize) {
        // Find and split the free range that contains this allocation
        for i in 0..self.free_ranges.len() {
            let (start, end) = self.free_ranges[i];
            if addr >= start && addr + size <= end {
                // Remove this range
                self.free_ranges.swap_remove(i);
                
                // Add remaining free pieces
                if addr > start {
                    let _ = self.free_ranges.push((start, addr));
                }
                if addr + size < end {
                    let _ = self.free_ranges.push((addr + size, end));
                }
                break;
            }
        }
    }
    
    /// Add a free range and coalesce adjacent ranges
    fn add_free_range(&mut self, addr: usize, size: usize) {
        let end = addr + size;
        
        // Find adjacent ranges to coalesce
        let mut to_remove = Vec::<usize, 16>::new();
        let mut new_start = addr;
        let mut new_end = end;
        
        for (i, &(range_start, range_end)) in self.free_ranges.iter().enumerate() {
            if range_end == addr {
                // This range ends where new range starts
                new_start = range_start;
                let _ = to_remove.push(i);
            } else if range_start == end {
                // This range starts where new range ends
                new_end = range_end;
                let _ = to_remove.push(i);
            }
        }
        
        // Remove coalesced ranges (in reverse order to maintain indices)
        for &i in to_remove.iter().rev() {
            self.free_ranges.swap_remove(i);
        }
        
        // Add the new coalesced range
        let _ = self.free_ranges.push((new_start, new_end));
    }
    
    /// Get total free memory
    pub fn get_free_memory(&self) -> usize {
        self.free_ranges.iter().map(|(start, end)| end - start).sum()
    }
    
    /// Check if a specific range is free
    pub fn is_range_free(&self, addr: usize, size: usize) -> bool {
        let end = addr + size;
        
        for &(start, range_end) in &self.free_ranges {
            if addr >= start && end <= range_end {
                return true;
            }
        }
        false
    }
    
    /// Get maximum file size this system can handle
    pub fn get_max_file_size(&self) -> usize {
        // Scale with available memory and allocation mode
        match self.config.mode {
            AllocationMode::Minimal => 64 * 1024,      // 64KB for minimal
            AllocationMode::Standard => 1024 * 1024,   // 1MB for standard  
            AllocationMode::Advanced => 16 * 1024 * 1024, // 16MB for advanced
        }
    }

    /// Get heap usage information (used, total, available)
    pub fn get_heap_usage(&self) -> (usize, usize, usize) {
        let used = self.total_allocated;
        let total = self.config.heap_size;
        let available = total.saturating_sub(used);
        (used, total, available)
    }

    /// Reset heap for testing (dangerous - only for debugging)
    pub fn reset_heap_for_testing(&mut self) {
        self.total_allocated = 0;
        self.allocation_count = 0;
        self.bump_position = self.heap_start;
        // Clear free ranges and add the main heap back
        self.free_ranges.clear();
        let _ = self.free_ranges.push((self.heap_start, self.heap_end));
    }

    /// Get memory statistics
    pub fn get_memory_stats(&self) -> MemoryStats {
        MemoryStats {
            detected_ram_size: self.config.total_ram,
            allocated_bytes: self.total_allocated,
            allocation_count: self.allocation_count,
            allocator_mode: self.config.mode,
            heap_size: self.config.heap_size,
            heap_used: self.total_allocated,
            regions_detected: self.regions.len(),
        }
    }

    /// Get optimal buffer size for different use cases
    pub fn get_optimal_buffer_size(&self, usage: BufferUsage) -> usize {
        match usage {
            BufferUsage::SectorIO => 512, // Always 512 for disk sectors
            BufferUsage::FileRead => {
                // Scale with available memory
                match self.config.mode {
                    AllocationMode::Minimal => 4 * 1024,      // 4KB for minimal systems
                    AllocationMode::Standard => 16 * 1024,    // 16KB for standard
                    AllocationMode::Advanced => 64 * 1024,    // 64KB for advanced
                }
            }
            BufferUsage::Command => {
                match self.config.mode {
                    AllocationMode::Minimal => 128,           // 128 bytes for minimal
                    AllocationMode::Standard => 512,          // 512 bytes for standard
                    AllocationMode::Advanced => 1024,         // 1KB for advanced
                }
            }
            BufferUsage::Network => {
                match self.config.mode {
                    AllocationMode::Minimal => 1500,          // MTU size for minimal
                    AllocationMode::Standard => 4096,         // 4KB for standard
                    AllocationMode::Advanced => 8192,         // 8KB for advanced
                }
            }
        }
    }

    /// Display current memory layout and statistics
    pub fn display_layout(&self) {
        console_println!("=== Unified Memory Manager Layout ===");
        console_println!("Configuration: {:?}", self.config.mode);
        console_println!("Total RAM: {} MB", self.config.total_ram / (1024 * 1024));
        console_println!();
        console_println!("Memory Regions:");
        console_println!("  Kernel:  0x{:08x} - 0x{:08x} ({} KB)",
                         self.config.kernel_start, self.config.kernel_end,
                         (self.config.kernel_end - self.config.kernel_start) / 1024);
        console_println!("  Heap:    0x{:08x} - 0x{:08x} ({} KB)",
                         self.heap_start, self.heap_end,
                         (self.heap_end - self.heap_start) / 1024);
        
        if self.buddy_end > self.buddy_start {
            console_println!("  Buddy:   0x{:08x} - 0x{:08x} ({} KB)",
                             self.buddy_start, self.buddy_end,
                             (self.buddy_end - self.buddy_start) / 1024);
        }
        
        if self.small_end > self.small_start {
            console_println!("  Small:   0x{:08x} - 0x{:08x} ({} KB)",
                             self.small_start, self.small_end,
                             (self.small_end - self.small_start) / 1024);
        }
        
        console_println!("  Device:  0x{:08x} - 0x{:08x} ({} KB)",
                         self.device_start, self.device_end,
                         (self.device_end - self.device_start) / 1024);
        console_println!();
        console_println!("Statistics:");
        console_println!("  Allocated: {} KB ({} allocations)",
                         self.total_allocated / 1024, self.allocation_count);
        console_println!("  Free: {} KB ({} ranges)",
                         self.get_free_memory() / 1024, self.free_ranges.len());
    }
}

/// Global unified memory manager
pub static UNIFIED_MEMORY_MANAGER: Mutex<Option<UnifiedMemoryManager>> = Mutex::new(None);

/// Initialize the global memory manager
pub fn init_unified_memory_manager() -> AllocResult<()> {
    let mut manager = UnifiedMemoryManager::new();
    manager.initialize()?;
    
    *UNIFIED_MEMORY_MANAGER.lock() = Some(manager);
    Ok(())
}

/// Get a reference to the global memory manager
pub fn with_memory_manager<F, R>(f: F) -> R
where
    F: FnOnce(&mut UnifiedMemoryManager) -> R,
{
    let mut guard = UNIFIED_MEMORY_MANAGER.lock();
    let manager = guard.as_mut().expect("Memory manager not initialized");
    f(manager)
}

/// Allocate memory using the global manager
pub fn allocate_memory(size: usize, align: usize) -> AllocResult<NonNull<u8>> {
    with_memory_manager(|mgr| mgr.allocate(size, align))
}

/// Deallocate memory using the global manager
pub fn deallocate_memory(ptr: NonNull<u8>, size: usize) {
    with_memory_manager(|mgr| mgr.deallocate(ptr, size))
}

/// Check if a memory range is free
pub fn is_memory_range_free(addr: usize, size: usize) -> bool {
    with_memory_manager(|mgr| mgr.is_range_free(addr, size))
}

/// Get total free memory
pub fn get_total_free_memory() -> usize {
    with_memory_manager(|mgr| mgr.get_free_memory())
}

/// Display memory layout
pub fn display_memory_layout() {
    with_memory_manager(|mgr| mgr.display_layout())
}

/// Get optimal buffer size using global manager
pub fn get_optimal_buffer_size(usage: BufferUsage) -> usize {
    with_memory_manager(|mgr| mgr.get_optimal_buffer_size(usage))
}

/// Get memory statistics using global manager
pub fn get_memory_stats() -> MemoryStats {
    with_memory_manager(|mgr| mgr.get_memory_stats())
}

/// Get maximum file size using global manager
pub fn get_max_file_size() -> usize {
    with_memory_manager(|mgr| mgr.get_max_file_size())
}

/// Get heap usage using global manager
pub fn get_heap_usage() -> (usize, usize, usize) {
    with_memory_manager(|mgr| mgr.get_heap_usage())
}

/// Reset heap for testing using global manager
pub fn reset_heap_for_testing() {
    with_memory_manager(|mgr| mgr.reset_heap_for_testing())
}