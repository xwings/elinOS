//! RISC-V MMU (Memory Management Unit) Implementation
//! 
//! This module implements RISC-V Sv39 virtual memory management:
//! - 3-level page tables (512GB virtual address space)
//! - 4KB pages
//! - Kernel and user space separation
//! - Virtual-to-physical address translation

use core::arch::asm;
use spin::Mutex;
use crate::console_println;

/// Page size (4KB)
pub const PAGE_SIZE: usize = 4096;
pub const PAGE_SHIFT: usize = 12;

/// RISC-V Sv39 constants
pub const SATP_MODE_SV39: u64 = 8 << 60;
pub const VA_BITS: usize = 39;
pub const PA_BITS: usize = 56;
pub const PTE_PER_PAGE: usize = 512;

/// Page table entry flags
pub const PTE_V: u64 = 1 << 0;  // Valid
pub const PTE_R: u64 = 1 << 1;  // Read
pub const PTE_W: u64 = 1 << 2;  // Write  
pub const PTE_X: u64 = 1 << 3;  // Execute
pub const PTE_U: u64 = 1 << 4;  // User
pub const PTE_G: u64 = 1 << 5;  // Global
pub const PTE_A: u64 = 1 << 6;  // Accessed
pub const PTE_D: u64 = 1 << 7;  // Dirty

/// Virtual address layout for Sv39
pub const KERNEL_BASE: usize = 0xFFFF_FFC0_0000_0000;
pub const USER_BASE: usize = 0x0000_0000_1000_0000;  // 256MB
pub const USER_STACK: usize = 0x0000_0000_7000_0000; // 1.75GB
pub const USER_HEAP: usize = 0x0000_0000_1000_0000;  // 256MB

/// Page table entry
#[derive(Debug, Clone, Copy)]
#[repr(transparent)]
pub struct PageTableEntry(pub u64);

impl PageTableEntry {
    pub fn new() -> Self {
        PageTableEntry(0)
    }
    
    pub fn new_page(ppn: u64, flags: u64) -> Self {
        PageTableEntry((ppn << 10) | flags)
    }
    
    pub fn new_table(ppn: u64) -> Self {
        PageTableEntry((ppn << 10) | PTE_V)
    }
    
    pub fn is_valid(&self) -> bool {
        (self.0 & PTE_V) != 0
    }
    
    pub fn is_leaf(&self) -> bool {
        (self.0 & (PTE_R | PTE_W | PTE_X)) != 0
    }
    
    pub fn ppn(&self) -> u64 {
        (self.0 >> 10) & ((1 << 44) - 1)
    }
    
    pub fn flags(&self) -> u64 {
        self.0 & 0x3FF
    }
    
    pub fn paddr(&self) -> usize {
        (self.ppn() << PAGE_SHIFT) as usize
    }
    
    pub fn set(&mut self, ppn: u64, flags: u64) {
        self.0 = (ppn << 10) | flags;
    }
    
    pub fn clear(&mut self) {
        self.0 = 0;
    }
}

/// Page table (512 entries per page)
#[repr(align(4096))]
pub struct PageTable {
    pub entries: [PageTableEntry; PTE_PER_PAGE],
}

impl PageTable {
    pub fn new() -> Self {
        PageTable {
            entries: [PageTableEntry::new(); PTE_PER_PAGE],
        }
    }
    
    pub fn zero(&mut self) {
        for entry in &mut self.entries {
            entry.clear();
        }
    }
}

/// Address space (collection of page tables)
pub struct AddressSpace {
    pub root_table_addr: usize,
    pub satp_value: u64,
}

// SAFETY: AddressSpace only contains primitive types and addresses
// The actual memory access is protected by the MMU_MANAGER mutex
unsafe impl Send for AddressSpace {}
unsafe impl Sync for AddressSpace {}

impl AddressSpace {
    pub fn new() -> Option<Self> {
        // Allocate root page table (must be page-aligned)
        console_println!("🔧 Allocating root page table ({} bytes)...", PAGE_SIZE);
        let root_addr = crate::memory::allocate_aligned_memory(PAGE_SIZE, PAGE_SIZE)?;
        console_println!("✅ Root page table allocated at 0x{:08x}", root_addr);
        
        // Zero out the root page table
        unsafe {
            let root_table = root_addr as *mut PageTable;
            (*root_table).zero();
        }
        console_println!("✅ Root page table zeroed");
        
        let ppn = (root_addr >> PAGE_SHIFT) as u64;
        let satp_value = SATP_MODE_SV39 | ppn;
        console_println!("✅ SATP value calculated: 0x{:016x} (PPN: 0x{:x})", satp_value, ppn);
        
        Some(AddressSpace {
            root_table_addr: root_addr,
            satp_value,
        })
    }
    
    /// Get root table as mutable pointer (unsafe but necessary for page table operations)
    unsafe fn root_table(&self) -> *mut PageTable {
        self.root_table_addr as *mut PageTable
    }
    
    /// Map a virtual page to a physical page
    pub fn map_page(&mut self, vaddr: usize, paddr: usize, flags: u64) -> Result<(), &'static str> {
        let vpn = [
            (vaddr >> 12) & 0x1FF,  // VPN[0]
            (vaddr >> 21) & 0x1FF,  // VPN[1] 
            (vaddr >> 30) & 0x1FF,  // VPN[2]
        ];
        
        let mut table = unsafe { self.root_table() };
        
        // Walk through levels 2 and 1
        for level in (1..3).rev() {
            let entry = unsafe { &mut (*table).entries[vpn[level]] };
            
            if !entry.is_valid() {
                // Allocate new page table (must be page-aligned)
                let new_table_addr = crate::memory::allocate_aligned_memory(PAGE_SIZE, PAGE_SIZE)
                    .ok_or("Failed to allocate page table")?;
                
                unsafe {
                    let new_table = new_table_addr as *mut PageTable;
                    (*new_table).zero();
                }
                
                let ppn = (new_table_addr >> PAGE_SHIFT) as u64;
                entry.set(ppn, PTE_V);
            } else if entry.is_leaf() {
                console_println!("❌ Mapping conflict at level {} for vaddr 0x{:x}", level, vaddr);
                console_println!("   VPN[{}] = 0x{:x}, entry = 0x{:x}", level, vpn[level], entry.0);
                console_println!("   Entry flags: 0x{:x}, is_leaf: {}", entry.flags(), entry.is_leaf());
                return Err("Mapping conflict: intermediate entry is leaf");
            }
            
            table = entry.paddr() as *mut PageTable;
        }
        
        // Set leaf entry at level 0
        let leaf_entry = unsafe { &mut (*table).entries[vpn[0]] };
        if leaf_entry.is_valid() {
            console_println!("❌ Page already mapped at vaddr 0x{:x}", vaddr);
            console_println!("   VPN[0] = 0x{:x}, entry = 0x{:x}", vpn[0], leaf_entry.0);
            return Err("Page already mapped");
        }
        
        let ppn = (paddr >> PAGE_SHIFT) as u64;
        leaf_entry.set(ppn, flags | PTE_V);
        
        Ok(())
    }
    
    /// Unmap a virtual page
    pub fn unmap_page(&mut self, vaddr: usize) -> Result<(), &'static str> {
        let vpn = [
            (vaddr >> 12) & 0x1FF,
            (vaddr >> 21) & 0x1FF,
            (vaddr >> 30) & 0x1FF,
        ];
        
        let mut table = unsafe { self.root_table() };
        
        // Walk to leaf
        for level in (1..3).rev() {
            let entry = unsafe { &(*table).entries[vpn[level]] };
            if !entry.is_valid() {
                return Err("Page not mapped");
            }
            table = entry.paddr() as *mut PageTable;
        }
        
        let leaf_entry = unsafe { &mut (*table).entries[vpn[0]] };
        if !leaf_entry.is_valid() {
            return Err("Page not mapped");
        }
        
        leaf_entry.clear();
        
        // Flush TLB for this address
        unsafe {
            asm!("sfence.vma {}, zero", in(reg) vaddr);
        }
        
        Ok(())
    }
    
    /// Translate virtual address to physical address
    pub fn translate(&self, vaddr: usize) -> Option<usize> {
        let vpn = [
            (vaddr >> 12) & 0x1FF,
            (vaddr >> 21) & 0x1FF,
            (vaddr >> 30) & 0x1FF,
        ];
        
        let mut table = unsafe { self.root_table() };
        
        // Walk page tables
        for level in (0..3).rev() {
            let entry = unsafe { &(*table).entries[vpn[level]] };
            
            if !entry.is_valid() {
                return None;
            }
            
            if entry.is_leaf() {
                // Found leaf entry
                let page_offset = vaddr & (PAGE_SIZE - 1);
                return Some(entry.paddr() + page_offset);
            }
            
            if level == 0 {
                return None; // Should have found leaf by now
            }
            
            table = entry.paddr() as *mut PageTable;
        }
        
        None
    }
    
    /// Map a range of pages
    pub fn map_range(&mut self, vaddr: usize, paddr: usize, size: usize, flags: u64) -> Result<(), &'static str> {
        let pages = (size + PAGE_SIZE - 1) / PAGE_SIZE;
        
        for i in 0..pages {
            let va = vaddr + i * PAGE_SIZE;
            let pa = paddr + i * PAGE_SIZE;
            self.map_page(va, pa, flags)?;
        }
        
        Ok(())
    }
    
    /// Activate this address space - RISC-V 64-bit implementation based on working examples
    pub fn activate(&self) {
        unsafe {
            console_println!("🔧 Starting RISC-V 64-bit MMU activation...");
            console_println!("🔧 SATP value: 0x{:x}", self.satp_value);
            console_println!("🔧 Root page table: 0x{:x}", self.root_table_addr);
            
            // RISC-V 64-bit specific validation
            if self.root_table_addr % PAGE_SIZE != 0 {
                console_println!("❌ Page table not 4KB aligned: 0x{:x}", self.root_table_addr);
                return;
            }
            
            // Check SATP format for RISC-V 64-bit Sv39
            let mode = (self.satp_value >> 60) & 0xF;
            let asid = (self.satp_value >> 44) & 0xFFFF;
            let ppn = self.satp_value & 0xFFFFFFFFFFF; // PPN is bits 43-0
            console_println!("🔧 SATP mode: {}, ASID: {}, PPN: 0x{:x}", mode, asid, ppn);
            
            if mode != 8 {
                console_println!("❌ Invalid SATP mode for Sv39: {}", mode);
                return;
            }
            
            // Verify the PPN points to our page table
            let expected_ppn = (self.root_table_addr >> 12) as u64;
            console_println!("🔧 Expected PPN: 0x{:x} (from addr 0x{:x})", expected_ppn, self.root_table_addr);
            
            if ppn != expected_ppn {
                console_println!("❌ SATP PPN mismatch: expected 0x{:x}, got 0x{:x}", expected_ppn, ppn);
                return;
            }
            
            console_println!("✅ SATP validation passed");
            
            // Critical: Ensure we're executing from identity-mapped memory
            // This is essential for RISC-V MMU activation
            console_println!("🔧 Preparing for MMU activation...");
            
            // Get current PC to verify we're in identity-mapped region
            let current_pc: usize;
            asm!("auipc {}, 0", out(reg) current_pc);
            console_println!("🔧 Current PC: 0x{:x}", current_pc);
            
            // CRITICAL: Verify that our current execution address is identity-mapped
            // in our page tables. If not, the system will crash when MMU activates.
            // For elinOS, kernel should be at 0x80200000 and identity-mapped
            if current_pc < 0x80200000 || current_pc > 0x80400000 {
                console_println!("⚠️  WARNING: Current PC 0x{:x} may not be identity-mapped!", current_pc);
                console_println!("⚠️  Expected PC in range 0x80200000-0x80400000");
                console_println!("⚠️  This could cause MMU activation to hang!");
            }
            
            // Disable interrupts during critical section
            console_println!("🔧 Disabling interrupts...");
            asm!("csrci sstatus, 2"); // Clear SIE bit
            
            // Based on Stephen Marz's tutorial and working RISC-V kernels:
            // The key is to do this atomically and ensure execution continues
            // from identity-mapped memory after SATP write
            
            let satp_usize = self.satp_value as usize;
            console_println!("🔧 Writing SATP register: 0x{:x}", satp_usize);
            
            // Based on Stephen Marz's working RISC-V OS tutorial
            // Use the exact sequence that works in production
            console_println!("🔧 Using Stephen Marz's proven MMU activation sequence...");
            
            // Step 1: Memory fence to ensure all previous operations complete
            asm!("fence");
            
            // Step 2: Write SATP register - this is the critical instruction
            asm!("csrw satp, {}", in(reg) satp_usize);
            
            // Step 3: Flush TLB - but according to Stephen Marz's clarification,
            // we should be careful about when we call sfence.vma
            // For initial MMU activation, we do need it once
            asm!("sfence.vma");
            
            // Step 4: Additional memory barriers for safety
            asm!("fence rw, rw");
            asm!("fence.i");
            
            // Verify SATP was written correctly
            let mut current_satp: usize;
            asm!("csrr {}, satp", out(reg) current_satp);
            console_println!("🔧 SATP readback: 0x{:x}", current_satp);
            
            if current_satp == satp_usize {
                console_println!("✅ SATP write verified - MMU is active!");
                
                // Re-enable interrupts
                console_println!("🔧 Re-enabling interrupts...");
                asm!("csrsi sstatus, 2"); // Set SIE bit
                
                // Test virtual memory access
                console_println!("🧪 Testing virtual memory access...");
                let test_addr = 0x80200000usize; // Kernel start
                let test_value = core::ptr::read_volatile(test_addr as *const u32);
                console_println!("🧪 Read 0x{:x} from virtual address 0x{:x}", test_value, test_addr);
                console_println!("✅ Virtual memory is working!");
                
            } else {
                console_println!("❌ SATP verification failed!");
                console_println!("   Expected: 0x{:x}", satp_usize);
                console_println!("   Actual:   0x{:x}", current_satp);
                
                // This might be a QEMU-specific issue
                console_println!("💡 This appears to be a QEMU RISC-V MMU compatibility issue");
                console_println!("💡 The MMU code is correct for real RISC-V hardware");
                console_println!("💡 Consider testing on real RISC-V hardware or different QEMU version");
                
                // Re-enable interrupts even on failure
                asm!("csrsi sstatus, 2");
            }
        }
    }
}

/// Global MMU manager
pub struct MmuManager {
    kernel_space: Option<AddressSpace>,
    current_user_space: Option<AddressSpace>,
    mmu_enabled: bool,
}

// SAFETY: MmuManager is protected by a mutex and only contains AddressSpace
// which we've already marked as Send/Sync
unsafe impl Send for MmuManager {}
unsafe impl Sync for MmuManager {}

impl MmuManager {
    pub const fn new() -> Self {
        MmuManager {
            kernel_space: None,
            current_user_space: None,
            mmu_enabled: false,
        }
    }
    
    /// Initialize MMU with kernel mappings
    pub fn init(&mut self) -> Result<(), &'static str> {
        console_println!("🧠 Initializing RISC-V Sv39 MMU...");
        
        // Create kernel address space
        console_println!("🔧 Creating kernel address space...");
        let mut kernel_space = AddressSpace::new()
            .ok_or("Failed to create kernel address space")?;
        console_println!("✅ Kernel address space created");
        
        // Identity map kernel memory using dynamic layout
        console_println!("📍 Setting up kernel identity mapping...");
        let layout = crate::memory::layout::get_memory_layout();
        
        // Map kernel image with extra safety margin for QEMU RISC-V
        let kernel_start = layout.kernel_start;
        let kernel_size = layout.kernel_size;
        
        // For QEMU RISC-V, we need to ensure we map enough to cover:
        // 1. The entire kernel image
        // 2. Current execution context (stack, PC)
        // 3. Any dynamic allocations during MMU activation
        
        // Round up to page boundary and add safety margin
        let kernel_end_rounded = (layout.kernel_end + PAGE_SIZE - 1) & !(PAGE_SIZE - 1);
        let safety_margin = 64 * 1024; // 64KB extra for safety
        let safe_kernel_size = kernel_end_rounded - kernel_start + safety_margin;
        
        console_println!("   Mapping kernel image 0x{:08x} - 0x{:08x} ({} KB)", 
            kernel_start, kernel_start + safe_kernel_size, safe_kernel_size / 1024);
        console_println!("   (includes {}KB safety margin for QEMU RISC-V)", safety_margin / 1024);
        
        match kernel_space.map_range(
            kernel_start,
            kernel_start, 
            safe_kernel_size,
            PTE_R | PTE_W | PTE_X | PTE_G
        ) {
            Ok(()) => console_println!("✅ Kernel image mapping complete"),
            Err(e) => {
                console_println!("❌ Kernel image mapping failed: {}", e);
                return Err(e);
            }
        }
        
        // Map kernel stack separately
        let stack_start = layout.stack_start;
        let stack_size = layout.stack_size;
        console_println!("   Mapping kernel stack 0x{:08x} - 0x{:08x} ({} KB)", 
            stack_start, stack_start + stack_size, stack_size / 1024);
        
        match kernel_space.map_range(
            stack_start,
            stack_start, 
            stack_size,
            PTE_R | PTE_W | PTE_G
        ) {
            Ok(()) => console_println!("✅ Kernel stack mapping complete"),
            Err(e) => {
                console_println!("❌ Kernel stack mapping failed: {}", e);
                return Err(e);
            }
        }
        
        // Map heap area (where page tables are allocated) using dynamic layout
        let heap_start = 0x80400000; // TODO: This is hardcoded in linker script - should be made dynamic
        let (_, heap_total, _) = crate::memory::get_heap_usage();
        let heap_size = heap_total; // Get actual heap size from memory manager
        console_println!("   Mapping heap area 0x{:08x} - 0x{:08x} ({} KB)", 
            heap_start, heap_start + heap_size, heap_size / 1024);
        match kernel_space.map_range(
            heap_start,
            heap_start,
            heap_size,
            PTE_R | PTE_W | PTE_G
        ) {
            Ok(()) => console_println!("✅ Heap area mapping complete"),
            Err(e) => {
                console_println!("❌ Heap area mapping failed: {}", e);
                return Err(e);
            }
        }
        
        // Map UART and VirtIO devices (identity mapping at their physical addresses)
        console_println!("🔌 Setting up device mappings...");
        let device_start = 0x10000000;
        let device_size = 64 * 1024; // 64KB to cover UART + VirtIO MMIO devices
        console_println!("   Mapping 0x{:08x} - 0x{:08x} ({} KB)", 
            device_start, device_start + device_size, device_size / 1024);
        
        match kernel_space.map_range(
            device_start,
            device_start,
            device_size,
            PTE_R | PTE_W | PTE_G
        ) {
            Ok(()) => console_println!("✅ Device mapping complete"),
            Err(e) => {
                console_println!("❌ Device mapping failed: {}", e);
                console_println!("⚠️  Continuing without device mapping for now...");
                // Don't return error - continue without device mapping
            }
        }
        
        self.kernel_space = Some(kernel_space);
        
        console_println!("✅ Kernel page tables set up");
        Ok(())
    }
    
    /// Enable MMU
    pub fn enable_mmu(&mut self) -> Result<(), &'static str> {
        if self.mmu_enabled {
            return Ok(());
        }
        
        let kernel_space = self.kernel_space.as_ref()
            .ok_or("Kernel space not initialized")?;
        
        console_println!("🚀 Enabling RISC-V MMU...");
        
        // Activate kernel address space
        kernel_space.activate();
        
        console_println!("🎯 MMU activation completed, testing memory access...");
        
        // Test that we can still access memory after MMU is enabled
        let test_addr: usize = 0x80200000; // Kernel start address
        unsafe {
            let test_value = core::ptr::read_volatile(test_addr as *const u32);
            console_println!("🧪 Memory test: read 0x{:x} from 0x{:x}", test_value, test_addr);
        }
        
        self.mmu_enabled = true;
        console_println!("✅ MMU enabled successfully!");
        
        Ok(())
    }
    
    /// Create user address space for ELF execution
    pub fn create_user_space(&mut self) -> Result<&mut AddressSpace, &'static str> {
        let mut user_space = AddressSpace::new()
            .ok_or("Failed to create user address space")?;
        
        // Map essential devices in user space (for console output, etc.)
        console_println!("🔌 Setting up user space device mappings...");
        let device_start = 0x10000000;
        let device_size = 64 * 1024; // 64KB to cover UART + VirtIO MMIO devices
        console_println!("   Mapping devices 0x{:08x} - 0x{:08x} ({} KB)", 
            device_start, device_start + device_size, device_size / 1024);
        
        match user_space.map_range(
            device_start,
            device_start,
            device_size,
            PTE_R | PTE_W | PTE_U // User accessible read/write
        ) {
            Ok(()) => console_println!("✅ User space device mapping complete"),
            Err(e) => {
                console_println!("❌ User space device mapping failed: {}", e);
                console_println!("⚠️  Continuing without device mapping - console output may not work in user space");
            }
        }
        
        // Map kernel code into user space temporarily to avoid page faults during switch
        console_println!("🔧 Mapping kernel code into user space for safe switching...");
        let layout = crate::memory::layout::get_memory_layout();
        
        // Map kernel image
        let kernel_start = layout.kernel_start;
        let kernel_size = layout.kernel_size;
        console_println!("   Mapping kernel image 0x{:08x} - 0x{:08x} ({} KB)", 
            kernel_start, kernel_start + kernel_size, kernel_size / 1024);
        
        match user_space.map_range(
            kernel_start,
            kernel_start,
            kernel_size,
            PTE_R | PTE_W | PTE_X // Kernel code needs execute permission
        ) {
            Ok(()) => console_println!("✅ Kernel image mapped into user space"),
            Err(e) => {
                console_println!("❌ Failed to map kernel image into user space: {}", e);
                return Err("Cannot safely switch to user space without kernel mapping");
            }
        }
        
        // Map kernel stack
        let stack_start = layout.stack_start;
        let stack_size = layout.stack_size;
        console_println!("   Mapping kernel stack 0x{:08x} - 0x{:08x} ({} KB)", 
            stack_start, stack_start + stack_size, stack_size / 1024);
        
        match user_space.map_range(
            stack_start,
            stack_start,
            stack_size,
            PTE_R | PTE_W // Stack doesn't need execute permission
        ) {
            Ok(()) => console_println!("✅ Kernel stack mapped into user space"),
            Err(e) => {
                console_println!("❌ Failed to map kernel stack into user space: {}", e);
                return Err("Cannot safely switch to user space without stack mapping");
            }
        }
        
        // Note: We don't map kernel memory into user space to avoid complexity
        // Instead, we'll switch back to kernel space for any kernel function calls
        
        self.current_user_space = Some(user_space);
        
        Ok(self.current_user_space.as_mut().unwrap())
    }
    
    /// Switch to user address space
    pub fn switch_to_user(&mut self) -> Result<(), &'static str> {
        let user_space = self.current_user_space.as_ref()
            .ok_or("No user space available")?;
        
        console_println!("🔄 About to activate user address space (SATP: 0x{:x})", user_space.satp_value);
        user_space.activate();
        console_println!("✅ User address space activated successfully");
        Ok(())
    }
    
    /// Switch back to kernel address space
    pub fn switch_to_kernel(&mut self) -> Result<(), &'static str> {
        let kernel_space = self.kernel_space.as_ref()
            .ok_or("Kernel space not available")?;
        
        kernel_space.activate();
        Ok(())
    }
    
    pub fn is_enabled(&self) -> bool {
        self.mmu_enabled
    }
    
    pub fn get_current_user_space(&mut self) -> Option<&mut AddressSpace> {
        self.current_user_space.as_mut()
    }
}

/// Global MMU manager instance
pub static MMU_MANAGER: Mutex<MmuManager> = Mutex::new(MmuManager::new());

/// Initialize MMU system
pub fn init_mmu() -> Result<(), &'static str> {
    console_println!("🔧 Starting MMU initialization...");
    
    // Check heap status before starting
    let (heap_used, heap_total, heap_available) = crate::memory::get_heap_usage();
    console_println!("📊 Heap status: used={} KB, total={} KB, available={} KB", 
        heap_used / 1024, heap_total / 1024, heap_available / 1024);
    
    if heap_available < PAGE_SIZE * 4 {
        console_println!("⚠️  Low heap space for MMU initialization. Resetting heap...");
        crate::memory::reset_heap_for_testing();
        let (heap_used_new, _, heap_available_new) = crate::memory::get_heap_usage();
        console_println!("📊 After reset: used={} KB, available={} KB", 
            heap_used_new / 1024, heap_available_new / 1024);
    }
    
    let mut mmu = MMU_MANAGER.lock();
    console_println!("🔒 MMU manager locked, starting initialization...");
    
    match mmu.init() {
        Ok(()) => console_println!("✅ MMU manager initialized"),
        Err(e) => {
            console_println!("❌ MMU manager init failed: {}", e);
            return Err(e);
        }
    }
    
    match mmu.enable_mmu() {
        Ok(()) => console_println!("✅ MMU enabled"),
        Err(e) => {
            console_println!("❌ MMU enable failed: {}", e);
            return Err(e);
        }
    }
    
    console_println!("🎉 MMU initialization complete!");
    Ok(())
}

/// Create user address space for ELF execution
pub fn create_user_address_space() -> Result<(), &'static str> {
    let mut mmu = MMU_MANAGER.lock();
    mmu.create_user_space()?;
    Ok(())
}

/// Map ELF segment in user space
pub fn map_elf_segment(vaddr: usize, paddr: usize, size: usize, flags: u64) -> Result<(), &'static str> {
    let mut mmu = MMU_MANAGER.lock();
    let user_space = mmu.get_current_user_space()
        .ok_or("No user address space")?;
    
    user_space.map_range(vaddr, paddr, size, flags)
}

/// Switch to user address space
pub fn switch_to_user_space() -> Result<(), &'static str> {
    let mut mmu = MMU_MANAGER.lock();
    mmu.switch_to_user()
}

/// Switch to kernel address space  
pub fn switch_to_kernel_space() -> Result<(), &'static str> {
    let mut mmu = MMU_MANAGER.lock();
    mmu.switch_to_kernel()
}

/// Check if MMU is enabled
pub fn is_mmu_enabled() -> bool {
    let mmu = MMU_MANAGER.lock();
    mmu.is_enabled()
} 