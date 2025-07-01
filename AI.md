### Introductions
> **elinOS** is an experimental operating system kernel designed for research, learning, and exploring advanced memory management techniques. Built entirely in Rust for RISC-V architecture, it features dynamic hardware detection, sophisticated multi-tier memory allocators, real filesystem implementations, and a comprehensive Linux-compatible system call interface.

### Supported Platform
- It should work with qemu and real hardware. Bios will be using OpenSBI standard.

### Development Notes
1. All printout must use a internal standardized call
    - debug_println() and debug_print() - to print in UART
    - console_println() and console_print() - to print in UART and Framebuffer

### Build and test
1. To clean build and test the kernel: make test
2. make small changes, and run "make build && make test".  To make sure we are in a right track
3. Do not use these commands, the reason is the it will run in qemu and will not help in development
    - make run-console
    - make run-graphics

### Stage 1: Move library to comon area
1. Some library shared between bootloader (stage 2) and kernel. 
2. Need to move to a comon place (folder can named libaray/)
3. Move and test properly before we move to stage 2

### Stage 2: Boot from QEMU and Real Hardware
1. Currently elinOS only generate kernel binary. 
2. Kernel binary comes with bootloader and kernel. 
3. Seperate kernel and bootloader into two different binary.
