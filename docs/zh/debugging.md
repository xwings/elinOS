# 调试和故障排除 ElinOS

> **🚧 翻译进行中** - 本文档正在翻译中，详细内容请参考 [英文完整版](../en/debugging.md)。

本指南介绍 ElinOS 开发的调试技术、常见问题和故障排除步骤。

## 调试设置

### QEMU 日志
调试信息自动记录到 `qemu.log`：

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

```bash
# 启动带有 GDB 服务器的 QEMU
qemu-system-riscv64 \
    -machine virt \
    -cpu rv64 \
    -smp 1 \
    -m 128M \
    -serial stdio \
    -bios default \
    -kernel kernel.bin \
    -s -S  # 端口 1234 上的 GDB 服务器，等待连接

# 在另一个终端中，使用 GDB 连接
riscv64-linux-gnu-gdb kernel.bin
(gdb) target remote :1234
(gdb) c  # 继续执行
```

## 常见问题

### 构建问题

#### 缺少 RISC-V 目标
**错误：** `error[E0463]: can't find crate for 'core'`

**解决方案：**
```bash
rustup target add riscv64gc-unknown-none-elf
```

#### 链接器错误
**错误：** `undefined reference to '_start'`

**解决方案：** 检查 `src/linker.ld` 并确保正确的入口点：
```ld
ENTRY(_start)
```

#### Cargo 构建失败
**错误：** 各种编译错误

**解决方案：**
```bash
# 清理并重新构建
cargo clean
./build.sh

# 检查缺少的依赖项
cargo check

# 更新工具链
rustup update
```

### 运行时问题

#### QEMU 启动失败
**症状：** 无输出，立即退出

**故障排除：**
1. **检查 QEMU 安装：**
   ```bash
   qemu-system-riscv64 --version
   ```

2. **验证内核二进制文件：**
   ```bash
   file kernel.bin
   # 应显示：kernel.bin: data
   ```

3. **检查内存设置：**
   ```bash
   # 尝试更多内存
   MEMORY=256M ./run.sh
   ```

#### 无串行输出
**症状：** QEMU 启动但没有文本出现

**解决方案：**
1. **检查串行配置：**
   ```bash
   # 确保 run.sh 中有 -serial stdio
   grep "serial" run.sh
   ```

2. **验证 UART 初始化：**
   - 检查 `src/main.rs` 中的 SBI UART 设置

### 内存问题

#### 栈溢出
**症状：** 随机崩溃、数据损坏

**解决方案：**
- 在链接器脚本中增加栈大小
- 减少局部变量使用
- 对大数据使用堆分配

#### 堆耗尽
**症状：** 分配失败、内存不足

**调试：**
```bash
elinOS> memory
Memory regions:
  Region 0: 0x80000000 - 0x88000000 (128 MB) RAM
```

**解决方案：**
- 增加 QEMU 内存：`MEMORY=256M ./run.sh`
- 优化内存使用
- 检查内存泄漏

## 系统调用问题

#### 无效的系统调用号
**错误：** `Invalid system call number`

**调试：**
```bash
elinOS> syscall
# 检查可用的系统调用

elinOS> categories
# 验证调用号范围
```

**解决方案：**
- 验证文档中的系统调用号范围
- 检查类别边界
- 使用正确的号码更新用户程序

## ELF 加载器问题

#### 无效的 ELF 文件
**错误：** `Invalid ELF magic number`

**调试：**
```bash
# 检查 ELF 文件有效性
file hello.elf
readelf -h hello.elf

# 验证魔数
hexdump -C hello.elf | head -1
# 应以：7f 45 4c 46 开始
```

**解决方案：**
- 使用正确的 RISC-V 工具链重新编译
- 检查编译标志
- 验证文件在传输过程中未损坏

## 开发调试技术

### 添加调试打印
```rust
// 调试构建的条件编译
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

### 断言宏
```rust
// 内核调试的自定义断言
macro_rules! kernel_assert {
    ($cond:expr, $msg:expr) => {
        if !$cond {
            panic!("Kernel assertion failed: {} at {}:{}", 
                   $msg, file!(), line!());
        }
    };
}
```

## 📖 完整文档

详细的调试技术、性能分析和高级故障排除，请参考英文完整版：

- [📖 英文完整版](../en/debugging.md) - 包含完整的调试指南和技术

## 🤝 参与翻译

如果您愿意帮助完善此文档的中文翻译：

1. 参考英文版本的详细调试技术
2. 翻译具体的错误消息和解决方案
3. 保持技术术语的准确性
4. 提交 Pull Request

---

> **提示**: 详细的调试技术和高级故障排除请参考英文文档。 