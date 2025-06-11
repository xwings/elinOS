// Simple Hello World for elinOS (fixed version)
// Uses direct system calls without libc

#define SYS_WRITE 64
#define STDOUT_FD 1

// Simple syscall wrapper for RISC-V
long syscall(long number, long arg1, long arg2, long arg3) {
    register long a7 asm("a7") = number;
    register long a0 asm("a0") = arg1;
    register long a1 asm("a1") = arg2;
    register long a2 asm("a2") = arg3;
    
    asm volatile ("ecall"
                  : "=r"(a0)
                  : "r"(a7), "r"(a0), "r"(a1), "r"(a2)
                  : "memory");
    return a0;
}

// Entry point required by linker
void _start() {
    main();
    // Exit syscall would go here in a real OS
    while(1) {} // Infinite loop for now
}

int main() {
    const char* message = "Hello, World from C!\n";
    
    // Calculate string length
    int len = 0;
    while (message[len] != '\0') len++;
    
    // Use SYS_WRITE system call
    syscall(SYS_WRITE, STDOUT_FD, (long)message, len);
    
    return 0;
} 