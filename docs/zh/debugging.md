# elinOS 调试与故障排除

本指南涵盖 elinOS 开发的调试技术、常见问题和故障排除步骤。

## 调试设置

### QEMU 日志
当使用 Makefile 中的 `run-debug` 目标时，QEMU 的调试信息会记录到 `qemu.log` 文件（通过 `-D qemu.log` 标志）。您也可以在 Makefile 的 `QEMU_ARGS` 中添加 `-d guest_errors,int,unimp` 或其他标志以获取更详细的日志。

```bash
# 实时监控日志
tail -f qemu.log

# 搜索特定问题
grep -i "error\|panic\|abort" qemu.log

# 查看最后 50 行
tail -50 qemu.log
```

### GDB 调试
使用 GDB 进行内核调试：

1.  **使用 Makefile 启动带 GDB 服务器的 QEMU：**
    Makefile 中的 `run-debug` 目标会启动 QEMU 并启用 GDB 服务器（`-s -S`），在端口 1234 等待连接。
    ```bash
    make run-debug
    ```
    QEMU 将打印类似信息：`Connect with: gdb target/riscv64gc-unknown-none-elf/debug/kernel -ex 'target remote :1234'`

2.  **在另一个终端中连接 GDB：**
    使用 `make run-debug` 输出提供的 GDB 命令。通常是：
    ```bash
    # 确保您有 RISC-V GDB，例如 gdb-multiarch 或 riscv64-unknown-elf-gdb
    gdb-multiarch target/riscv64gc-unknown-none-elf/debug/kernel
    # 或 riscv64-unknown-elf-gdb target/riscv64gc-unknown-none-elf/debug/kernel
    ```
    然后在 GDB 内部：
    ```gdb
    (gdb) target remote :1234
    (gdb) # 设置断点，例如 break kmain
    (gdb) c  # 继续执行
    ```
    内核 ELF 文件 (`target/riscv64gc-unknown-none-elf/debug/kernel`) 包含有效调试所需的符号。

### 串口输出
所有内核输出 (`console_println!`) 都通过串口控制台。监控以下信息：
- 启动消息
- 系统调用跟踪
- 错误消息
- Panic 信息

## 常见问题

### 构建问题

#### 缺少 RISC-V 目标
**错误：** `error[E0463]: can't find crate for 'core'`

**解决方案：**
```bash
rustup target add riscv64gc-unknown-none-elf
```

#### 链接器错误
**错误：** `undefined reference to '_start'` (或类似错误)

**解决方案：** 检查 `src/linker.ld` 并确保正确的入口点和段布局：
```ld
ENTRY(_start)
/* ... 其他段 ... */
```

#### Cargo 构建失败
**错误：** 各种编译错误

**解决方案：**
```bash
# 清理并重新构建
make clean
make build # 或者直接 'make'

# 检查缺少的依赖项或其他错误
cargo check --target riscv64gc-unknown-none-elf

# 更新工具链
rustup update
```

### 运行时问题

#### QEMU 启动失败
**症状：** 没有输出，QEMU 立即退出，或 QEMU 本身报错。

**故障排除：**
1.  **检查 QEMU 安装：**
    ```bash
    qemu-system-riscv64 --version
    ```
2.  **验证内核 ELF 文件：**
    使用的内核文件通常是 `target/riscv64gc-unknown-none-elf/debug/kernel`。
    ```bash
    file target/riscv64gc-unknown-none-elf/debug/kernel
    # 应显示：... ELF 64-bit LSB executable, RISC-V, ...
    ```
3.  **检查 `Makefile` 中的内存设置：**
    `Makefile` 中的 `QEMU_MEMORY` 变量（例如 `128M`）。如果需要，尝试增加。
4.  **OpenSBI：**
    确保 QEMU 可以找到/使用 OpenSBI（`Makefile` 会尝试查找它）。如果 QEMU 抱怨 BIOS 问题，这可能是一个原因。

#### OpenSBI 问题
**症状：** 启动停在 OpenSBI，或 OpenSBI 打印错误。

**解决方案：**
1.  **检查 OpenSBI 版本/路径：**
    - `Makefile` 会尝试定位 OpenSBI。确保它找到的路径是正确的，或者 QEMU 的默认设置可以工作。
    - 如果怀疑 OpenSBI 兼容性问题，请尝试不同版本的 QEMU。
2.  **内存布局问题：**
    - 根据 OpenSBI 对内核加载地址的期望，验证链接器脚本地址。
    - 检查内存重叠。

#### 无串口输出
**症状：** QEMU 启动，但在 OpenSBI 之后没有内核文本出现。

**解决方案：**
1.  **检查 `Makefile` 中的串口配置：**
    - `run` 和 `run-debug` 目标通常使用 `-nographic`，它将串口定向到标准输入输出。
    - `run-graphics` 使用 `-serial mon:vc`。
2.  **验证内核中的 UART 初始化：**
    - 检查 `src/main.rs` 或 `src/uart.rs` 中的早期 UART 设置。
    - 确保 `console_println!` 或直接的 UART 写入功能正常。

### 内存问题

#### 栈溢出
**症状：** 随机崩溃、损坏、意外跳转。

**调试：**
- 崩溃时使用 GDB 检查栈指针和回溯信息。
- 如果怀疑，考虑在非常早期的启动阶段或关键部分添加栈金丝雀检查。

**解决方案：**
- 在 `src/linker.ld` 中增加栈大小（例如 `_stack_size = 4K;`）。
- 减少栈上的大型局部变量；使用堆或静态分配。

#### 堆耗尽
**症状：** 分配失败（`memory::allocate_memory` 返回 `None`），内核报告 `Out of memory` 错误。

**调试：**
```bash
elinOS> memory  # 检查分配器统计信息
elinOS> config  # 检查总 RAM 和内核堆大小
```

**解决方案：**
- 通过 `Makefile` 中的 `QEMU_MEMORY` 增加 QEMU 内存。
- 优化内核中的内存使用。
- 检查内存泄漏（内存已分配但从未释放）。

#### 内存损坏
**症状：** 随机行为、数据损坏、无法解释的 panic。

**调试：**
1.  **启用内存调试功能（如果可用或添加它们）：**
    ```rust
    // 示例：为关键缓冲区访问添加边界检查
    // fn safe_memory_access(addr: usize, len: usize) -> Result<&'static [u8], &'static str> { ... }
    ```
2.  **使用 GDB 观察点：**
    在怀疑被损坏的内存位置设置观察点。
3.  **对已释放内存下毒：**
    如果您有自定义堆，在释放内存时，向其写入一个模式（例如 `0xDEADBEEF`）。如果之后读取或执行此模式，则表示存在释放后使用（use-after-free）。
    ```rust
    // 在 your_allocator::deallocate 中
    // unsafe { core::ptr::write_bytes(ptr as *mut u8, 0xDE, layout.size()); }
    ```

### 系统调用问题

#### 无效的系统调用号
**错误：** Shell 或内核报告 `Unknown system call` 或类似错误。

**调试：**
```bash
elinOS> syscall # 检查可用/已实现系统调用的摘要
```
（先前 언급된 `categories` 命令可能不再可用）。
查阅 `docs/en/syscalls.md` 获取系统调用、其编号和类别的权威列表。

**解决方案：**
- 根据 `docs/en/syscalls.md` 验证用户空间应用程序使用的系统调用号。
- 检查 `src/syscall/mod.rs` 中的主分发器和子模块处理程序（例如 `src/syscall/file.rs`）如何路由和处理编号。

#### 参数验证错误
**错误：** 由于参数不正确，系统调用失败或行为异常。

**调试：**
1.  **在内核系统调用处理程序中添加参数日志记录：**
    ```rust
    // 系统调用处理函数中的示例
    // pub fn sys_openat(args: &SyscallArgs) -> SysCallResult {
    //     console_println!("sys_openat: dirfd={}, path_ptr=0x{:x}, flags={}, mode={}",
    //         args.arg0_as_i32(), args.arg1, args.arg2, args.arg3);
    //     // ... 实现
    // }
    ```
2.  **在用户空间进行验证（如果适用）：**
    确保任何测试程序或用户空间代码正确准备和传递参数。

### VirtIO 设备问题

#### 未找到设备
**症状：** `elinOS> devices` 命令不显示 VirtIO 设备或显示的数量少于预期。

**调试：**
```bash
elinOS> devices
# 检查输出中列出的 VirtIO 块设备。
```

**解决方案：**
1.  **检查 `Makefile` 中的 QEMU 配置：**
    - 确保配置了 VirtIO 磁盘（例如 `-drive ... -device virtio-blk-device,...`）。`Makefile` 通常会处理此问题。
2.  **验证 MMIO 地址和内核驱动程序：**
    - 检查内核驱动程序使用的 VirtIO MMIO 基地址是否与 QEMU 的 `virt` 机器规范匹配。
    - 确保 `src/virtio_blk.rs` 中的 VirtIO 驱动程序正确探测和初始化设备。
3.  **QEMU 日志：** 检查 `qemu.log` 中 QEMU 本身报告的任何 VirtIO 相关错误。

---
本文档提供了一个起点。有效的调试通常涉及这些技术的组合和仔细的代码审查。 