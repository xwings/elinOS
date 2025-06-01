#!/bin/bash

rm -rf qemu.log

# Check if kernel.bin exists
if [ ! -f "kernel.bin" ]; then
    echo "Error: kernel.bin not found. Run build.sh first."
    exit 1
fi

# Show kernel information
echo "Kernel information:"
file kernel.bin
ls -l kernel.bin

# Run with QEMU
qemu-system-riscv64 \
    -machine virt \
    -nographic \
    -bios /usr/share/qemu/opensbi-riscv64-generic-fw_dynamic.bin \
    -kernel kernel.bin \
    -serial mon:stdio \
    -d guest_errors,int,unimp,in_asm \
    -D qemu.log 