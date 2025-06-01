#!/bin/bash

# Clean the target directory
rm -rf target
cargo clean
rm -rf kernel.bin

# Build the kernel with release profile for better optimization
RUSTFLAGS="-C target-cpu=generic-rv64 -C target-feature=+m,+a,+c,+d,+f -C link-arg=-Tsrc/linker.ld" cargo build --release --target riscv64gc-unknown-none-elf

# Check if the ELF file exists
if [ ! -f "target/riscv64gc-unknown-none-elf/release/kernel" ]; then
    echo "Error: ELF file not found. Build failed."
    exit 1
fi

# Show information about the ELF file
echo "ELF file information:"
file target/riscv64gc-unknown-none-elf/release/kernel

# Show sections
echo "ELF sections:"
~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/x86_64-unknown-linux-gnu/bin/rust-objdump -h target/riscv64gc-unknown-none-elf/release/kernel

# Show the first few instructions
echo "First few instructions:"
~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/x86_64-unknown-linux-gnu/bin/rust-objdump -d target/riscv64gc-unknown-none-elf/release/kernel | head -n 20

# Create a bootable image with explicit flags
~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/x86_64-unknown-linux-gnu/bin/rust-objcopy \
    --strip-all \
    --set-section-flags .bss=alloc,load,contents \
    --set-section-flags .text=alloc,load,contents \
    --set-section-flags .rodata=alloc,load,contents \
    --set-section-flags .data=alloc,load,contents \
    -O binary \
    target/riscv64gc-unknown-none-elf/release/kernel \
    kernel.bin

# Check if kernel.bin exists
if [ ! -f "kernel.bin" ]; then
    echo "Error: kernel.bin not found. Build failed."
    exit 1
else
    echo "Kernel built successfully"
    echo "Binary file information:"
    file kernel.bin
    echo "Binary size:"
    ls -l kernel.bin
    echo "First 32 bytes of binary (hex):"
    hexdump -C -n 32 kernel.bin
fi
