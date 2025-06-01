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

# Disk image configuration
DISK_IMAGE=${DISK_IMAGE:-"disk.qcow2"}
DISK_SIZE=${DISK_SIZE:-"100M"}

# Create disk image if it doesn't exist
if [ ! -f "$DISK_IMAGE" ]; then
    echo "Creating disk image: $DISK_IMAGE (size: $DISK_SIZE)"
    qemu-img create -f qcow2 "$DISK_IMAGE" "$DISK_SIZE"
    echo "Disk image created successfully"
else
    echo "Using existing disk image: $DISK_IMAGE"
fi

# Run with QEMU - specify memory size (128MB default, can be overridden with MEMORY env var)
MEMORY_SIZE=${MEMORY:-128M}
echo "Starting QEMU with ${MEMORY_SIZE} of memory and disk ${DISK_IMAGE}..."

qemu-system-riscv64 \
    -machine virt \
    -nographic \
    -m ${MEMORY_SIZE} \
    -bios /usr/share/qemu/opensbi-riscv64-generic-fw_dynamic.bin \
    -kernel kernel.bin \
    -drive file=${DISK_IMAGE},format=qcow2,id=hd0,if=none \
    -device virtio-blk-device,drive=hd0 \
    -serial mon:stdio \
    -d guest_errors,int,unimp,in_asm \
    -D qemu.log 