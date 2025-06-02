// Virtual Memory Manager for elinKernel  
// Implements virtual memory management for mmap/brk support
// Phase 4 implementation with page table management

use core::fmt;
use heapless::Vec;
use crate::memory::allocate_memory;
use spin::Mutex;
use crate::UART;
use core::fmt::Write;

/// Page size (4KB on RISC-V)
pub const PAGE_SIZE: usize = 4096;

/// Page table entry flags (RISC-V Sv39)
pub const PTE_VALID: u64 = 1 << 0;
pub const PTE_READ: u64 = 1 << 1;
pub const PTE_WRITE: u64 = 1 << 2;
pub const PTE_EXEC: u64 = 1 << 3;
pub const PTE_USER: u64 = 1 << 4;
pub const PTE_GLOBAL: u64 = 1 << 5;
pub const PTE_ACCESS: u64 = 1 << 6;
pub const PTE_DIRTY: u64 = 1 << 7;

/// Maximum number of VMAs per process
const MAX_VMAS: usize = 64;

/// Virtual memory area
#[derive(Debug, Clone)]
pub struct VirtualMemoryArea {
    pub start: usize,
    pub end: usize,
    pub protection: Protection,
    pub flags: VmaFlags,
    pub offset: usize, // For file-backed mappings
}

impl VirtualMemoryArea {
    pub fn new(start: usize, end: usize, protection: Protection, flags: VmaFlags) -> Self {
        VirtualMemoryArea {
            start,
            end,
            protection,
            flags,
            offset: 0,
        }
    }
    
    pub fn size(&self) -> usize {
        self.end - self.start
    }
    
    pub fn contains(&self, addr: usize) -> bool {
        addr >= self.start && addr < self.end
    }
    
    pub fn overlaps(&self, start: usize, end: usize) -> bool {
        !(end <= self.start || start >= self.end)
    }
}

/// Memory protection flags
#[derive(Debug, Clone, Copy)]
pub struct Protection {
    pub read: bool,
    pub write: bool,
    pub execute: bool,
}

impl Protection {
    pub fn new(read: bool, write: bool, execute: bool) -> Self {
        Protection { read, write, execute }
    }
    
    pub fn to_pte_flags(&self) -> u64 {
        let mut flags = PTE_VALID;
        if self.read { flags |= PTE_READ; }
        if self.write { flags |= PTE_WRITE; }
        if self.execute { flags |= PTE_EXEC; }
        flags |= PTE_USER; // User mode accessible
        flags
    }
}

/// VMA flags
#[derive(Debug, Clone, Copy)]
pub struct VmaFlags {
    pub shared: bool,
    pub anonymous: bool,
    pub fixed: bool,
    pub locked: bool,
}

impl VmaFlags {
    pub fn new() -> Self {
        VmaFlags {
            shared: false,
            anonymous: true,
            fixed: false,
            locked: false,
        }
    }
}

/// Page table entry
#[derive(Debug, Clone, Copy)]
pub struct PageTableEntry {
    pub value: u64,
}

impl PageTableEntry {
    pub fn new(ppn: u64, flags: u64) -> Self {
        PageTableEntry {
            value: (ppn << 10) | flags,
        }
    }
    
    pub fn is_valid(&self) -> bool {
        (self.value & PTE_VALID) != 0
    }
    
    pub fn ppn(&self) -> u64 {
        self.value >> 10
    }
    
    pub fn flags(&self) -> u64 {
        self.value & 0x3FF
    }
}

/// Simple page table management (simplified for now)
pub struct PageTable {
    entries: Vec<PageTableEntry, 512>, // Simple flat page table for now
    base_addr: usize,
}

impl PageTable {
    pub fn new(base_addr: usize) -> Self {
        PageTable {
            entries: Vec::new(),
            base_addr,
        }
    }
    
    pub fn map_page(&mut self, vaddr: usize, paddr: usize, flags: u64) -> Result<(), &'static str> {
        let vpn = vaddr / PAGE_SIZE;
        let ppn = (paddr / PAGE_SIZE) as u64;
        
        let entry = PageTableEntry::new(ppn, flags);
        
        // For simplicity, just add to our list (real implementation would use hierarchical tables)
        if self.entries.len() < 512 {
            self.entries.push(entry).map_err(|_| "Page table full")?;
            Ok(())
        } else {
            Err("Page table full")
        }
    }
    
    pub fn unmap_page(&mut self, vaddr: usize) -> Result<(), &'static str> {
        // TODO: Implement page unmapping
        Ok(())
    }
}

/// Virtual memory manager
pub struct VirtualMemoryManager {
    /// Virtual memory areas
    vmas: Vec<VirtualMemoryArea, MAX_VMAS>,
    
    /// Page table
    page_table: PageTable,
    
    /// Next available virtual address for anonymous mappings
    next_vaddr: usize,
    
    /// Program break (for brk implementation)
    program_break: usize,
    
    /// Heap start
    heap_start: usize,
    
    /// Statistics
    total_mapped: usize,
    total_allocated: usize,
}

impl VirtualMemoryManager {
    pub fn new() -> Self {
        // User space virtual memory layout
        let heap_start = 0x10000000; // 256MB
        let page_table_base = 0x80000000; // Use high memory for page table
        
        VirtualMemoryManager {
            vmas: Vec::new(),
            page_table: PageTable::new(page_table_base),
            next_vaddr: 0x20000000, // 512MB for mmap
            program_break: heap_start,
            heap_start,
            total_mapped: 0,
            total_allocated: 0,
        }
    }
    
    /// Memory mapping implementation
    pub fn mmap(&mut self, addr: usize, size: usize, prot: Protection, flags: VmaFlags) -> Result<usize, &'static str> {
        if size == 0 {
            return Err("Invalid size");
        }
        
        // Align size to page boundary
        let aligned_size = (size + PAGE_SIZE - 1) & !(PAGE_SIZE - 1);
        
        // Determine mapping address
        let map_addr = if flags.fixed && addr != 0 {
            // Fixed mapping at specified address
            if addr % PAGE_SIZE != 0 {
                return Err("Address not page aligned");
            }
            addr
        } else {
            // Find available virtual address
            self.find_free_vaddr(aligned_size)?
        };
        
        let map_end = map_addr + aligned_size;
        
        // Check for overlaps with existing VMAs
        for vma in &self.vmas {
            if vma.overlaps(map_addr, map_end) {
                return Err("Address range conflicts with existing mapping");
            }
        }
        
        // Allocate physical memory if anonymous mapping
        let mut physical_addrs = Vec::<usize, 256>::new();
        if flags.anonymous {
            for _i in 0..(aligned_size / PAGE_SIZE) {
                // Use our memory allocator to get physical pages
                if let Some(paddr) = allocate_memory(PAGE_SIZE) {
                    physical_addrs.push(paddr).map_err(|_| "Too many pages")?;
                } else {
                    // Clean up allocated pages on failure
                    for &paddr in &physical_addrs {
                        // TODO: Implement proper deallocation
                        let _ = paddr; // Suppress unused warning
                    }
                    return Err("Out of physical memory");
                }
            }
        }
        
        // Map pages in page table
        let pte_flags = prot.to_pte_flags();
        for (i, &paddr) in physical_addrs.iter().enumerate() {
            let vaddr = map_addr + (i * PAGE_SIZE);
            self.page_table.map_page(vaddr, paddr, pte_flags)?;
        }
        
        // Create VMA
        let vma = VirtualMemoryArea::new(map_addr, map_end, prot, flags);
        self.vmas.push(vma).map_err(|_| "Too many VMAs")?;
        
        // Update statistics
        self.total_mapped += aligned_size;
        self.total_allocated += aligned_size;
        
        Ok(map_addr)
    }
    
    /// Memory unmapping implementation
    pub fn munmap(&mut self, addr: usize, size: usize) -> Result<(), &'static str> {
        if addr % PAGE_SIZE != 0 {
            return Err("Address not page aligned");
        }
        
        let aligned_size = (size + PAGE_SIZE - 1) & !(PAGE_SIZE - 1);
        let end_addr = addr + aligned_size;
        
        // Find and remove overlapping VMAs
        let mut i = 0;
        while i < self.vmas.len() {
            let vma = &self.vmas[i];
            if vma.overlaps(addr, end_addr) {
                // Unmap pages from page table
                let start_page = vma.start / PAGE_SIZE;
                let end_page = (vma.end + PAGE_SIZE - 1) / PAGE_SIZE;
                
                for page in start_page..end_page {
                    let vaddr = page * PAGE_SIZE;
                    self.page_table.unmap_page(vaddr)?;
                    
                    // Free physical memory for anonymous mappings
                    if vma.flags.anonymous {
                        // TODO: Get physical address from page table and free it
                        // For now, we'll skip this to avoid complexity
                    }
                }
                
                // Update statistics
                self.total_mapped -= vma.size();
                
                // Remove VMA
                self.vmas.swap_remove(i);
            } else {
                i += 1;
            }
        }
        
        Ok(())
    }
    
    /// Program break implementation
    pub fn brk(&mut self, addr: usize) -> Result<usize, &'static str> {
        if addr == 0 {
            // Return current break
            return Ok(self.program_break);
        }
        
        if addr < self.heap_start {
            return Err("Break below heap start");
        }
        
        // Align to page boundary
        let aligned_addr = (addr + PAGE_SIZE - 1) & !(PAGE_SIZE - 1);
        
        if aligned_addr > self.program_break {
            // Expanding heap
            let size = aligned_addr - self.program_break;
            let prot = Protection::new(true, true, false); // RW-
            let mut flags = VmaFlags::new();
            flags.anonymous = true;
            
            // Map additional pages
            self.mmap(self.program_break, size, prot, flags)?;
        } else if aligned_addr < self.program_break {
            // Shrinking heap
            let size = self.program_break - aligned_addr;
            self.munmap(aligned_addr, size)?;
        }
        
        self.program_break = aligned_addr;
        Ok(self.program_break)
    }
    
    /// Find free virtual address range
    fn find_free_vaddr(&mut self, size: usize) -> Result<usize, &'static str> {
        let mut candidate = self.next_vaddr;
        
        // Align to page boundary
        candidate = (candidate + PAGE_SIZE - 1) & !(PAGE_SIZE - 1);
        
        // Check against existing VMAs
        loop {
            let end_candidate = candidate + size;
            let mut conflicts = false;
            
            for vma in &self.vmas {
                if vma.overlaps(candidate, end_candidate) {
                    conflicts = true;
                    candidate = vma.end;
                    break;
                }
            }
            
            if !conflicts {
                self.next_vaddr = end_candidate;
                return Ok(candidate);
            }
            
            // Prevent infinite loop
            if candidate > 0x80000000 {
                return Err("Virtual address space exhausted");
            }
        }
    }
    
    /// Get VMM statistics
    pub fn get_stats(&self) -> VmmStats {
        VmmStats {
            total_vmas: self.vmas.len(),
            total_mapped: self.total_mapped,
            total_allocated: self.total_allocated,
            program_break: self.program_break,
            heap_start: self.heap_start,
            next_vaddr: self.next_vaddr,
        }
    }
    
    /// List all VMAs for debugging
    pub fn list_vmas(&self) -> &[VirtualMemoryArea] {
        &self.vmas
    }
}

/// VMM statistics
#[derive(Debug)]
pub struct VmmStats {
    pub total_vmas: usize,
    pub total_mapped: usize,
    pub total_allocated: usize,
    pub program_break: usize,
    pub heap_start: usize,
    pub next_vaddr: usize,
} 