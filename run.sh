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

# Run with QEMU - specify memory size (128MB default, can be overridden with MEMORY env var)
MEMORY_SIZE=${MEMORY:-128M}
echo "Starting QEMU with ${MEMORY_SIZE} of memory..."

qemu-system-riscv64 \
    -machine virt \
    -nographic \
    -m ${MEMORY_SIZE} \
    -bios /usr/share/qemu/opensbi-riscv64-generic-fw_dynamic.bin \
    -kernel kernel.bin \
    -serial mon:stdio \
    -d guest_errors,int,unimp,in_asm \
    -D qemu.log 