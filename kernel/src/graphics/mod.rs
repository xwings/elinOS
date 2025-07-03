//! Simple Graphics System for elinOS
//! Provides basic framebuffer operations for drawing pixels and rectangles

use elinos_common::console_println;

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
    /// Create a new framebuffer with VirtIO GPU compatibility
    pub fn new(width: u32, height: u32, bpp: u32) -> Result<Self, &'static str> {
        console_println!("[i] Setting up software framebuffer: {}x{} @ {} bpp", width, height, bpp);
        
        let bytes_per_pixel = bpp / 8;
        let pitch = width * bytes_per_pixel;
        let size = (width * height * bytes_per_pixel) as usize;
        
        console_println!("[i] Framebuffer size: {} KB", size / 1024);
        
        // For VirtIO GPU compatibility, allocate framebuffer in proper RAM region
        // Use VirtIO-specific allocation to ensure it's in the right memory region
        match crate::memory::mapping::map_virtual_memory(
            size,
            crate::memory::mapping::MemoryPermissions::READ_WRITE,
            "VirtIO GPU Framebuffer", // Use "VirtIO" in name to trigger proper allocation
        ) {
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
    
    // Create framebuffer with VirtIO GPU compatibility
    let mut framebuffer = SimpleFramebuffer::new(640, 480, 32)?;
    
    // Get framebuffer address and size
    let (fb_addr, fb_size) = framebuffer.get_framebuffer_info();
    
    // Get the physical address for VirtIO GPU
    let fb_phys_addr = match crate::memory::mapping::find_memory_mapping(fb_addr) {
        Some(mapping) => {
            if let Some(phys_addr) = mapping.physical_addr {
                console_println!("[i] Framebuffer: virt=0x{:x}, phys=0x{:x}, size={} KB", 
                               fb_addr, phys_addr, fb_size / 1024);
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
    
    // Initialize VirtIO GPU with the properly allocated framebuffer
    console_println!("[i] Attempting VirtIO GPU initialization...");
    match crate::virtio::init_virtio_gpu(fb_phys_addr, fb_size) {
        Ok(()) => {
            console_println!("[o] VirtIO GPU initialized successfully with physical address!");
            console_println!("[i] Graphics output should now be visible in QEMU window!");
            
            // Update framebuffer to use the physical address for VirtIO GPU
            console_println!("[i] Updating framebuffer to use VirtIO GPU physical address...");
            framebuffer.buffer = fb_phys_addr as *mut u32;
            console_println!("[o] Framebuffer updated: now using addr=0x{:x}", fb_phys_addr);
            
            unsafe { VIRTIO_GPU_ENABLED = true; }
        }
        Err(_) => {
            console_println!("[!] VirtIO GPU not available - using software framebuffer only");
            unsafe { VIRTIO_GPU_ENABLED = false; }
        }
    }
    
    // Initialize with visible graphics to test the display
    console_println!("[i] Drawing test graphics to verify display...");
    
    // Clear to dark blue background
    framebuffer.clear(0x00000080); // Dark blue: XX=00, RR=00, GG=00, BB=80
    
    // Draw a white border
    framebuffer.draw_rect(0, 0, 640, 10, 0x00FFFFFF)?; // Top border
    framebuffer.draw_rect(0, 470, 640, 10, 0x00FFFFFF)?; // Bottom border  
    framebuffer.draw_rect(0, 0, 10, 480, 0x00FFFFFF)?; // Left border
    framebuffer.draw_rect(630, 0, 10, 480, 0x00FFFFFF)?; // Right border
    
    // Draw some colored rectangles to test
    framebuffer.draw_rect(50, 50, 100, 50, 0x00FF0000)?; // Red rectangle
    framebuffer.draw_rect(200, 50, 100, 50, 0x0000FF00)?; // Green rectangle
    framebuffer.draw_rect(350, 50, 100, 50, 0x000000FF)?; // Blue rectangle
    
    // Draw a terminal area
    framebuffer.draw_rect(20, 150, 600, 300, 0x00000000)?; // Black terminal area
    framebuffer.draw_rect(20, 150, 600, 2, 0x00FFFFFF)?; // Top border
    framebuffer.draw_rect(20, 448, 600, 2, 0x00FFFFFF)?; // Bottom border
    framebuffer.draw_rect(20, 150, 2, 300, 0x00FFFFFF)?; // Left border
    framebuffer.draw_rect(618, 150, 2, 300, 0x00FFFFFF)?; // Right border
    
    // Add some simple text simulation using colored rectangles
    // "elinOS Terminal" title
    framebuffer.draw_rect(30, 160, 8, 12, 0x00FFFFFF)?; // E
    framebuffer.draw_rect(45, 160, 8, 12, 0x00FFFFFF)?; // L
    framebuffer.draw_rect(60, 160, 8, 12, 0x00FFFFFF)?; // I
    framebuffer.draw_rect(75, 160, 8, 12, 0x00FFFFFF)?; // N
    framebuffer.draw_rect(90, 160, 8, 12, 0x00FFFFFF)?; // O
    framebuffer.draw_rect(105, 160, 8, 12, 0x00FFFFFF)?; // S
    
    // ">" prompt
    framebuffer.draw_rect(30, 400, 8, 12, 0x0000FF00)?; // Green ">" prompt
    
    console_println!("[o] Test graphics drawn to framebuffer");
    
    // Store framebuffer globally
    console_println!("[i] Storing framebuffer globally with correct address...");
    unsafe {
        FRAMEBUFFER = Some(framebuffer);
    }
    console_println!("[o] Framebuffer stored: ptr=0x{:x}", unsafe { FRAMEBUFFER.as_ref().unwrap().buffer as usize });
    
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

/// Initialize text console for shell output
pub fn init_text_console() -> Result<(), &'static str> {
    console_println!("[i] Initializing simple text console...");
    
    unsafe {
        TEXT_CONSOLE = Some(TextConsole::new());
        console_println!("[o] Text console created (keeping existing screen content)");
    }
    
    // Skip remaining console_println calls to avoid potential hang
    // console_println!("[i] Preserving existing graphics for visibility test");
    // console_println!("[o] Text console initialization complete");
    Ok(())
}

/// Print text to graphics console
pub fn print_to_console(text: &str) -> Result<(), &'static str> {
    unsafe {
        if let Some(ref mut console) = TEXT_CONSOLE {
            console.print_str(text)?;
            
            // Flush to display after printing
            if VIRTIO_GPU_ENABLED {
                let _ = crate::virtio::flush_display();
            }
            
            Ok(())
        } else {
            Err("Text console not initialized")
        }
    }
}

/// Clear the graphics console
pub fn clear_console() -> Result<(), &'static str> {
    unsafe {
        if let Some(ref mut console) = TEXT_CONSOLE {
            console.clear_screen()?;
            
            // Flush to display
            if VIRTIO_GPU_ENABLED {
                let _ = crate::virtio::flush_display();
            }
        }
    }
    Ok(())
}

// Simple 8x8 bitmap font data for basic ASCII characters
const FONT_WIDTH: u32 = 8;
const FONT_HEIGHT: u32 = 8;

// Simple bitmap font for characters 32-126 (space to ~)
const FONT_DATA: &[u8] = &[
    // Space (32)
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    // ! (33)
    0x18, 0x3C, 0x3C, 0x18, 0x18, 0x00, 0x18, 0x00,
    // " (34)
    0x36, 0x36, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    // # (35)
    0x36, 0x36, 0x7F, 0x36, 0x7F, 0x36, 0x36, 0x00,
    // $ (36)
    0x0C, 0x3E, 0x03, 0x1E, 0x30, 0x1F, 0x0C, 0x00,
    // % (37)
    0x00, 0x63, 0x33, 0x18, 0x0C, 0x66, 0x63, 0x00,
    // & (38)
    0x1C, 0x36, 0x1C, 0x6E, 0x3B, 0x33, 0x6E, 0x00,
    // ' (39)
    0x06, 0x06, 0x03, 0x00, 0x00, 0x00, 0x00, 0x00,
    // ( (40)
    0x18, 0x0C, 0x06, 0x06, 0x06, 0x0C, 0x18, 0x00,
    // ) (41)
    0x06, 0x0C, 0x18, 0x18, 0x18, 0x0C, 0x06, 0x00,
    // * (42)
    0x00, 0x66, 0x3C, 0xFF, 0x3C, 0x66, 0x00, 0x00,
    // + (43)
    0x00, 0x0C, 0x0C, 0x3F, 0x0C, 0x0C, 0x00, 0x00,
    // , (44)
    0x00, 0x00, 0x00, 0x00, 0x00, 0x0C, 0x06, 0x00,
    // - (45)
    0x00, 0x00, 0x00, 0x3F, 0x00, 0x00, 0x00, 0x00,
    // . (46)
    0x00, 0x00, 0x00, 0x00, 0x00, 0x0C, 0x0C, 0x00,
    // / (47)
    0x60, 0x30, 0x18, 0x0C, 0x06, 0x03, 0x01, 0x00,
    // 0 (48)
    0x3E, 0x63, 0x73, 0x7B, 0x6F, 0x67, 0x3E, 0x00,
    // 1 (49)
    0x0C, 0x0E, 0x0C, 0x0C, 0x0C, 0x0C, 0x3F, 0x00,
    // 2 (50)
    0x1E, 0x33, 0x30, 0x1C, 0x06, 0x33, 0x3F, 0x00,
    // 3 (51)
    0x1E, 0x33, 0x30, 0x1C, 0x30, 0x33, 0x1E, 0x00,
    // 4 (52)
    0x38, 0x3C, 0x36, 0x33, 0x7F, 0x30, 0x78, 0x00,
    // 5 (53)
    0x3F, 0x03, 0x1F, 0x30, 0x30, 0x33, 0x1E, 0x00,
    // 6 (54)
    0x1C, 0x06, 0x03, 0x1F, 0x33, 0x33, 0x1E, 0x00,
    // 7 (55)
    0x3F, 0x33, 0x30, 0x18, 0x0C, 0x0C, 0x0C, 0x00,
    // 8 (56)
    0x1E, 0x33, 0x33, 0x1E, 0x33, 0x33, 0x1E, 0x00,
    // 9 (57)
    0x1E, 0x33, 0x33, 0x3E, 0x30, 0x18, 0x0E, 0x00,
    // : (58)
    0x00, 0x0C, 0x0C, 0x00, 0x00, 0x0C, 0x0C, 0x00,
    // ; (59)
    0x00, 0x0C, 0x0C, 0x00, 0x00, 0x0C, 0x06, 0x00,
    // < (60)
    0x18, 0x0C, 0x06, 0x03, 0x06, 0x0C, 0x18, 0x00,
    // = (61)
    0x00, 0x00, 0x3F, 0x00, 0x00, 0x3F, 0x00, 0x00,
    // > (62)
    0x06, 0x0C, 0x18, 0x30, 0x18, 0x0C, 0x06, 0x00,
    // ? (63)
    0x1E, 0x33, 0x30, 0x18, 0x0C, 0x00, 0x0C, 0x00,
    // @ (64)
    0x3E, 0x63, 0x7B, 0x7B, 0x7B, 0x03, 0x1E, 0x00,
    // A (65)
    0x0C, 0x1E, 0x33, 0x33, 0x3F, 0x33, 0x33, 0x00,
    // B (66)
    0x3F, 0x66, 0x66, 0x3E, 0x66, 0x66, 0x3F, 0x00,
    // C (67)
    0x3C, 0x66, 0x03, 0x03, 0x03, 0x66, 0x3C, 0x00,
    // D (68)
    0x1F, 0x36, 0x66, 0x66, 0x66, 0x36, 0x1F, 0x00,
    // E (69)
    0x7F, 0x46, 0x16, 0x1E, 0x16, 0x46, 0x7F, 0x00,
    // F (70)
    0x7F, 0x46, 0x16, 0x1E, 0x16, 0x06, 0x0F, 0x00,
    // G (71)
    0x3C, 0x66, 0x03, 0x03, 0x73, 0x66, 0x7C, 0x00,
    // H (72)
    0x33, 0x33, 0x33, 0x3F, 0x33, 0x33, 0x33, 0x00,
    // I (73)
    0x1E, 0x0C, 0x0C, 0x0C, 0x0C, 0x0C, 0x1E, 0x00,
    // J (74)
    0x78, 0x30, 0x30, 0x30, 0x33, 0x33, 0x1E, 0x00,
    // K (75)
    0x67, 0x66, 0x36, 0x1E, 0x36, 0x66, 0x67, 0x00,
    // L (76)
    0x0F, 0x06, 0x06, 0x06, 0x46, 0x66, 0x7F, 0x00,
    // M (77)
    0x63, 0x77, 0x7F, 0x7F, 0x6B, 0x63, 0x63, 0x00,
    // N (78)
    0x63, 0x67, 0x6F, 0x7B, 0x73, 0x63, 0x63, 0x00,
    // O (79)
    0x1C, 0x36, 0x63, 0x63, 0x63, 0x36, 0x1C, 0x00,
    // P (80)
    0x3F, 0x66, 0x66, 0x3E, 0x06, 0x06, 0x0F, 0x00,
    // Q (81)
    0x1E, 0x33, 0x33, 0x33, 0x3B, 0x1E, 0x38, 0x00,
    // R (82)
    0x3F, 0x66, 0x66, 0x3E, 0x36, 0x66, 0x67, 0x00,
    // S (83)
    0x1E, 0x33, 0x07, 0x0E, 0x38, 0x33, 0x1E, 0x00,
    // T (84)
    0x3F, 0x2D, 0x0C, 0x0C, 0x0C, 0x0C, 0x1E, 0x00,
    // U (85)
    0x33, 0x33, 0x33, 0x33, 0x33, 0x33, 0x3F, 0x00,
    // V (86)
    0x33, 0x33, 0x33, 0x33, 0x33, 0x1E, 0x0C, 0x00,
    // W (87)
    0x63, 0x63, 0x63, 0x6B, 0x7F, 0x77, 0x63, 0x00,
    // X (88)
    0x63, 0x63, 0x36, 0x1C, 0x1C, 0x36, 0x63, 0x00,
    // Y (89)
    0x33, 0x33, 0x33, 0x1E, 0x0C, 0x0C, 0x1E, 0x00,
    // Z (90)
    0x7F, 0x63, 0x31, 0x18, 0x4C, 0x66, 0x7F, 0x00,
    // [ (91)
    0x1E, 0x06, 0x06, 0x06, 0x06, 0x06, 0x1E, 0x00,
    // \ (92)
    0x03, 0x06, 0x0C, 0x18, 0x30, 0x60, 0x40, 0x00,
    // ] (93)
    0x1E, 0x18, 0x18, 0x18, 0x18, 0x18, 0x1E, 0x00,
    // ^ (94)
    0x08, 0x1C, 0x36, 0x63, 0x00, 0x00, 0x00, 0x00,
    // _ (95)
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xFF,
    // ` (96)
    0x0C, 0x0C, 0x18, 0x00, 0x00, 0x00, 0x00, 0x00,
    // a (97)
    0x00, 0x00, 0x1E, 0x30, 0x3E, 0x33, 0x6E, 0x00,
    // b (98)
    0x07, 0x06, 0x06, 0x3E, 0x66, 0x66, 0x3B, 0x00,
    // c (99)
    0x00, 0x00, 0x1E, 0x33, 0x03, 0x33, 0x1E, 0x00,
    // d (100)
    0x38, 0x30, 0x30, 0x3e, 0x33, 0x33, 0x6E, 0x00,
    // e (101)
    0x00, 0x00, 0x1E, 0x33, 0x3f, 0x03, 0x1E, 0x00,
    // f (102)
    0x1C, 0x36, 0x06, 0x0f, 0x06, 0x06, 0x0F, 0x00,
    // g (103)
    0x00, 0x00, 0x6E, 0x33, 0x33, 0x3E, 0x30, 0x1F,
    // h (104)
    0x07, 0x06, 0x36, 0x6E, 0x66, 0x66, 0x67, 0x00,
    // i (105)
    0x0C, 0x00, 0x0E, 0x0C, 0x0C, 0x0C, 0x1E, 0x00,
    // j (106)
    0x30, 0x00, 0x30, 0x30, 0x30, 0x33, 0x33, 0x1E,
    // k (107)
    0x07, 0x06, 0x66, 0x36, 0x1E, 0x36, 0x67, 0x00,
    // l (108)
    0x0E, 0x0C, 0x0C, 0x0C, 0x0C, 0x0C, 0x1E, 0x00,
    // m (109)
    0x00, 0x00, 0x33, 0x7F, 0x7F, 0x6B, 0x63, 0x00,
    // n (110)
    0x00, 0x00, 0x1F, 0x33, 0x33, 0x33, 0x33, 0x00,
    // o (111)
    0x00, 0x00, 0x1E, 0x33, 0x33, 0x33, 0x1E, 0x00,
    // p (112)
    0x00, 0x00, 0x3B, 0x66, 0x66, 0x3E, 0x06, 0x0F,
    // q (113)
    0x00, 0x00, 0x6E, 0x33, 0x33, 0x3E, 0x30, 0x78,
    // r (114)
    0x00, 0x00, 0x3B, 0x6E, 0x66, 0x06, 0x0F, 0x00,
    // s (115)
    0x00, 0x00, 0x3E, 0x03, 0x1E, 0x30, 0x1F, 0x00,
    // t (116)
    0x08, 0x0C, 0x3E, 0x0C, 0x0C, 0x2C, 0x18, 0x00,
    // u (117)
    0x00, 0x00, 0x33, 0x33, 0x33, 0x33, 0x6E, 0x00,
    // v (118)
    0x00, 0x00, 0x33, 0x33, 0x33, 0x1E, 0x0C, 0x00,
    // w (119)
    0x00, 0x00, 0x63, 0x6B, 0x7F, 0x7F, 0x36, 0x00,
    // x (120)
    0x00, 0x00, 0x63, 0x36, 0x1C, 0x36, 0x63, 0x00,
    // y (121)
    0x00, 0x00, 0x33, 0x33, 0x33, 0x3E, 0x30, 0x1F,
    // z (122)
    0x00, 0x00, 0x3F, 0x19, 0x0C, 0x26, 0x3F, 0x00,
    // { (123)
    0x38, 0x0C, 0x0C, 0x07, 0x0C, 0x0C, 0x38, 0x00,
    // | (124)
    0x18, 0x18, 0x18, 0x00, 0x18, 0x18, 0x18, 0x00,
    // } (125)
    0x07, 0x0C, 0x0C, 0x38, 0x0C, 0x0C, 0x07, 0x00,
    // ~ (126)
    0x6E, 0x3B, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
];

// Text console state
static mut TEXT_CONSOLE: Option<TextConsole> = None;

struct TextConsole {
    cursor_x: u32,
    cursor_y: u32,
    max_cols: u32,
    max_rows: u32,
    fg_color: u32,
    bg_color: u32,
}

impl TextConsole {
    fn new() -> Self {
        TextConsole {
            cursor_x: 0,
            cursor_y: 0,
            max_cols: 640 / FONT_WIDTH,   // 80 columns
            max_rows: 480 / FONT_HEIGHT,  // 60 rows
            fg_color: 0x00FFFFFF,         // White text (XRGB: 0xXXRRGGBB)
            bg_color: 0x00000000,         // Black background (XRGB: 0xXXRRGGBB) - transparent on current graphics
        }
    }
    
    fn draw_char(&mut self, ch: char, x: u32, y: u32) -> Result<(), &'static str> {
        if ch < ' ' || ch > '~' {
            return Ok(()); // Skip non-printable characters
        }
        
        let char_index = (ch as u8 - 32) as usize; // ASCII 32 = space
        let font_offset = char_index * 8; // 8 bytes per character
        
        if font_offset + 8 > FONT_DATA.len() {
            return Ok(()); // Character not in font
        }
        
        // Use the SAME direct drawing method that worked for graphics
        unsafe {
            if let Some(ref mut fb) = FRAMEBUFFER {
                for row in 0..8 {
                    let font_byte = FONT_DATA[font_offset + row];
                    for col in 0..8 {
                        let pixel_x = x + col;
                        let pixel_y = y + row as u32;
                        
                        if pixel_x < 640 && pixel_y < 480 {
                            let color = if (font_byte & (0x80 >> col)) != 0 {
                                self.fg_color // Foreground color for set bits
                            } else {
                                continue; // Don't draw background, leave existing pixels
                            };
                            
                            // Direct pixel write - same as graphics rectangles
                            let offset = (pixel_y * 640 + pixel_x) as usize;
                            *fb.buffer.add(offset) = color;
                        }
                    }
                }
                
                // Force flush to VirtIO GPU after drawing character
                if VIRTIO_GPU_ENABLED {
                    let _ = crate::virtio::flush_display();
                }
            }
        }
        Ok(())
    }
    
    fn print_char(&mut self, ch: char) -> Result<(), &'static str> {
        match ch {
            '\n' => {
                self.cursor_x = 0;
                self.cursor_y += 1;
                if self.cursor_y >= self.max_rows {
                    self.scroll_up();
                }
            }
            '\r' => {
                self.cursor_x = 0;
            }
            '\t' => {
                // Tab to next 8-character boundary
                self.cursor_x = (self.cursor_x + 8) & !7;
                if self.cursor_x >= self.max_cols {
                    self.cursor_x = 0;
                    self.cursor_y += 1;
                    if self.cursor_y >= self.max_rows {
                        self.scroll_up();
                    }
                }
            }
            _ => {
                if self.cursor_x >= self.max_cols {
                    self.cursor_x = 0;
                    self.cursor_y += 1;
                    if self.cursor_y >= self.max_rows {
                        self.scroll_up();
                    }
                }
                
                self.draw_char(ch, self.cursor_x * FONT_WIDTH, self.cursor_y * FONT_HEIGHT)?;
                self.cursor_x += 1;
            }
        }
        Ok(())
    }
    
    fn print_str(&mut self, s: &str) -> Result<(), &'static str> {
        for ch in s.chars() {
            self.print_char(ch)?;
        }
        Ok(())
    }
    
    fn scroll_up(&mut self) {
        // Move all text up by one line
        unsafe {
            if let Some(ref mut fb) = FRAMEBUFFER {
                // Copy each line up
                for y in 1..self.max_rows {
                    for x in 0..self.max_cols {
                        for row in 0..FONT_HEIGHT {
                            for col in 0..FONT_WIDTH {
                                let src_x = x * FONT_WIDTH + col;
                                let src_y = y * FONT_HEIGHT + row;
                                let dst_x = x * FONT_WIDTH + col;
                                let dst_y = (y - 1) * FONT_HEIGHT + row;
                                
                                if src_x < 640 && src_y < 480 && dst_x < 640 && dst_y < 480 {
                                    let offset_src = (src_y * 640 + src_x) as usize;
                                    let offset_dst = (dst_y * 640 + dst_x) as usize;
                                    
                                    let color = *fb.buffer.add(offset_src);
                                    *fb.buffer.add(offset_dst) = color;
                                }
                            }
                        }
                    }
                }
                
                // Clear the last line
                for x in 0..self.max_cols {
                    for row in 0..FONT_HEIGHT {
                        for col in 0..FONT_WIDTH {
                            let pixel_x = x * FONT_WIDTH + col;
                            let pixel_y = (self.max_rows - 1) * FONT_HEIGHT + row;
                            
                            if pixel_x < 640 && pixel_y < 480 {
                                let _ = fb.set_pixel(pixel_x, pixel_y, self.bg_color);
                            }
                        }
                    }
                }
            }
        }
        
        self.cursor_y = self.max_rows - 1;
    }
    
    fn clear_screen(&mut self) -> Result<(), &'static str> {
        self.cursor_x = 0;
        self.cursor_y = 0;
        
        unsafe {
            if let Some(ref mut fb) = FRAMEBUFFER {
                fb.clear(self.bg_color); // Clear to black background
                // Note: Don't flush here to avoid potential deadlock
            }
        }
        Ok(())
    }
}


/// Draw shell prompt to the framebuffer
pub fn draw_shell_prompt() -> Result<(), &'static str> {
    unsafe {
        if let Some(ref mut fb) = FRAMEBUFFER {
            // Clear the prompt area (bottom of terminal)
            fb.draw_rect(25, 420, 590, 20, 0x00000000)?; // Clear prompt line
            
            // Draw "elinOS> " prompt using simple pixel art
            let prompt_x = 30;
            let prompt_y = 425;
            
            // Draw "elinOS> " in green pixels (simple bitmap font simulation)
            // 'e'
            fb.draw_rect(prompt_x, prompt_y, 6, 1, 0x0000FF00)?;
            fb.draw_rect(prompt_x, prompt_y + 2, 6, 1, 0x0000FF00)?;
            fb.draw_rect(prompt_x, prompt_y + 4, 6, 1, 0x0000FF00)?;
            fb.draw_rect(prompt_x, prompt_y, 1, 5, 0x0000FF00)?;
            
            // 'l'
            let x = prompt_x + 8;
            fb.draw_rect(x, prompt_y, 1, 5, 0x0000FF00)?;
            fb.draw_rect(x, prompt_y + 4, 3, 1, 0x0000FF00)?;
            
            // 'i'
            let x = prompt_x + 12;
            fb.draw_rect(x, prompt_y, 1, 5, 0x0000FF00)?;
            fb.draw_rect(x, prompt_y - 1, 1, 1, 0x0000FF00)?;
            
            // 'n'
            let x = prompt_x + 16;
            fb.draw_rect(x, prompt_y + 1, 1, 4, 0x0000FF00)?;
            fb.draw_rect(x + 1, prompt_y, 3, 1, 0x0000FF00)?;
            fb.draw_rect(x + 4, prompt_y + 1, 1, 4, 0x0000FF00)?;
            
            // 'O'
            let x = prompt_x + 22;
            fb.draw_rect(x, prompt_y, 4, 1, 0x0000FF00)?;
            fb.draw_rect(x, prompt_y + 4, 4, 1, 0x0000FF00)?;
            fb.draw_rect(x, prompt_y + 1, 1, 3, 0x0000FF00)?;
            fb.draw_rect(x + 3, prompt_y + 1, 1, 3, 0x0000FF00)?;
            
            // 'S'
            let x = prompt_x + 28;
            fb.draw_rect(x, prompt_y, 4, 1, 0x0000FF00)?;
            fb.draw_rect(x, prompt_y + 2, 4, 1, 0x0000FF00)?;
            fb.draw_rect(x, prompt_y + 4, 4, 1, 0x0000FF00)?;
            fb.draw_rect(x, prompt_y + 1, 1, 1, 0x0000FF00)?;
            fb.draw_rect(x + 3, prompt_y + 3, 1, 1, 0x0000FF00)?;
            
            // '>' 
            let x = prompt_x + 34;
            fb.draw_rect(x, prompt_y + 1, 1, 1, 0x0000FF00)?;
            fb.draw_rect(x + 1, prompt_y + 2, 1, 1, 0x0000FF00)?;
            fb.draw_rect(x, prompt_y + 3, 1, 1, 0x0000FF00)?;
            
            // ' ' (space - just move cursor)
            
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

 