// Simple test program that doesn't use system calls
// Just returns a magic number to test ELF execution

// Entry point required by linker
int _start() {
    int result = main();
    // Return the result instead of infinite loop
    return result;
}

int main() {
    // Just do some simple computation and return
    // No system calls, no memory access issues
    int a = 42;
    int b = 24;
    int result = a + b; // Should be 66
    
    return result;
} 