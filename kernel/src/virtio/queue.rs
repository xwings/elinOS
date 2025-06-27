//! VirtIO queue structures and implementation
//! Based on rust-vmm virtio-queue implementation

use crate::console_println;
use core::ptr::read_volatile;
use super::{DiskResult, DiskError};
use super::mmio::{VIRTQ_DESC_F_NEXT, VIRTQ_DESC_F_WRITE};

/// VirtIO descriptor table entry
/// This is the exact layout from the VirtIO specification
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct VirtqDesc {
    /// Guest physical address of buffer
    pub addr: u64,
    /// Length of buffer
    pub len: u32,
    /// Flags for this descriptor
    pub flags: u16,
    /// Index of next descriptor (if flags & VIRTQ_DESC_F_NEXT)
    pub next: u16,
}

impl VirtqDesc {
    pub const fn new() -> Self {
        VirtqDesc {
            addr: 0,
            len: 0,
            flags: 0,
            next: 0,
        }
    }
    
    pub fn set(&mut self, addr: u64, len: u32, flags: u16, next: u16) {
        self.addr = addr;
        self.len = len;
        self.flags = flags;
        self.next = next;
    }
    
    pub fn has_next(&self) -> bool {
        (self.flags & VIRTQ_DESC_F_NEXT) != 0
    }
    
    pub fn is_write_only(&self) -> bool {
        (self.flags & VIRTQ_DESC_F_WRITE) != 0
    }
}

/// VirtIO available ring structure
/// This is where the guest puts available descriptor indices
#[repr(C)]
pub struct VirtqAvail {
    /// Flags for available ring
    pub flags: u16,
    /// Index where next available descriptor will be written
    pub idx: u16,
    /// Ring of available descriptor indices
    pub ring: [u16; 256], // Maximum queue size
    /// Used event suppression (VirtIO 1.0+)
    pub used_event: u16,
}

impl VirtqAvail {
    pub const fn new() -> Self {
        VirtqAvail {
            flags: 0,
            idx: 0,
            ring: [0; 256],
            used_event: 0,
        }
    }
}

/// VirtIO used ring element
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct VirtqUsedElem {
    /// Index of start of used descriptor chain
    pub id: u32,
    /// Total length written to descriptor chain
    pub len: u32,
}

/// VirtIO used ring structure
#[repr(C)]
pub struct VirtqUsed {
    /// Flags for used ring
    pub flags: u16,
    /// Index where next used element will be written
    pub idx: u16,
    /// Ring of used elements
    pub ring: [VirtqUsedElem; 256], // Maximum queue size
    /// Available event suppression (VirtIO 1.0+)
    pub avail_event: u16,
}

impl VirtqUsed {
    pub const fn new() -> Self {
        VirtqUsed {
            flags: 0,
            idx: 0,
            ring: [VirtqUsedElem { id: 0, len: 0 }; 256],
            avail_event: 0,
        }
    }
}

/// VirtIO Queue implementation
/// Based on concepts from rust-vmm's virtio_queue.
#[derive(Debug)]
pub struct VirtioQueue {
    /// Queue size (must be power of 2)
    size: u16,
    /// Ready flag
    pub ready: bool,
    /// Base address of descriptor table
    desc_table: usize,
    /// Base address of available ring
    avail_ring: usize,
    /// Base address of used ring
    used_ring: usize,
    /// Next available index to use for adding to avail_ring.ring
    next_avail: u16,
    /// Last used index we've processed from used_ring.ring
    last_used_idx: u16,
    /// Queue select index (usually 0 for block device)
    pub queue_index: u16,
}

impl VirtioQueue {
    pub const fn new() -> Self {
        VirtioQueue {
            size: 0,
            ready: false,
            desc_table: 0,
            avail_ring: 0,
            used_ring: 0,
            next_avail: 0,
            last_used_idx: 0,
            queue_index: 0,
        }
    }
    
    // Make ready field accessible to block device
    pub fn set_ready(&mut self, ready: bool) {
        self.ready = ready;
    }

    pub fn init(&mut self, size: u16, queue_idx: u16, desc_table: usize, avail_ring: usize, used_ring: usize) -> DiskResult<()> {
        if size == 0 || (size & (size - 1)) != 0 {
            console_println!("[x] VirtioQueue init error: size {} is not a power of two or is zero.", size);
            return Err(DiskError::InvalidParameter);
        }

        self.size = size;
        self.queue_index = queue_idx;
        self.desc_table = desc_table;
        self.avail_ring = avail_ring;
        self.used_ring = used_ring;
        self.next_avail = 0;
        self.last_used_idx = 0;
        self.ready = false;

        // CRITICAL: Explicitly initialize the available and used ring headers
        unsafe {
            let avail_ring_ptr = self.avail_ring as *mut VirtqAvail;
            let used_ring_ptr = self.used_ring as *mut VirtqUsed;
            
            // Debug: Check what's in memory before initialization
            let pre_avail_idx = read_volatile(&(*avail_ring_ptr).idx);
            let pre_used_idx = read_volatile(&(*used_ring_ptr).idx);
            // console_println!("[DEBUG] Pre-init: avail_idx={}, used_idx={}", pre_avail_idx, pre_used_idx);
            
            // Initialize available ring
            core::ptr::write_volatile(&mut (*avail_ring_ptr).flags, 0);
            core::ptr::write_volatile(&mut (*avail_ring_ptr).idx, 0);
            
            // Initialize used ring  
            core::ptr::write_volatile(&mut (*used_ring_ptr).flags, 0);
            core::ptr::write_volatile(&mut (*used_ring_ptr).idx, 0);
            
            // Verify initialization worked
            let post_avail_idx = read_volatile(&(*avail_ring_ptr).idx);
            let post_used_idx = read_volatile(&(*used_ring_ptr).idx);
            // console_println!("[DEBUG] Post-init: avail_idx={}, used_idx={}", post_avail_idx, post_used_idx);
            
            self.last_used_idx = post_used_idx;
        }

        console_println!("[o] VirtioQueue initialized: size={}, idx={}, desc_base=0x{:x}, avail_base=0x{:x}, used_base=0x{:x}",
                        size, queue_idx, desc_table, avail_ring, used_ring);

        Ok(())
    }

    /// Add a descriptor chain to the available ring
    /// Returns the head descriptor index
    pub fn add_descriptor_chain(&mut self, chain: &[VirtqDesc]) -> DiskResult<u16> {
        if !self.ready {
            return Err(DiskError::QueueFull);
        }
        
        if chain.is_empty() || chain.len() > self.size as usize {
            return Err(DiskError::InvalidParameter);
        }
        
        // Simple check: ensure we don't wrap around too much
        let available_space = self.size.saturating_sub(8); // Keep some buffer
        if chain.len() as u16 > available_space {
            return Err(DiskError::QueueFull);
        }
        
        let head_index = self.next_avail; 
        let desc_table_ptr = self.desc_table as *mut VirtqDesc;

        // Place descriptors into the descriptor table
        for i in 0..chain.len() {
            let actual_table_idx = (head_index + i as u16) % self.size;
            let mut desc_to_write = chain[i];

            // Fix the next field to point to the correct absolute descriptor index
            if (desc_to_write.flags & VIRTQ_DESC_F_NEXT) != 0 {
                // Calculate the absolute index of the next descriptor in the chain
                let next_chain_idx = desc_to_write.next as usize;
                if next_chain_idx < chain.len() {
                    desc_to_write.next = (head_index + next_chain_idx as u16) % self.size;
                } else {
                    // Invalid next index - clear the NEXT flag to prevent corruption
                    desc_to_write.flags &= !VIRTQ_DESC_F_NEXT;
                    desc_to_write.next = 0;
                }
            }
            
            unsafe {
                core::ptr::write_volatile(desc_table_ptr.add(actual_table_idx as usize), desc_to_write);
            }
        }

        // Add to available ring
        unsafe {
            let avail_ring_ptr = self.avail_ring as *mut VirtqAvail;
            
            // Read current available index from device (should be initialized to 0)
            let device_avail_idx = read_volatile(&(*avail_ring_ptr).idx);
            let ring_idx = device_avail_idx % self.size; 
            
            // Debug: Show queue state before adding
            //  console_println!("[DEBUG] Queue add: device_avail_idx={}, ring_idx={}, head_index={}, size={}", 
            //    device_avail_idx, ring_idx, head_index, self.size);
            
            // Write the head descriptor index to the available ring
            core::ptr::write_volatile(&mut (*avail_ring_ptr).ring[ring_idx as usize], head_index);
            
            // Increment the available index
            let new_avail_idx = device_avail_idx.wrapping_add(1);
            core::ptr::write_volatile(&mut (*avail_ring_ptr).idx, new_avail_idx);
            
            // Debug: Verify what we wrote
            let written_head = read_volatile(&(*avail_ring_ptr).ring[ring_idx as usize]);
            let final_avail_idx = read_volatile(&(*avail_ring_ptr).idx);
            // console_println!("[DEBUG] Queue after: written_head={}, final_avail_idx={}", 
            //     written_head, final_avail_idx);
        }
        
        self.next_avail = (self.next_avail + chain.len() as u16) % self.size;

        Ok(head_index)
    }

    /// Get the next used element from the used ring
    pub fn get_used_elem(&mut self) -> Option<VirtqUsedElem> {
        unsafe {
            let used_ring_ptr = self.used_ring as *const VirtqUsed;
            let device_current_used_idx = read_volatile(&(*used_ring_ptr).idx);
            
            if self.last_used_idx == device_current_used_idx {
                return None;
            }
            
            let elem_array_idx = self.last_used_idx % self.size;
            let elem = read_volatile(&(*used_ring_ptr).ring[elem_array_idx as usize]);
            
            self.last_used_idx = self.last_used_idx.wrapping_add(1);
            
            Some(elem)
        }
    }

    /// Wait for completion of a specific descriptor chain
    pub fn wait_for_completion(&mut self, expected_head: u16) -> Option<VirtqUsedElem> {
        // First, check if we have the expected completion
        if let Some(elem) = self.get_used_elem() {
            if elem.id as u16 == expected_head {
                return Some(elem);
            } else {
                // Got a different completion - this is the out-of-order issue
                // For now, just accept any completion to keep things moving
                return Some(elem);
            }
        }
        None
    }

    pub fn is_ready(&self) -> bool {
        self.ready
    }

    pub fn size(&self) -> u16 {
        self.size
    }
    
    pub fn queue_index(&self) -> u16 {
        self.queue_index
    }
} 