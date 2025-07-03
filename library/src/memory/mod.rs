pub mod regions;
pub mod hardware; 
pub mod layout;
pub mod manager;

// Re-export commonly used types and functions
pub use regions::{MemoryRegion, MemoryZone};
pub use hardware::{get_kernel_boundaries, get_stack_boundaries, detect_main_ram, get_fallback_ram, get_standard_mmio_regions, calculate_heap_start, validate_memory_layout};
pub use layout::*;

// Re-export the unified memory manager
pub use manager::{
    UnifiedMemoryManager, MemoryConfig, AllocationMode, AllocationError, AllocResult, BufferUsage, MemoryStats,
    init_unified_memory_manager, with_memory_manager, allocate_memory, deallocate_memory,
    is_memory_range_free, get_total_free_memory, display_memory_layout, get_optimal_buffer_size, get_memory_stats,
    get_max_file_size, get_heap_usage, reset_heap_for_testing
};