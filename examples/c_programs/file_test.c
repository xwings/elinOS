// File operations test for elinOS
// Tests reading files using elinOS file syscalls

#define SYS_WRITE 64
#define SYS_READ 63
#define SYS_OPENAT 56
#define SYS_CLOSE 57
#define STDOUT_FD 1
#define AT_FDCWD -100

// Simple syscall wrapper for RISC-V
long syscall(long number, long arg1, long arg2, long arg3, long arg4) {
    register long a7 asm("a7") = number;
    register long a0 asm("a0") = arg1;
    register long a1 asm("a1") = arg2;
    register long a2 asm("a2") = arg3;
    register long a3 asm("a3") = arg4;
    
    asm volatile ("ecall"
                  : "=r"(a0)
                  : "r"(a7), "r"(a0), "r"(a1), "r"(a2), "r"(a3)
                  : "memory");
    return a0;
}

void print_string(const char* str) {
    int len = 0;
    while (str[len] != '\0') len++;
    syscall(SYS_WRITE, STDOUT_FD, (long)str, len, 0);
}
    
int main() {
    print_string("File Test Program for elinOS\n");
    print_string("==============================\n");
    
    // Try to open a file (should exist in your filesystem)
    const char* filename = "test.txt";
    print_string("Attempting to open file: ");
    print_string(filename);
    print_string("\n");
    
    long fd = syscall(SYS_OPENAT, AT_FDCWD, (long)filename, 0, 0);
    
    if (fd < 0) {
        print_string("Error: Could not open file\n");
        return 1;
    }
    
    print_string("File opened successfully!\n");
    
    // Try to read from the file
    char buffer[256];
    long bytes_read = syscall(SYS_READ, fd, (long)buffer, 255, 0);
    
    if (bytes_read > 0) {
        buffer[bytes_read] = '\0';  // Null terminate
        print_string("File contents:\n");
        print_string(buffer);
        print_string("\n");
    } else {
        print_string("Could not read from file\n");
    }
    
    // Close the file
    syscall(SYS_CLOSE, fd, 0, 0, 0);
    print_string("File closed.\n");
    
    return 0;
} 


// Entry point required by linker - ensure it's at the start of text section
__attribute__((section(".text.start")))
void _start() {
    main();
    // Exit syscall would go here in a real OS
    while(1) {} // Infinite loop for now
}