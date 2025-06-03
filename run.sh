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

# Disk image configuration - Use RAW format to avoid encryption
DISK_IMAGE=${DISK_IMAGE:-"disk.raw"}
DISK_SIZE=${DISK_SIZE:-"512"}

# Create disk image if it doesn't exist
if [ ! -f "$DISK_IMAGE" ]; then
    echo "Creating raw disk image: $DISK_IMAGE (size: $DISK_SIZE MB)"
    # 1. Create raw image
    dd if=/dev/zero of=$DISK_IMAGE bs=1M count=$DISK_SIZE
    # 2. Format raw image with ext4
    mkfs.ext4 $DISK_IMAGE
    echo "Raw disk image created successfully"
else
    echo "Using existing disk image: $DISK_IMAGE"
fi

# Mount and add coreutils:
# sudo mount -o loop rootfs.qcow2 /mnt
# sudo mkdir -p /mnt/bin
#sudo cp /bin/{ls,cat,cp,mv,mkdir,rm,touch} /mnt/bin/
#sudo umount /mnt

# Run with QEMU - specify memory size (128MB default, can be overridden with MEMORY env var)
MEMORY_SIZE=${MEMORY:-128M}
echo "Starting QEMU with ${MEMORY_SIZE} of memory and SCSI disk ${DISK_IMAGE}..."


if [ -z "$DISPLAY" ]; then
    echo "Running in terminal mode (no graphics)"
    qemu-system-riscv64 \
        -machine virt \
        -nographic \
        -m ${MEMORY_SIZE} \
        -bios /usr/share/qemu/opensbi-riscv64-generic-fw_dynamic.bin \
        -kernel kernel.bin \
        -drive file=${DISK_IMAGE},format=raw,id=hd0,if=none \
        -device virtio-blk-device,drive=hd0 \
        -d guest_errors,int,unimp,trace:virtio*,trace:dma_* \
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
        -drive file=${DISK_IMAGE},format=raw,id=hd0,if=none \
        -device virtio-blk-device,drive=hd0 \
        -d guest_errors,int,unimp,trace:virtio*,trace:dma_* \
        -D qemu.log
fi