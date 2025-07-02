### Introductions
> **elinOS** is an experimental operating system kernel designed for research, learning, and exploring advanced memory management techniques. Built entirely in Rust for RISC-V architecture, it features dynamic hardware detection, sophisticated multi-tier memory allocators, real filesystem implementations, and a comprehensive Linux-compatible system call interface.

### Supported Platform
- It should work with qemu and real hardware. Bios will be using OpenSBI standard.

### Development Notes
1. All printout must use a internal standardized call
    - debug_println() and debug_print() - to print in UART
    - console_println() and console_print() - to print in UART and Framebuffer
2. Always follow "no news is good news" principle, no extra prinout 

### Build and test
1. To clean build and test the kernel: make test
2. make small changes, and run "make build && make test".  To make sure we are in a right track
3. DO NOT use these commands, it will elinOS run in qemu and will not help in development and testing
    - make run-console
    - make run-graphics
4. Only use ""make test", it will test the command make sure everything runs well.

### Stage 1: Move library to comon area
1. Boot loader is not ready yet
2. Some library shared between bootloader (stage 2) and kernel. 
3. Need to move to a comon place (folder can named libaray/)
4. Move and test properly before we move to stage 2

### Stage 2: Boot from QEMU and Real Hardware
1. Currently elinOS only generate kernel binary. 
2. Kernel binary comes with bootloader and kernel. 
3. Seperate kernel and bootloader into two different binary.
