//! Simple Graphics System for elinOS
//! Provides basic framebuffer operations for drawing pixels and rectangles

use crate::console_println;
use crate::memory::mapping::{map_virtual_memory, MemoryPermissions};

/// Simple framebuffer for basic graphics operations
pub struct SimpleFramebuffer {
    buffer: *mut u32,
    width: u32,
    height: u32,
    pitch: u32, // bytes per row
    size: usize,
    bpp: u32, // bits per pixel
}

// Safety: We control access to the framebuffer through proper synchronization
unsafe impl Send for SimpleFramebuffer {}
unsafe impl Sync for SimpleFramebuffer {}

impl SimpleFramebuffer {
    /// Create a new framebuffer
    pub fn new(width: u32, height: u32, bpp: u32) -> Result<Self, &'static str> {
        let bytes_per_pixel = bpp / 8;
        let pitch = width * bytes_per_pixel;
        let size = (pitch * height) as usize;
        
        console_println!("[i] Setting up software framebuffer: {}x{} @ {} bpp", width, height, bpp);
        console_println!("[i] Framebuffer size: {} KB", size / 1024);
        
        // Allocate framebuffer memory using virtual memory manager
        match map_virtual_memory(size, MemoryPermissions::READ_WRITE, "Graphics-Framebuffer") {
            Ok(addr) => {
                console_println!("[o] VGA framebuffer mapped at 0x{:x}", addr);
                
                let framebuffer = SimpleFramebuffer {
                    buffer: addr as *mut u32,
                    width,
                    height,
                    pitch,
                    size,
                    bpp,
                };
                
                console_println!("[i] Simple framebuffer created:");
                console_println!("   Resolution: {}x{}", width, height);
                console_println!("   BPP: {}, Pitch: {}", bpp, pitch);
                console_println!("   Size: {} KB", size / 1024);
                console_println!("   Address: 0x{:x}", addr);
                
                Ok(framebuffer)
            }
            Err(_) => Err("Failed to allocate framebuffer memory")
        }
    }
    
    /// Clear the entire screen to a color
    pub fn clear(&mut self, color: u32) {
        let pixel_count = (self.width * self.height) as usize;
        unsafe {
            for i in 0..pixel_count {
                *self.buffer.add(i) = color;
            }
        }
    }
    
    /// Set a pixel at the given coordinates
    pub fn set_pixel(&mut self, x: u32, y: u32, color: u32) -> Result<(), &'static str> {
        if x >= self.width || y >= self.height {
            return Err("Pixel coordinates out of bounds");
        }
        
        let offset = (y * self.width + x) as usize;
        unsafe {
            *self.buffer.add(offset) = color;
        }
        Ok(())
    }
    
    /// Draw a filled rectangle
    pub fn draw_rect(&mut self, x: u32, y: u32, width: u32, height: u32, color: u32) -> Result<(), &'static str> {
        // Bounds checking
        if x >= self.width || y >= self.height {
            return Err("Rectangle coordinates out of bounds");
        }
        
        let end_x = (x + width).min(self.width);
        let end_y = (y + height).min(self.height);
        
        for row in y..end_y {
            for col in x..end_x {
                self.set_pixel(col, row, color)?;
            }
        }
        Ok(())
    }
    
    /// Get framebuffer information for VirtIO GPU
    pub fn get_framebuffer_info(&self) -> (usize, usize) {
        (self.buffer as usize, self.size)
    }
    
    pub fn get_dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }
}

// Global framebuffer instance
static mut FRAMEBUFFER: Option<SimpleFramebuffer> = None;
static mut VIRTIO_GPU_ENABLED: bool = false;

/// Initialize graphics system with VirtIO GPU support
pub fn init_graphics() -> Result<(), &'static str> {
    console_println!("[i] Initializing VGA graphics system...");
    
    // Create framebuffer first
    let mut framebuffer = SimpleFramebuffer::new(640, 480, 32)?;
    
    // Get framebuffer virtual address and size
    let (fb_virt_addr, fb_size) = framebuffer.get_framebuffer_info();
    
    // Get the physical address for VirtIO GPU
    let fb_phys_addr = match crate::memory::mapping::find_memory_mapping(fb_virt_addr) {
        Some(mapping) => {
            if let Some(phys_addr) = mapping.physical_addr {
                console_println!("[i] Framebuffer: virt=0x{:x}, phys=0x{:x}, size={} KB", 
                               fb_virt_addr, phys_addr, fb_size / 1024);
                phys_addr
            } else {
                console_println!("[!] No physical address found for framebuffer");
                return Err("Failed to get framebuffer physical address");
            }
        }
        None => {
            console_println!("[!] Framebuffer mapping not found");
            return Err("Framebuffer mapping not found");
        }
    };
    
    // Try to initialize VirtIO GPU with physical address
    console_println!("[i] Attempting VirtIO GPU initialization...");
    match crate::virtio::init_virtio_gpu(fb_phys_addr, fb_size) {
        Ok(()) => {
            console_println!("[o] VirtIO GPU initialized successfully!");
            console_println!("[i] Graphics output should now be visible in QEMU window!");
            unsafe { VIRTIO_GPU_ENABLED = true; }
        }
        Err(_) => {
            console_println!("[!] VirtIO GPU not available - using software framebuffer only");
            unsafe { VIRTIO_GPU_ENABLED = false; }
        }
    }
    
    // Initialize display mode
    console_println!("[i] Initializing VGA display mode...");
    
    // Clear to bright red background to make it very obvious (BGRA format: 0xBBGGRRAA)
    framebuffer.clear(0x0000FFFF); // Bright red in BGRA format
    console_println!("[o] VGA framebuffer cleared to bright red (BGRA)");
    
    // Draw test pattern - bright colors in BGRA format
    framebuffer.draw_rect(50, 50, 100, 100, 0x00FF00FF)?; // Bright green square
    framebuffer.draw_rect(200, 200, 200, 100, 0x00FFFFFF)?; // Bright yellow rectangle
    framebuffer.draw_rect(0, 0, 640, 10, 0xFFFFFFFF)?; // White top border
    framebuffer.draw_rect(0, 470, 640, 10, 0xFFFFFFFF)?; // White bottom border  
    console_println!("[o] VGA test pattern drawn with bright colors (BGRA format)");
    
    // Store framebuffer globally BEFORE flushing
    unsafe {
        FRAMEBUFFER = Some(framebuffer);
    }
    
    // Flush to display if VirtIO GPU is available
    if unsafe { VIRTIO_GPU_ENABLED } {
        console_println!("[i] Attempting to flush framebuffer to VirtIO GPU...");
        match crate::virtio::flush_display() {
            Ok(()) => console_println!("[o] Framebuffer successfully flushed to VirtIO GPU display"),
            Err(e) => console_println!("[!] Failed to flush framebuffer to display: {:?}", e),
        }
        
        // Try a second flush to make sure
        match crate::virtio::flush_display() {
            Ok(()) => console_println!("[o] Second flush completed"),
            Err(e) => console_println!("[!] Second flush failed: {:?}", e),
        }
    } else {
        console_println!("[i] VirtIO GPU not enabled, skipping flush");
    }
    
    console_println!("[o] VGA graphics system initialized");
    Ok(())
}

/// Clear screen to color
pub fn clear_screen(color: u32) -> Result<(), &'static str> {
    unsafe {
        if let Some(ref mut fb) = FRAMEBUFFER {
            fb.clear(color);
            
            // Flush to display if VirtIO GPU is available
            if VIRTIO_GPU_ENABLED {
                let _ = crate::virtio::flush_display();
            }
            
            Ok(())
        } else {
            Err("Graphics not initialized")
        }
    }
}

/// Set a pixel
pub fn set_pixel(x: u32, y: u32, color: u32) -> Result<(), &'static str> {
    unsafe {
        if let Some(ref mut fb) = FRAMEBUFFER {
            let result = fb.set_pixel(x, y, color);
            
            // Flush to display if VirtIO GPU is available
            if VIRTIO_GPU_ENABLED {
                let _ = crate::virtio::flush_display();
            }
            
            result
        } else {
            Err("Graphics not initialized")
        }
    }
}

/// Draw a rectangle
pub fn draw_rect(x: u32, y: u32, width: u32, height: u32, color: u32) -> Result<(), &'static str> {
    unsafe {
        if let Some(ref mut fb) = FRAMEBUFFER {
            let result = fb.draw_rect(x, y, width, height, color);
            
            // Flush to display if VirtIO GPU is available
            if VIRTIO_GPU_ENABLED {
                let _ = crate::virtio::flush_display();
            }
            
            result
        } else {
            Err("Graphics not initialized")
        }
    }
}

/// Get framebuffer dimensions
pub fn get_dimensions() -> Result<(u32, u32), &'static str> {
    unsafe {
        if let Some(ref fb) = FRAMEBUFFER {
            Ok(fb.get_dimensions())
        } else {
            Err("Graphics not initialized")
        }
    }
}

/// Flush framebuffer to display (if VirtIO GPU is available)
pub fn flush_to_display() -> Result<(), &'static str> {
    unsafe {
        if VIRTIO_GPU_ENABLED {
            match crate::virtio::flush_display() {
                Ok(()) => Ok(()),
                Err(_) => Err("Failed to flush to display"),
            }
        } else {
            Ok(()) // No-op for software framebuffer
        }
    }
} 