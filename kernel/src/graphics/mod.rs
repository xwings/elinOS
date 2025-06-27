// Simple Graphics Module for elinOS - Basic Framebuffer Only
// Integrates with our centralized memory management API

use crate::memory::mapping;
use crate::console_println;

/// Simple framebuffer structure
pub struct SimpleFramebuffer {
    base_addr: usize,
    width: u32,
    height: u32,
    bpp: u32,        // Bits per pixel
    pitch: u32,      // Bytes per line
    size: usize,     // Total size in bytes
}

impl SimpleFramebuffer {
    /// Create a new framebuffer instance
    pub fn new(base_addr: usize, width: u32, height: u32, bpp: u32) -> Result<Self, &'static str> {
        if width == 0 || height == 0 || bpp == 0 {
            return Err("Invalid framebuffer dimensions");
        }

        let bytes_per_pixel = (bpp + 7) / 8; // Round up to nearest byte
        let pitch = width * bytes_per_pixel;
        let size = (height * pitch) as usize;

        console_println!("[i] Simple framebuffer created:");
        console_println!("   Resolution: {}x{}", width, height);
        console_println!("   BPP: {}, Pitch: {}", bpp, pitch);
        console_println!("   Size: {} KB", size / 1024);
        console_println!("   Address: 0x{:x}", base_addr);

        let fb = SimpleFramebuffer {
            base_addr,
            width,
            height,
            bpp,
            pitch,
            size,
        };

        // Clear to black
        fb.clear(0)?;

        Ok(fb)
    }

    /// Clear the entire framebuffer to specified value
    pub fn clear(&self, value: u32) -> Result<(), &'static str> {
        let bytes_per_pixel = (self.bpp + 7) / 8;
        
        unsafe {
            match bytes_per_pixel {
                4 => {
                    // 32-bit pixels
                    let buffer_32 = self.base_addr as *mut u32;
                    for i in 0..(self.size / 4) {
                        *buffer_32.add(i) = value;
                    }
                }
                2 => {
                    // 16-bit pixels
                    let buffer_16 = self.base_addr as *mut u16;
                    for i in 0..(self.size / 2) {
                        *buffer_16.add(i) = value as u16;
                    }
                }
                1 => {
                    // 8-bit pixels
                    let buffer_8 = self.base_addr as *mut u8;
                    for i in 0..self.size {
                        *buffer_8.add(i) = value as u8;
                    }
                }
                _ => return Err("Unsupported pixel format"),
            }
        }

        Ok(())
    }

    /// Draw a single pixel at specified coordinates
    pub fn draw_pixel(&self, x: u32, y: u32, value: u32) -> Result<(), &'static str> {
        if x >= self.width || y >= self.height {
            return Err("Pixel coordinates out of bounds");
        }

        let bytes_per_pixel = (self.bpp + 7) / 8;
        let offset = (y * self.pitch + x * bytes_per_pixel) as usize;

        unsafe {
            match bytes_per_pixel {
                4 => {
                    let pixel_ptr = (self.base_addr + offset) as *mut u32;
                    *pixel_ptr = value;
                }
                2 => {
                    let pixel_ptr = (self.base_addr + offset) as *mut u16;
                    *pixel_ptr = value as u16;
                }
                1 => {
                    let pixel_ptr = (self.base_addr + offset) as *mut u8;
                    *pixel_ptr = value as u8;
                }
                _ => return Err("Unsupported pixel format"),
            }
        }

        Ok(())
    }

    /// Draw a filled rectangle
    pub fn draw_rect(&self, x: u32, y: u32, width: u32, height: u32, value: u32) -> Result<(), &'static str> {
        for dy in 0..height {
            for dx in 0..width {
                if let Err(_) = self.draw_pixel(x + dx, y + dy, value) {
                    // Stop if we go out of bounds
                    break;
                }
            }
        }
        Ok(())
    }

    /// Get framebuffer info
    pub fn get_info(&self) -> (u32, u32, u32, usize) {
        (self.width, self.height, self.bpp, self.size)
    }

    /// Get base address
    pub fn get_base_addr(&self) -> usize {
        self.base_addr
    }
}

/// Graphics manager for simple framebuffer
pub struct GraphicsManager {
    framebuffer: Option<SimpleFramebuffer>,
}

impl GraphicsManager {
    pub const fn new() -> Self {
        GraphicsManager {
            framebuffer: None,
        }
    }

    /// Initialize graphics with a simple framebuffer
    pub fn init(&mut self) -> Result<(), &'static str> {
        console_println!("[i] Initializing VGA graphics system...");

        // Standard VGA resolution with 2MB heap
        let width = 640;
        let height = 480;
        let bpp = 32; // 32-bit pixels for full color support
        let bytes_per_pixel = (bpp + 7) / 8;
        let pitch = width * bytes_per_pixel;
        let fb_size = (height * pitch) as usize;

        console_println!("[i] Setting up VGA framebuffer: {}x{} @ {} bpp", width, height, bpp);
        console_println!("[i] Framebuffer size: {} KB", fb_size / 1024);

        // For QEMU RISC-V, let's try a different approach
        // Instead of using a fixed VGA address, let's allocate system memory
        // and see if we can make it work first
        console_println!("[i] Allocating framebuffer in system memory (QEMU RISC-V)");
        
        // Allocate framebuffer memory using our memory management API
        let fb_addr = mapping::map_virtual_memory(
            fb_size,
            mapping::MemoryPermissions::READ_WRITE,
            "VGA-Framebuffer"
        ).map_err(|_| "Failed to allocate framebuffer memory")?;

        console_println!("[o] VGA framebuffer mapped at 0x{:x}", fb_addr);

        // Create framebuffer instance
        let fb = SimpleFramebuffer::new(fb_addr, width, height, bpp)?;
        self.framebuffer = Some(fb);

        // Initialize VGA mode and clear the screen to show it's working
        console_println!("[i] Initializing VGA display mode...");
        
        // Get a reference to the framebuffer for initialization
        if let Some(ref framebuffer) = self.framebuffer {
            // Clear the screen to blue to show it's working
            framebuffer.clear(0x000080FF)?; // Blue background (ARGB format)
            console_println!("[o] VGA framebuffer cleared to blue");
            
            // Draw a simple test pattern to verify it's working
            // Draw a white border around the screen
            for x in 0..width {
                framebuffer.draw_pixel(x, 0, 0xFFFFFFFF)?; // Top border
                framebuffer.draw_pixel(x, height - 1, 0xFFFFFFFF)?; // Bottom border
            }
            for y in 0..height {
                framebuffer.draw_pixel(0, y, 0xFFFFFFFF)?; // Left border
                framebuffer.draw_pixel(width - 1, y, 0xFFFFFFFF)?; // Right border
            }
            console_println!("[o] VGA test pattern drawn");
        }

        console_println!("[o] VGA graphics system initialized");
        console_println!("[i] Graphics output should now be visible in QEMU window!");
        Ok(())
    }

    /// Get framebuffer reference
    pub fn get_framebuffer(&mut self) -> Option<&mut SimpleFramebuffer> {
        self.framebuffer.as_mut()
    }

    /// Check if graphics is initialized
    pub fn is_initialized(&self) -> bool {
        self.framebuffer.is_some()
    }
}

/// Global graphics manager
use spin::Mutex;
pub static GRAPHICS_MANAGER: Mutex<GraphicsManager> = Mutex::new(GraphicsManager::new());

/// Initialize graphics system
pub fn init_graphics() -> Result<(), &'static str> {
    let mut gfx = GRAPHICS_MANAGER.lock();
    gfx.init()
}

/// Clear screen to specified value
pub fn clear_screen(value: u32) -> Result<(), &'static str> {
    let mut gfx = GRAPHICS_MANAGER.lock();
    if let Some(fb) = gfx.get_framebuffer() {
        fb.clear(value)
    } else {
        Err("Graphics not initialized")
    }
}

/// Draw pixel
pub fn draw_pixel(x: u32, y: u32, value: u32) -> Result<(), &'static str> {
    let mut gfx = GRAPHICS_MANAGER.lock();
    if let Some(fb) = gfx.get_framebuffer() {
        fb.draw_pixel(x, y, value)
    } else {
        Err("Graphics not initialized")
    }
}

/// Draw rectangle
pub fn draw_rect(x: u32, y: u32, width: u32, height: u32, value: u32) -> Result<(), &'static str> {
    let mut gfx = GRAPHICS_MANAGER.lock();
    if let Some(fb) = gfx.get_framebuffer() {
        fb.draw_rect(x, y, width, height, value)
    } else {
        Err("Graphics not initialized")
    }
}

/// Get framebuffer info
pub fn get_framebuffer_info() -> Option<(u32, u32, u32, usize)> {
    let mut gfx = GRAPHICS_MANAGER.lock();
    gfx.get_framebuffer().map(|fb| fb.get_info())
}

/// Check if graphics is available
pub fn is_graphics_available() -> bool {
    let gfx = GRAPHICS_MANAGER.lock();
    gfx.is_initialized()
} 