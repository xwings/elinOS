### Introduction
> **elinOS** is an experimental operating system kernel designed for research, learning, and exploring advanced memory management techniques. Built entirely in Rust for RISC-V architecture, it features dynamic hardware detection, sophisticated multi-tier memory allocators, real filesystem implementations, and a comprehensive Linux-compatible system call interface.

### Supported Platform
- It should work with qemu and real hardware. Bios will be using OpenSBI standard.

### Development Notes
1. All printout must use an internal standardized call
    - debug_println() and debug_print() - to print in UART
    - console_println() and console_print() - to print in UART and Framebuffer
2. Always follow "no news is good news" principle, no extra printout 

### Build and test
1. To clean build and test the kernel: make test
2. Make small changes, and run "make build && make test". To make sure we are on the right track
3. DO NOT use these commands, they will run elinOS in qemu and will not help in development and testing
    - make run-console
    - make run-graphics
4. Only use "make test", it will test the command and make sure everything runs well.

### Successful test
1. Run "make build" without errors
2. Run "timeout 60 make run-console-debug" and be able to see elinOS>
3. Pass all test in "make test" or "make test-fb"

### Boot stage - boot.bin
1. QEMU load OpenSBI
2. OpenSBI load bootloader (boot.bin)
3. Bootloader (boot.bin) runs kernel
4. After kernel boot, it will show interactive shell "elinOS>" 
6. MUST run "make test", it will test the command make sure everything runs well.
7. Must support both qemu and real hardware.
8. What do we need to have in boot.bin, after qemu/hardware load boot.bin
    - Add storage support - Implement VirtIO block device driver
    - Add ext2 support - Read ext2 filesystem to find /kernel
    - Load from /kernel - Read kernel binary from SD card instead of memory

### How to run and test in QEMU
- make run-console : interactive shell with console (uart) tty
- make run-console-debug : interactive shell with console (uart) tty, with qemu.log
- make test-console : unittest with interactive shell with console (uart) tty, make sure 100% passing rate

### Work in progress
- make test-fb : test console with frame buffer, make sure 100% passing rate
- make run-fb : interactive shell in both console (uart) and framebuffer tty
- make run-fb-debug : interactive shell in both console (uart) and framebuffer tty, with qemu.log