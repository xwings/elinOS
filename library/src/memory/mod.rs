pub mod regions;
pub mod hardware; 
pub mod layout;

// Re-export commonly used types and functions
pub use regions::{MemoryRegion, MemoryZone};
pub use hardware::{get_kernel_boundaries, get_stack_boundaries, detect_main_ram, get_fallback_ram, get_standard_mmio_regions, calculate_heap_start, validate_memory_layout};
pub use layout::*;