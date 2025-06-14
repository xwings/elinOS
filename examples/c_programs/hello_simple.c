// Simple Hello World without system calls
// Just returns a magic value to prove it ran


int main() {
    // Instead of printing, just do some computation 
    // that proves the program ran correctly
    
    // Calculate a recognizable result
    int magic = 0x48454C4C; // "HELL" in ASCII
    int world = 0x4F4F4F4F; // "OOOO" in ASCII
    
    // Simple computation so we know it executed
    int result = (magic >> 16) + (world & 0xFFFF); // Should be 0x4845 + 0x4F4F = 0x9794
    
    return result; // Return 0x9794 (38804 decimal) 
} 

// Entry point required by linker - ensure it's at the start of text section
__attribute__((section(".text.start")))
int _start() {
    int result = main();
    return result;
}