# elinOS 开发指南

本指南介绍如何参与 elinOS 项目开发，包括环境搭建、代码结构、开发流程等。

## 开发环境搭建

### 基础环境要求

```bash
# 安装 Rust nightly 工具链
rustup default nightly
rustup target add riscv64gc-unknown-none-elf

# 安装 QEMU 模拟器
# Ubuntu/Debian
sudo apt install qemu-system-misc

# macOS
brew install qemu

# 安装开发工具
sudo apt install git build-essential gdb-multiarch
```

### 克隆项目

```bash
git clone <elinOS-repository>
cd elinOS

# 编译项目
cargo build --target riscv64gc-unknown-none-elf

# 运行测试
./run.sh  # 或直接运行 QEMU 命令
```

## 代码架构概览

### 目录结构

```
elinOS/
├── src/                 # 内核源代码
│   ├── main.rs         # 内核入口点
│   ├── syscall/        # 系统调用模块
│   ├── commands.rs     # 用户命令处理
│   ├── memory.rs       # 内存管理
│   ├── filesystem.rs   # FAT32 文件系统
│   ├── virtio_blk.rs   # VirtIO 块设备驱动
│   └── ...
├── docs/               # 项目文档
├── disk.raw            # 虚拟磁盘镜像
├── Cargo.toml          # Rust 项目配置
└── linker.ld           # 链接器脚本
```

### 模块职责

| 模块 | 职责 | 主要文件 |
|------|------|----------|
| **系统调用** | Linux 兼容的系统调用接口 | `src/syscall/` |
| **文件系统** | FAT32 文件系统实现 | `src/filesystem.rs` |
| **块设备** | VirtIO 块设备驱动 | `src/virtio_blk.rs` |
| **内存管理** | 动态内存分配和布局 | `src/memory.rs` |
| **用户接口** | 交互式命令处理 | `src/commands.rs` |

## 开发工作流

### 添加新命令

1. **实现命令函数**

在 `src/commands.rs` 中添加新命令：

```rust
fn cmd_newcommand() -> Result<(), &'static str> {
    console_println!("这是一个新命令！");
    // 执行命令逻辑
    Ok(())
}
```

2. **注册命令**

在 `process_command()` 函数中添加：

```rust
match command {
    // ... 现有命令
    "newcommand" => cmd_newcommand(),
    // ...
}
```

3. **更新帮助信息**

在 `cmd_help()` 中添加命令说明：

```rust
console_println!("  newcommand  - 新命令的说明");
```

### 添加新系统调用

1. **选择系统调用号**

根据功能类别选择合适的系统调用号：

```rust
// 在相应的 syscall/*.rs 文件中
pub const SYS_NEW_SYSCALL: usize = 65; // 文件 I/O 类别
```

2. **实现系统调用处理器**

```rust
fn sys_new_syscall(arg1: usize, arg2: usize) -> SysCallResult {
    // 实现系统调用逻辑
    console_println!("执行新系统调用：arg1={}, arg2={}", arg1, arg2);
    SysCallResult::Success(0)
}
```

3. **注册到分发器**

在相应的 `handle_*_syscall()` 函数中添加：

```rust
match args.syscall_num {
    // ... 现有系统调用
    SYS_NEW_SYSCALL => sys_new_syscall(args.arg0, args.arg1),
    // ...
}
```

### 扩展文件系统功能

1. **理解 FAT32 结构**

```rust
// FAT32 目录条目结构
struct DirEntry {
    name: [u8; 8],        // 文件名
    ext: [u8; 3],         // 扩展名
    attr: u8,             // 属性
    // ... 其他字段
}
```

2. **添加新的文件操作**

```rust
impl Fat32FileSystem {
    pub fn create_file(&mut self, name: &str) -> Result<(), FsError> {
        // 实现文件创建逻辑
        // 1. 分配新的目录条目
        // 2. 写入文件元数据
        // 3. 更新 FAT 表
        Ok(())
    }
}
```

### VirtIO 设备开发

1. **理解 VirtIO 协议**

```rust
// VirtIO 描述符
struct VirtqDesc {
    addr: u64,    // 缓冲区物理地址
    len: u32,     // 缓冲区长度
    flags: u16,   // 描述符标志
    next: u16,    // 下一个描述符索引
}
```

2. **添加新的 VirtIO 操作**

```rust
impl RustVmmVirtIOBlock {
    pub fn flush_cache(&mut self) -> DiskResult<()> {
        // 实现缓存刷新
        // 1. 构造 VirtIO 请求
        // 2. 提交到队列
        // 3. 等待完成
        Ok(())
    }
}
```

## 调试技巧

### 使用 GDB 调试

1. **启动 GDB 调试会话**

```bash
# 终端 1：启动 QEMU 等待 GDB 连接
qemu-system-riscv64 \
    -machine virt \
    -cpu rv64 \
    -smp 1 \
    -m 128M \
    -nographic \
    -bios /usr/share/qemu/opensbi-riscv64-generic-fw_dynamic.bin \
    -kernel target/riscv64gc-unknown-none-elf/debug/elinOS \
    -drive file=disk.raw,format=raw,if=none,id=virtio-disk \
    -device virtio-blk-device,drive=virtio-disk \
    -s -S

# 终端 2：连接 GDB
gdb-multiarch target/riscv64gc-unknown-none-elf/debug/elinOS
(gdb) target remote localhost:1234
(gdb) continue
```

2. **常用 GDB 命令**

```bash
# 设置断点
(gdb) break main
(gdb) break commands::process_command

# 查看变量
(gdb) print variable_name
(gdb) x/10x memory_address

# 单步执行
(gdb) step
(gdb) next
```

### 内核日志调试

使用 `console_println!` 宏添加调试输出：

```rust
console_println!("ℹ️ 调试信息：变量值 = {}", value);
console_println!("ℹ️ 执行到函数：{}", function_name);
```

### 系统状态检查

利用内置命令检查系统状态：

```bash
elinOS> memory    # 检查内存布局
elinOS> devices   # 检查设备状态
elinOS> syscall   # 检查系统调用信息
```

## 测试指南

### 单元测试

虽然 elinOS 是 `no_std` 环境，但可以为某些模块编写测试：

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fat32_parsing() {
        // 测试 FAT32 解析逻辑
    }
}
```

### 集成测试

编写自动化测试脚本：

```bash
#!/bin/bash
# tests/integration_test.sh

echo "启动 elinOS 测试..."
timeout 30 expect << 'EOF'
spawn qemu-system-riscv64 [QEMU参数]
expect "elinOS>"
send "ls\r"
expect "FILE"
send "shutdown\r"
expect eof
EOF
echo "测试完成"
```

### 性能测试

测量关键操作的性能：

```rust
let start_time = get_time();
// 执行操作
let end_time = get_time();
console_println!("操作耗时：{} 毫秒", end_time - start_time);
```

## 代码规范

### Rust 代码风格

```rust
// 1. 使用有意义的变量名
let file_descriptor = open_file(filename)?;
let bytes_read = read_file_content(&mut buffer)?;

// 2. 错误处理
match result {
    Ok(value) => process_value(value),
    Err(error) => {
        console_println!("操作失败：{:?}", error);
        return Err(error);
    }
}

// 3. 文档注释
/// 读取 FAT32 文件系统中的文件
/// 
/// # 参数
/// 
/// * `filename` - 要读取的文件名
/// 
/// # 返回值
/// 
/// 成功时返回文件内容，失败时返回错误
pub fn read_file(filename: &str) -> Result<Vec<u8>, FsError> {
    // 实现代码
}
```

### 提交信息格式

```bash
git commit -m "类型(范围): 简短描述

详细描述（可选）

相关 Issue: #123"
```

**提交类型**：
- `feat`: 新功能
- `fix`: 错误修复
- `docs`: 文档更新
- `style`: 代码格式调整
- `refactor`: 代码重构
- `test`: 添加测试
- `chore`: 构建工具或辅助工具更改

## 贡献流程

### 提交代码

1. **Fork 项目**
2. **创建功能分支**

```bash
git checkout -b feature/新功能名称
```

3. **开发和测试**
4. **提交更改**

```bash
git add .
git commit -m "feat(syscall): 添加新的文件操作系统调用"
```

5. **推送到 Fork**

```bash
git push origin feature/新功能名称
```

6. **创建 Pull Request**

### 代码审查

Pull Request 将经过以下审查：

- **代码质量**：遵循 Rust 最佳实践
- **功能完整性**：确保新功能正常工作
- **测试覆盖**：包含适当的测试
- **文档更新**：更新相关文档

## 常见问题

### 编译错误

```bash
# 错误：目标不存在
rustup target add riscv64gc-unknown-none-elf

# 错误：链接失败
cargo clean && cargo build --target riscv64gc-unknown-none-elf
```

### 运行时错误

```bash
# QEMU 无法启动
# 检查 QEMU 安装和 OpenSBI 路径

# 系统调用失败
# 检查系统调用号和参数
```

### 调试技巧

```rust
// 在关键位置添加调试输出
console_println!("❌ 到达关键点：函数 = {}, 行 = {}", 
    function_name!(), line!());

// 检查内存地址
console_println!("ℹ️ 变量地址：{:p}", &variable);
```

## 进阶开发

### 添加新设备驱动

1. 研究设备规范（如 VirtIO 网络设备）
2. 实现设备发现和初始化
3. 添加设备特定的系统调用
4. 创建用户空间测试命令

### 实现新文件系统

1. 研究文件系统格式（如 ext2）
2. 实现文件系统解析器
3. 添加文件操作接口
4. 集成到现有的系统调用中

### 性能优化

1. 使用 `perf` 工具分析性能瓶颈
2. 优化关键路径代码
3. 实现缓存机制
4. 减少不必要的内存分配

## 学习资源

### 推荐阅读

- **《Rust 系统编程》** - 深入理解 Rust 在系统级编程中的应用
- **《操作系统设计与实现》** - 操作系统基础理论
- **VirtIO 规范** - 了解 VirtIO 设备接口
- **RISC-V 指令集手册** - 理解 RISC-V 架构

### 在线资源

- **Rust 官方文档**: https://doc.rust-lang.org/
- **RISC-V 基金会**: https://riscv.org/
- **QEMU 文档**: https://qemu.readthedocs.io/

## 参与社区

- 提交 Issue 报告问题或建议
- 参与 Pull Request 讨论
- 完善项目文档
- 分享使用经验和心得

---

> **提示**: 开发过程中遇到问题，可以参考英文文档或在 GitHub 上提交 Issue。 