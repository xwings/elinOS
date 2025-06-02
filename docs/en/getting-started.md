# Getting Started with elinOS

This guide covers setting up, building, and running elinOS from source.

## Prerequisites

- **Rust toolchain** with `riscv64gc-unknown-none-elf` target
- **QEMU** with RISC-V support (`qemu-system-riscv64`)
- **mkfs.fat** for disk image formatting (optional)

### Installing Prerequisites

#### Rust Toolchain
```bash
# Install Rust if not already installed
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Add RISC-V target
rustup target add riscv64gc-unknown-none-elf
```

#### QEMU Installation

**Ubuntu/Debian:**
```bash
sudo apt update
sudo apt install qemu-system-misc
```

**Arch Linux:**
```bash
sudo pacman -S qemu-arch-extra
```

**macOS:**
```bash
brew install qemu
```

## Building the Kernel

Build elinOS using the provided script:

```bash
./build.sh
```

This script:
1. Builds the kernel using `cargo build --release`
2. Strips the ELF binary to create `kernel.bin`
3. Sets up the target for RISC-V 64-bit

## Running with QEMU

### Basic Usage

```bash
# Run with default configuration (128MB RAM, 100MB disk)
./run.sh
```

### Advanced Configuration

```bash
# Run with custom memory size
MEMORY=256M ./run.sh

# Run with custom disk size
DISK_SIZE=500M ./run.sh

# Run with custom disk image
DISK_IMAGE=my_custom_disk.qcow2 ./run.sh

# Combine multiple options
MEMORY=512M DISK_SIZE=1G ./run.sh
```

### QEMU Parameters Explained

The `run.sh` script uses these QEMU parameters:
- `-machine virt`: Use QEMU's generic RISC-V virtual machine
- `-cpu rv64`: 64-bit RISC-V CPU
- `-smp 1`: Single CPU core
- `-m 128M`: Memory size (configurable)
- `-serial stdio`: Connect serial port to terminal
- `-bios default`: Use OpenSBI firmware
- `-kernel kernel.bin`: Load our kernel
- `-drive file=disk.qcow2,if=virtio,format=qcow2`: VirtIO block device

## Configuration Options

### Memory Configuration

The system automatically detects available memory, but you can override defaults:

```bash
# Set specific memory sizes
MEMORY=64M ./run.sh     # Minimal configuration
MEMORY=256M ./run.sh    # Standard configuration
MEMORY=1G ./run.sh      # Large configuration
```

### Disk Configuration

Disk images are automatically created and formatted:

```bash
# Create different sized disks
DISK_SIZE=50M ./run.sh   # Minimal disk
DISK_SIZE=500M ./run.sh  # Standard disk
DISK_SIZE=2G ./run.sh    # Large disk

# Use existing disk image
DISK_IMAGE=existing.qcow2 ./run.sh
```

### Build Configuration

Edit `Cargo.toml` to modify dependencies or `src/linker.ld` for memory layout adjustments.

## Troubleshooting

### Build Issues

**Missing RISC-V target:**
```bash
rustup target add riscv64gc-unknown-none-elf
```

**Cargo build fails:**
```bash
# Clean and rebuild
cargo clean
./build.sh
```

### Runtime Issues

**QEMU not found:**
- Install QEMU as shown above
- Ensure `qemu-system-riscv64` is in PATH

**Boot fails:**
- Check that OpenSBI firmware is available
- Verify memory settings aren't too low (minimum 64MB recommended)

**No output:**
- Ensure serial console is properly connected
- Try adding `-nographic` to QEMU options in `run.sh`

### Performance Tips

**Faster builds:**
```bash
# Use parallel compilation
export CARGO_BUILD_JOBS=4
./build.sh
```

**QEMU acceleration:**
```bash
# Add to run.sh for better performance (if supported)
-enable-kvm  # On Linux with KVM
-accel hvf   # On macOS with Hypervisor Framework
```

## Development Workflow

1. **Edit source code** in `src/`
2. **Build kernel** with `./build.sh`
3. **Test in QEMU** with `./run.sh`
4. **Check logs** in `qemu.log` for debugging
5. **Iterate** and repeat

## Next Steps

- See [Commands](commands.md) for using the elinOS shell
- See [Development](development.md) for creating user programs
- See [Architecture](architecture.md) for technical details 