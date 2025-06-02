# Debugging and Troubleshooting elinOS

This guide covers debugging techniques, common issues, and troubleshooting steps for elinOS development.

## Debugging Setup

### QEMU Logs
Debugging information is automatically logged to `qemu.log`:

```bash
# Monitor logs in real-time
tail -f qemu.log

# Search for specific issues
grep -i "error\|panic\|abort" qemu.log

# View last 50 lines
tail -50 qemu.log
```

### GDB Debugging
For kernel debugging with GDB:

```bash
# Start QEMU with GDB server
qemu-system-riscv64 \
    -machine virt \
    -cpu rv64 \
    -smp 1 \
    -m 128M \
    -serial stdio \
    -bios default \
    -kernel kernel.bin \
    -s -S  # GDB server on port 1234, wait for connection

# In another terminal, connect with GDB
riscv64-linux-gnu-gdb kernel.bin
(gdb) target remote :1234
(gdb) c  # Continue execution
```

### Serial Output
All kernel output goes through the serial console. Monitor for:
- Boot messages
- System call traces
- Error messages
- Panic information

## Common Issues

### Build Problems

#### Missing RISC-V Target
**Error:** `error[E0463]: can't find crate for 'core'`

**Solution:**
```bash
rustup target add riscv64gc-unknown-none-elf
```

#### Linker Errors
**Error:** `undefined reference to '_start'`

**Solution:** Check `src/linker.ld` and ensure proper entry point:
```ld
ENTRY(_start)
```

#### Cargo Build Fails
**Error:** Various compilation errors

**Solutions:**
```bash
# Clean and rebuild
cargo clean
./build.sh

# Check for missing dependencies
cargo check

# Update toolchain
rustup update
```

### Runtime Issues

#### QEMU Boot Failure
**Symptoms:** No output, immediate exit

**Troubleshooting:**
1. **Check QEMU installation:**
   ```bash
   qemu-system-riscv64 --version
   ```

2. **Verify kernel binary:**
   ```bash
   file kernel.bin
   # Should show: kernel.bin: data
   ```

3. **Check memory settings:**
   ```bash
   # Try with more memory
   MEMORY=256M ./run.sh
   ```

#### OpenSBI Issues
**Symptoms:** Boot stops at OpenSBI

**Solutions:**
1. **Check OpenSBI version:**
   - Ensure QEMU has compatible OpenSBI firmware
   - Try different QEMU versions

2. **Memory layout problems:**
   - Verify linker script addresses
   - Check for memory overlap

#### No Serial Output
**Symptoms:** QEMU starts but no text appears

**Solutions:**
1. **Check serial configuration:**
   ```bash
   # Ensure -serial stdio in run.sh
   grep "serial" run.sh
   ```

2. **Verify UART initialization:**
   - Check SBI UART setup in `src/main.rs`
   - Ensure proper character output

### Memory Issues

#### Stack Overflow
**Symptoms:** Random crashes, corruption

**Debugging:**
```rust
// Add stack canary checking
fn check_stack_integrity() {
    // Implementation to detect stack overflow
}
```

**Solutions:**
- Increase stack size in linker script
- Reduce local variable usage
- Use heap allocation for large data

#### Heap Exhaustion
**Symptoms:** Allocation failures, OOM

**Debugging:**
```bash
elinOS> memory
Memory regions:
  Region 0: 0x80000000 - 0x88000000 (128 MB) RAM
```

**Solutions:**
- Increase QEMU memory: `MEMORY=256M ./run.sh`
- Optimize memory usage
- Check for memory leaks

#### Memory Corruption
**Symptoms:** Random behavior, data corruption

**Debugging:**
1. **Enable memory debugging:**
   ```rust
   // Add bounds checking
   fn safe_memory_access(addr: usize) -> Result<u8, &'static str> {
       if addr < HEAP_START || addr > HEAP_END {
           return Err("Out of bounds access");
       }
       // Safe access
   }
   ```

2. **Use address sanitizer patterns:**
   ```rust
   // Poison freed memory
   fn debug_free(ptr: *mut u8, size: usize) {
       unsafe {
           ptr::write_bytes(ptr, 0xDE, size); // Poison pattern
       }
   }
   ```

### System Call Issues

#### Invalid System Call Numbers
**Error:** `Invalid system call number`

**Debugging:**
```bash
elinOS> syscall
# Check available system calls

elinOS> categories
# Verify call number ranges
```

**Solutions:**
- Verify syscall number ranges in documentation
- Check category boundaries
- Update user programs with correct numbers

#### Parameter Validation Errors
**Error:** System calls fail with parameter errors

**Debugging:**
1. **Add parameter logging:**
   ```rust
   pub fn handle_syscall(num: usize, args: &[usize]) -> SysCallResult {
       println!("Syscall {} with args: {:?}", num, args);
       // ... implementation
   }
   ```

2. **Validate in user space:**
   ```c
   // Check parameters before syscall
   if (filename == NULL || strlen(filename) == 0) {
       return -1;
   }
   ```

### VirtIO Device Issues

#### Device Not Found
**Symptoms:** `devices` command shows no devices

**Debugging:**
```bash
elinOS> devices
Probing for VirtIO devices...
# Should show VirtIO devices
```

**Solutions:**
1. **Check QEMU configuration:**
   ```bash
   # Ensure VirtIO disk is configured
   grep "virtio" run.sh
   ```

2. **Verify MMIO addresses:**
   ```rust
   // Check standard VirtIO MMIO addresses
   const VIRTIO_MMIO_BASE: usize = 0x10001000;
   ```

#### Block Device Initialization Fails
**Error:** VirtIO block device not responding

**Debugging:**
1. **Check device registers:**
   ```rust
   unsafe fn debug_virtio_device(base: usize) {
       let magic = ptr::read_volatile((base + 0x00) as *const u32);
       let version = ptr::read_volatile((base + 0x04) as *const u32);
       println!("VirtIO Magic: 0x{:x}, Version: {}", magic, version);
   }
   ```

2. **Verify queue setup:**
   ```rust
   fn debug_virtqueue(device: &VirtIOBlockDevice) {
       println!("Queue size: {}", device.queue_size);
       println!("Queue ready: {}", device.is_queue_ready());
   }
   ```

### ELF Loader Issues

#### Invalid ELF Files
**Error:** `Invalid ELF magic number`

**Debugging:**
```bash
# Check ELF file validity
file hello.elf
readelf -h hello.elf

# Verify magic number
hexdump -C hello.elf | head -1
# Should start with: 7f 45 4c 46
```

**Solutions:**
- Recompile with correct RISC-V toolchain
- Check compilation flags
- Verify file wasn't corrupted during transfer

#### Unsupported ELF Features
**Error:** `Unsupported ELF class/machine`

**Solutions:**
1. **Ensure RISC-V 64-bit:**
   ```bash
   readelf -h program.elf | grep Machine
   # Should show: Machine: RISC-V
   ```

2. **Check ELF class:**
   ```bash
   readelf -h program.elf | grep Class
   # Should show: Class: ELF64
   ```

### Filesystem Issues

#### File Not Found
**Error:** `File not found` errors

**Debugging:**
```bash
elinOS> ls
# Check available files

# Verify filename exactly
elinOS> cat "exact_filename.txt"
```

**Solutions:**
- Check file was added to `src/filesystem.rs`
- Verify filename case sensitivity
- Ensure file size limits aren't exceeded

#### Filesystem Full
**Error:** Cannot create new files

**Debugging:**
```rust
// Check filesystem capacity
const MAX_FILES: usize = 16;
const MAX_FILE_SIZE: usize = 4096;
```

**Solutions:**
- Increase `MAX_FILES` in filesystem.rs
- Increase `MAX_FILE_SIZE` for larger files
- Remove unnecessary files

## Performance Issues

### Slow Boot Time
**Symptoms:** Long time to reach shell prompt

**Profiling:**
1. **Add timing to boot stages:**
   ```rust
   fn boot_stage(name: &str) {
       let start = get_time();
       // ... stage implementation
       let end = get_time();
       println!("{} took {} cycles", name, end - start);
   }
   ```

2. **Optimize device probing:**
   ```rust
   // Cache device probe results
   static mut DEVICES_PROBED: bool = false;
   ```

### Memory Allocation Overhead
**Symptoms:** High memory usage

**Solutions:**
- Use `heapless` collections more extensively
- Reduce static buffer sizes
- Implement memory pooling

### System Call Overhead
**Symptoms:** Slow command execution

**Optimization:**
```rust
// Inline hot paths
#[inline]
pub fn fast_syscall_path(num: usize) -> SysCallResult {
    // Optimized implementation
}
```

## Development Debugging Techniques

### Adding Debug Prints
```rust
// Conditional compilation for debug builds
#[cfg(debug_assertions)]
macro_rules! debug_print {
    ($($arg:tt)*) => {
        println!("[DEBUG] {}", format_args!($($arg)*));
    };
}

#[cfg(not(debug_assertions))]
macro_rules! debug_print {
    ($($arg:tt)*) => {};
}
```

### Assert Macros
```rust
// Custom assert for kernel debugging
macro_rules! kernel_assert {
    ($cond:expr, $msg:expr) => {
        if !$cond {
            panic!("Kernel assertion failed: {} at {}:{}", 
                   $msg, file!(), line!());
        }
    };
}
```

### Memory Debugging
```rust
// Track memory allocations
struct MemoryTracker {
    allocations: usize,
    deallocations: usize,
}

impl MemoryTracker {
    fn track_alloc(&mut self, size: usize) {
        self.allocations += size;
        debug_print!("Allocated {} bytes, total: {}", size, self.allocations);
    }
}
```

## Testing Strategies

### Unit Testing
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_syscall_dispatch() {
        let result = handle_syscall(1, &[1, 0x1000, 10]);
        assert!(matches!(result, SysCallResult::Success(_)));
    }
}
```

### Integration Testing
```bash
# Script to test various scenarios
#!/bin/bash
echo "Testing elinOS functionality..."

# Test 1: Basic boot
timeout 30 ./run.sh << EOF
version
shutdown
EOF

# Test 2: File operations
timeout 30 ./run.sh << EOF
ls
cat hello.txt
shutdown
EOF
```

### Stress Testing
```rust
// Stress test system calls
fn stress_test_syscalls() {
    for i in 0..1000 {
        let result = handle_syscall(1, &[1, 0x1000, 10]);
        assert!(result.is_success());
    }
}
```

## Advanced Debugging

### Kernel Crash Analysis
When the kernel panics:

1. **Capture panic information:**
   ```rust
   #[panic_handler]
   fn panic(info: &PanicInfo) -> ! {
       println!("KERNEL PANIC: {}", info);
       println!("Location: {:?}", info.location());
       // Dump registers, stack trace, etc.
       loop {}
   }
   ```

2. **Analyze with GDB:**
   ```gdb
   (gdb) bt        # Stack trace
   (gdb) info reg  # Register dump
   (gdb) x/16x $sp # Stack contents
   ```

### Memory Layout Debugging
```rust
fn debug_memory_layout() {
    extern "C" {
        static __text_start: u8;
        static __text_end: u8;
        static __data_start: u8;
        static __data_end: u8;
    }
    
    unsafe {
        println!("Text: 0x{:x} - 0x{:x}", 
                 &__text_start as *const _ as usize,
                 &__text_end as *const _ as usize);
        println!("Data: 0x{:x} - 0x{:x}",
                 &__data_start as *const _ as usize,
                 &__data_end as *const _ as usize);
    }
}
```

## Recovery Procedures

### Soft Reset
If the system becomes unresponsive:
```bash
# Force QEMU exit
Ctrl+A, X

# Or from another terminal
pkill qemu-system-riscv64
```

### Hard Reset
For complete recovery:
```bash
# Clean all generated files
make clean  # or rm -f kernel.bin qemu.log disk.qcow2

# Rebuild from scratch
./build.sh
./run.sh
```

### Backup Procedures
```bash
# Backup working configuration
cp -r src/ src.backup/
cp Cargo.toml Cargo.toml.backup

# Create development snapshot
git add -A
git commit -m "Working state before changes"
```

This debugging guide should help you identify and resolve most issues encountered during elinOS development and usage. 