# Debugging and Troubleshooting elinOS

This guide covers debugging techniques, common issues, and troubleshooting steps for elinOS development.

## Debugging Setup

### QEMU Logs
When using the `run-debug` target in the Makefile, debugging information from QEMU is logged to `qemu.log` (due to `-D qemu.log` flag). You can also add `-d guest_errors,int,unimp` or other flags to QEMU_ARGS in Makefile for more detailed logs.

```bash
# Monitor logs in real-time
tail -f qemu.log

# Search for specific issues
grep -i "error\\|panic\\|abort" qemu.log

# View last 50 lines
tail -50 qemu.log
```

### GDB Debugging
For kernel debugging with GDB:

1.  **Start QEMU with GDB server using the Makefile:**
    The `run-debug` target in the `Makefile` starts QEMU with the GDB server enabled (`-s -S`), waiting for a connection on port 1234.
    ```bash
    make run-debug
    ```
    QEMU will print a message like: `Connect with: gdb target/riscv64gc-unknown-none-elf/debug/kernel -ex 'target remote :1234'`

2.  **In another terminal, connect with GDB:**
    Use the GDB command provided by the `make run-debug` output. This will typically be:
    ```bash
    # Make sure you have a RISC-V GDB, e.g., gdb-multiarch or riscv64-unknown-elf-gdb
    gdb-multiarch target/riscv64gc-unknown-none-elf/debug/kernel
    # or riscv64-unknown-elf-gdb target/riscv64gc-unknown-none-elf/debug/kernel
    ```
    Then, within GDB:
    ```gdb
    (gdb) target remote :1234
    (gdb) # Set breakpoints, e.g., break kmain
    (gdb) c  # Continue execution
    ```
    The kernel ELF file (`target/riscv64gc-unknown-none-elf/debug/kernel`) contains symbols needed for effective debugging.

### Serial Output
All kernel output (`console_println!`) goes through the serial console. Monitor for:
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
**Error:** `undefined reference to '_start'` (or similar)

**Solution:** Check `src/linker.ld` and ensure proper entry point and section layout:
```ld
ENTRY(_start)
/* ... other sections ... */
```

#### Cargo Build Fails
**Error:** Various compilation errors

**Solutions:**
```bash
# Clean and rebuild
make clean
make build # or just 'make'

# Check for missing dependencies or other errors
cargo check --target riscv64gc-unknown-none-elf

# Update toolchain
rustup update
```

### Runtime Issues

#### QEMU Boot Failure
**Symptoms:** No output, immediate QEMU exit, or errors from QEMU itself.

**Troubleshooting:**
1.  **Check QEMU installation:**
    ```bash
    qemu-system-riscv64 --version
    ```
2.  **Verify kernel ELF file:**
    The kernel file used is typically `target/riscv64gc-unknown-none-elf/debug/kernel`.
    ```bash
    file target/riscv64gc-unknown-none-elf/debug/kernel
    # Should show: ... ELF 64-bit LSB executable, RISC-V, ...
    ```
3.  **Check memory settings in `Makefile`:**
    The `QEMU_MEMORY` variable in the `Makefile` (e.g., `128M`). Try increasing if needed.
4.  **OpenSBI:**
    Ensure QEMU can find/use OpenSBI (the `Makefile` attempts to find it). If QEMU complains about BIOS, this could be an issue.

#### OpenSBI Issues
**Symptoms:** Boot stops at OpenSBI, or OpenSBI prints errors.

**Solutions:**
1.  **Check OpenSBI version/path:**
    - The `Makefile` tries to locate OpenSBI. Ensure the path it finds is correct or QEMU's default is working.
    - Try different QEMU versions if OpenSBI compatibility is suspected.
2.  **Memory layout problems:**
    - Verify linker script addresses against OpenSBI's expectations for kernel load address.
    - Check for memory overlap.

#### No Serial Output
**Symptoms:** QEMU starts but no kernel text appears after OpenSBI.

**Solutions:**
1.  **Check serial configuration in `Makefile`:**
    - `run` and `run-debug` targets typically use `-nographic` which directs serial to stdio.
    - `run-graphics` uses `-serial mon:vc`.
2.  **Verify UART initialization in kernel:**
    - Check early UART setup in `src/main.rs` or `src/uart.rs`.
    - Ensure `console_println!` or direct UART writes are functioning.

### Memory Issues

#### Stack Overflow
**Symptoms:** Random crashes, corruption, unexpected jumps.

**Debugging:**
- Use GDB to inspect stack pointer and backtrace when a crash occurs.
- Consider adding stack canary checking in very early boot or for critical sections if suspected.

**Solutions:**
- Increase stack size in `src/linker.ld` (e.g., `_stack_size = 4K;`).
- Reduce large local variables on the stack; use heap or static allocation.

#### Heap Exhaustion
**Symptoms:** Allocation failures (`memory::allocate_memory` returns `None`), `Out of memory` errors from kernel.

**Debugging:**
```bash
elinOS> memory  # Check allocator statistics
elinOS> config  # Check total RAM and kernel heap size
```

**Solutions:**
- Increase QEMU memory via `QEMU_MEMORY` in `Makefile`.
- Optimize memory usage in the kernel.
- Check for memory leaks (memory allocated but never freed).

#### Memory Corruption
**Symptoms:** Random behavior, data corruption, inexplicable panics.

**Debugging:**
1.  **Enable memory debugging features if available (or add them):**
    ```rust
    // Example: Add bounds checking for critical buffer accesses
    // fn safe_memory_access(addr: usize, len: usize) -> Result<&'static [u8], &\'static str> { ... }
    ```
2.  **Use GDB watchpoints:**
    Set watchpoints on memory locations suspected of being corrupted.
3.  **Poison freed memory:**
    If you have a custom heap, when freeing memory, write a pattern (e.g., `0xDEADBEEF`) to it. If this pattern is later read or executed, it indicates use-after-free.
    ```rust
    // In your_allocator::deallocate
    // unsafe { core::ptr::write_bytes(ptr as *mut u8, 0xDE, layout.size()); }
    ```

### System Call Issues

#### Invalid System Call Numbers
**Error:** `Unknown system call` or similar error from the shell or kernel.

**Debugging:**
```bash
elinOS> syscall # Check a summary of available/implemented system calls
```
(The `categories` command mentioned previously may no longer be available).
Consult `docs/en/syscalls.md` for the definitive list of syscalls, their numbers, and categories.

**Solutions:**
- Verify syscall numbers used by user-space applications against `docs/en/syscalls.md`.
- Check the main dispatcher in `src/syscall/mod.rs` and submodule handlers (e.g. `src/syscall/file.rs`) for how numbers are routed and handled.

#### Parameter Validation Errors
**Error:** System calls fail, or behave unexpectedly, due to incorrect parameters.

**Debugging:**
1.  **Add parameter logging in kernel syscall handlers:**
    ```rust
    // Example in a syscall handler function
    // pub fn sys_openat(args: &SyscallArgs) -> SysCallResult {
    //     console_println!("sys_openat: dirfd={}, path_ptr=0x{:x}, flags={}, mode={}",
    //         args.arg0_as_i32(), args.arg1, args.arg2, args.arg3);
    //     // ... implementation
    // }
    ```
2.  **Validate in user space (if applicable):**
    Ensure any test programs or user-space code correctly prepares and passes arguments.

### VirtIO Device Issues

#### Device Not Found
**Symptoms:** `elinOS> devices` command shows no VirtIO devices or fewer than expected.

**Debugging:**
```bash
elinOS> devices
# Check output for listed VirtIO block devices.
```

**Solutions:**
1.  **Check QEMU configuration in `Makefile`:**
    - Ensure VirtIO disk is configured (e.g., `-drive ... -device virtio-blk-device,...`). The `Makefile` typically handles this.
2.  **Verify MMIO addresses and kernel driver:**
    - Check that the VirtIO MMIO base address used by the kernel driver matches QEMU's `virt` machine specification.
    - Ensure the VirtIO driver in `src/virtio_blk.rs` is correctly probing and initializing devices.
3.  **QEMU Logs:** Check `qemu.log` for any VirtIO related errors reported by QEMU itself.

---
This document provides a starting point. Effective debugging often involves a combination of these techniques and careful code review. 