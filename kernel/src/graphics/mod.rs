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
    
    // Try virtual address first (as shown in Stephen Marz working example)
    console_println!("[i] Trying VirtIO GPU with virtual address...");
    match crate::virtio::init_virtio_gpu(fb_virt_addr, fb_size) {
        Ok(()) => {
            console_println!("[o] VirtIO GPU initialized successfully with virtual address!");
            console_println!("[i] Graphics output should now be visible in QEMU window!");
            unsafe { VIRTIO_GPU_ENABLED = true; }
        }
        Err(_) => {
            console_println!("[!] Virtual address failed, trying physical address...");
            // Fallback to physical address
            match crate::virtio::init_virtio_gpu(fb_phys_addr, fb_size) {
                Ok(()) => {
                    console_println!("[o] VirtIO GPU initialized successfully with physical address!");
                    console_println!("[i] Graphics output should now be visible in QEMU window!");
                    unsafe { VIRTIO_GPU_ENABLED = true; }
                }
                Err(_) => {
                    console_println!("[!] VirtIO GPU not available - using software framebuffer only");
                    unsafe { VIRTIO_GPU_ENABLED = false; }
                }
            }
        }
    }
    
    // Initialize display mode
    console_println!("[i] Initializing VGA display mode...");
    
    // Clear to bright red background to make it very obvious (BGRA format: 0xBBGGRRAA)
    framebuffer.clear(0x0000FFFF); // Bright red in BGRA format
    console_println!("[o] VGA framebuffer cleared to bright red (BGRA)");
    
    // Fill entire framebuffer with different patterns to test
    console_println!("[i] Testing different color patterns...");
    
    // Test 1: Fill entire screen with solid white (should be visible in any format)
    unsafe {
        let fb_ptr = framebuffer.buffer;
        let total_pixels = (640 * 480) as isize;
        for i in 0..total_pixels {
            *fb_ptr.offset(i) = 0xFFFFFFFF; // Solid white - should work in any format
        }
    }
    console_println!("[o] Filled framebuffer with solid white");
    
    // Draw test pattern - bright colors in BGRA format
    framebuffer.draw_rect(50, 50, 100, 100, 0x00FF00FF)?; // Bright green square (same in BGRA)
    framebuffer.draw_rect(200, 200, 200, 100, 0x00FFFFFF)?; // Bright yellow rectangle
    framebuffer.draw_rect(0, 0, 640, 10, 0xFFFFFFFF)?; // White top border (same in BGRA)
    framebuffer.draw_rect(0, 470, 640, 10, 0xFFFFFFFF)?; // White bottom border (same in BGRA)
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
    
    // Initialize text console for shell display
    console_println!("[i] Initializing text console for shell display...");
    match init_text_console() {
        Ok(()) => {
            console_println!("[o] Text console initialized - shell should now be visible in QEMU window!");
            
            // Test the graphics console directly
            match print_to_console("TEST: Graphics console is working!\n") {
                Ok(()) => console_println!("[o] Graphics console test successful"),
                Err(e) => console_println!("[!] Graphics console test failed: {}", e),
            }
            
            // Test a simple shell prompt
            match print_to_console("elinOS> ") {
                Ok(()) => console_println!("[o] Shell prompt test successful"),
                Err(e) => console_println!("[!] Shell prompt test failed: {}", e),
            }
        }
        Err(e) => {
            console_println!("[!] Failed to initialize text console: {}", e);
        }
    }
    
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
    unsafe {
        TEXT_CONSOLE = Some(TextConsole::new());
        
        // Clear screen to black and display welcome message
        if let Some(ref mut console) = TEXT_CONSOLE {
            console.clear_screen()?;
            console.print_str("elinOS Graphics Console\n")?;
            console.print_str("======================\n\n")?;
            
            // Flush to display
            if VIRTIO_GPU_ENABLED {
                let _ = crate::virtio::flush_display();
            }
        }
    }
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
            fg_color: 0xFFFFFFFF,         // White text
            bg_color: 0x000000FF,         // Black background (BGRA)
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
                                self.bg_color // Background color for unset bits
                            };
                            
                            let _ = fb.set_pixel(pixel_x, pixel_y, color);
                        }
                    }
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
                fb.clear(self.bg_color);
            }
        }
        Ok(())
    }
} 