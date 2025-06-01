use core::fmt::Write;
use core::option::Option::{self, Some, None};
use core::writeln;
use spin::Mutex;
use crate::UART;
use crate::sbi;

// Memory region structure
#[derive(Debug, Clone, Copy)]
pub struct MemoryRegion {
    pub start: usize,
    pub size: usize,
    pub is_ram: bool,
}

// Memory manager structure
pub struct MemoryManager {
    regions: [MemoryRegion; 8],  // Support up to 8 memory regions
    region_count: usize,
    heap_start: usize,
    heap_end: usize,
    current_heap: usize,
}

impl MemoryManager {
    pub const fn new() -> Self {
        MemoryManager {
            regions: [MemoryRegion { start: 0, size: 0, is_ram: false }; 8],
            region_count: 0,
            heap_start: 0,
            heap_end: 0,
            current_heap: 0,
        }
    }

    // Initialize memory regions from OpenSBI
    pub fn init(&mut self) {
        {
            let mut uart = UART.lock();
            let _ = writeln!(uart, "Detecting memory regions through OpenSBI...");
        }
        
        // Get memory regions from OpenSBI
        let sbi_regions = sbi::get_memory_regions();
        
        // Convert SBI regions to our format
        self.region_count = sbi_regions.count;
        for i in 0..self.region_count {
            let sbi_region = &sbi_regions.regions[i];
            self.regions[i] = MemoryRegion {
                start: sbi_region.start,
                size: sbi_region.size,
                is_ram: (sbi_region.flags & 1) != 0,  // Check if region is RAM
            };
            
            // Print region information
            {
                let mut uart = UART.lock();
                let _ = writeln!(uart, "Region {}: 0x{:x} - 0x{:x} ({} MB) {}",
                    i,
                    sbi_region.start,
                    sbi_region.start + sbi_region.size,
                    sbi_region.size / (1024 * 1024),
                    if (sbi_region.flags & 1) != 0 { "RAM" } else { "MMIO" }
                );
            }
        }
        
        // Set up heap in the first RAM region
        if self.region_count > 0 {
            for region in &self.regions[..self.region_count] {
                if region.is_ram {
                    self.heap_start = region.start + 2 * 1024 * 1024;  // Leave 2MB for kernel
                    self.heap_end = region.start + region.size;
                    self.current_heap = self.heap_start;
                    
                    {
                        let mut uart = UART.lock();
                        let _ = writeln!(uart, "Heap configured: 0x{:x} - 0x{:x}",
                            self.heap_start,
                            self.heap_end
                        );
                    }
                    break;
                }
            }
        }
    }

    pub fn allocate(&mut self, size: usize) -> Option<usize> {
        let aligned_size = (size + 7) & !7;  // 8-byte alignment
        if self.current_heap + aligned_size > self.heap_end {
            None
        } else {
            let ptr = self.current_heap;
            self.current_heap += aligned_size;
            Some(ptr)
        }
    }

    pub fn get_memory_info(&self) -> &[MemoryRegion] {
        &self.regions[..self.region_count]
    }
}

// Global memory manager instance
pub static MEMORY_MANAGER: Mutex<MemoryManager> = Mutex::new(MemoryManager::new()); 