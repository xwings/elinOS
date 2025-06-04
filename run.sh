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

# Disk image configuration - Use RAW format with FAT32 (simpler than ext4)
DISK_IMAGE=${DISK_IMAGE:-"disk.raw"}
DISK_SIZE=${DISK_SIZE:-"64"}

# Create disk image if it doesn't exist
if [ ! -f "$DISK_IMAGE" ]; then
    echo "Creating FAT32 disk image: $DISK_IMAGE (size: $DISK_SIZE MB)"
    # 1. Create raw image
    dd if=/dev/zero of=$DISK_IMAGE bs=1M count=$DISK_SIZE
    # 2. Format with FAT32 (much simpler than ext4)
    mkfs.ext4 $DISK_IMAGE
    
    # 3. Mount and add some test files
    mkdir -p /tmp/elinOS_mount
    sudo mount -o loop $DISK_IMAGE /tmp/elinOS_mount
    
    # Add test files
    echo "Hello from elinOS, hello XiaoMa, Hello XiaoBai" | sudo tee /tmp/elinOS_mount/hello.txt
    echo "# elinOS README

This is a simple FAT32 filesystem on IDE disk.

## Features
- IDE interface (simplest disk interface)
- FAT32 filesystem (simple to implement)
- Real disk file reading

## Files
- hello.txt - Sample text file
- README.md - This file
" | sudo tee /tmp/elinOS_mount/README.md
    
    echo "This is a test file from the IDE disk." | sudo tee /tmp/elinOS_mount/test.txt
    
    sudo umount /tmp/elinOS_mount
    rmdir /tmp/elinOS_mount
    
    echo "FAT32 disk image created successfully"
else
    echo "Using existing disk image: $DISK_IMAGE"
fi

# Run with QEMU - specify memory size (128MB default, can be overridden with MEMORY env var)
MEMORY_SIZE=${MEMORY:-128M}
echo "Starting QEMU with ${MEMORY_SIZE} of memory and IDE disk ${DISK_IMAGE}..."

if [ -z "$DISPLAY" ]; then
    echo "Running in terminal mode (no graphics)"
    qemu-system-riscv64 \
        -machine virt \
        -nographic \
        -m ${MEMORY_SIZE} \
        -bios /usr/share/qemu/opensbi-riscv64-generic-fw_dynamic.bin \
        -kernel kernel.bin \
        -drive file=${DISK_IMAGE},format=raw,if=none,id=disk0 \
        -device virtio-blk-device,drive=disk0 \
        -d guest_errors,int,unimp \
        -D qemu.log 
elif [ "$DISPLAY" == "gtk" ]; then
    echo "Running in graphics mode (QEMU window)"
    qemu-system-riscv64 \
        -machine virt \
        -display gtk \
        -serial mon:vc \
        -m ${MEMORY_SIZE} \
        -bios /usr/share/qemu/opensbi-riscv64-generic-fw_dynamic.bin \
        -kernel kernel.bin \
        -drive file=${DISK_IMAGE},format=raw,if=none,id=disk0 \
        -device virtio-blk-device,drive=disk0 \
        -d guest_errors,int,unimp \
        -D qemu.log
fi