#include <stddef.h>

// System call numbers (must match kernel definitions)
#define SYS_WRITE   64
#define SYS_EXIT    93
#define SYS_GETPID  172
#define SYS_GETPPID 173
#define SYS_FORK    220
#define SYS_WAIT4   260

// Simple syscall wrapper
static inline long syscall(long num, long arg1, long arg2, long arg3, long arg4) {
    register long a7 asm("a7") = num;
    register long a0 asm("a0") = arg1;
    register long a1 asm("a1") = arg2;
    register long a2 asm("a2") = arg3;
    register long a3 asm("a3") = arg4;
    
    asm volatile ("ecall" : "+r"(a0) : "r"(a7), "r"(a1), "r"(a2), "r"(a3) : "memory");
    return a0;
}

// Helper functions
void print(const char* str) {
    size_t len = 0;
    while (str[len]) len++;  // Calculate length
    syscall(SYS_WRITE, 1, (long)str, len, 0);
}

void print_number(int num) {
    char buf[20];
    int i = 0;
    
    if (num == 0) {
        buf[i++] = '0';
    } else {
        // Convert number to string (simple implementation)
        int temp = num;
        int digits = 0;
        while (temp > 0) {
            temp /= 10;
            digits++;
        }
        
        for (int j = digits - 1; j >= 0; j--) {
            buf[j] = '0' + (num % 10);
            num /= 10;
        }
        i = digits;
    }
    
    buf[i] = '\0';
    print(buf);
}

int getpid() {
    return syscall(SYS_GETPID, 0, 0, 0, 0);
}

int getppid() {
    return syscall(SYS_GETPPID, 0, 0, 0, 0);
}

int fork() {
    return syscall(SYS_FORK, 0, 0, 0, 0);
}

int wait4(int pid, int* status, int options) {
    return syscall(SYS_WAIT4, pid, (long)status, options, 0);
}

void exit(int code) {
    syscall(SYS_EXIT, code, 0, 0, 0);
}

int main() {
    print("=== elinOS Fork Test ===\n");
    
    print("Initial process PID: ");
    print_number(getpid());
    print("\n");
    
    print("Initial process PPID: ");
    print_number(getppid());
    print("\n");
    
    print("About to fork...\n");
    
    int child_pid = fork();
    
    if (child_pid == 0) {
        // Child process
        print("CHILD: I am the child process!\n");
        print("CHILD: My PID is: ");
        print_number(getpid());
        print("\n");
        print("CHILD: My parent PID is: ");
        print_number(getppid());
        print("\n");
        print("CHILD: Exiting with code 42\n");
        exit(42);
    } else if (child_pid > 0) {
        // Parent process
        print("PARENT: Fork successful! Child PID is: ");
        print_number(child_pid);
        print("\n");
        print("PARENT: My PID is: ");
        print_number(getpid());
        print("\n");
        print("PARENT: Waiting for child to exit...\n");
        
        int status = 0;
        int waited_pid = wait4(-1, &status, 0);
        
        if (waited_pid > 0) {
            print("PARENT: Child ");
            print_number(waited_pid);
            print(" exited with status: ");
            print_number(status);
            print("\n");
        } else {
            print("PARENT: Wait failed or no children\n");
        }
        
        print("PARENT: All done!\n");
    } else {
        // Fork failed
        print("ERROR: Fork failed!\n");
        exit(1);
    }
    
    exit(0);
} 