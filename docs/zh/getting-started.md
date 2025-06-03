# elinOS 快速上手指南

elinOS 是一个基于 Rust 语言开发的 RISC-V64 实验性操作系统，支持 VirtIO 块设备和 FAT32 文件系统。

## 环境准备

### 开发环境
- **Rust nightly 工具链** 包含 `riscv64gc-unknown-none-elf` 目标平台
- **QEMU RISC-V 系统模拟器** (`qemu-system-riscv64`)
- **Linux/macOS/WSL** 开发环境

### 安装 Rust 编译目标
```bash
rustup target add riscv64gc-unknown-none-elf
```

## 编译运行

### 编译内核
```bash
cargo build --target riscv64gc-unknown-none-elf
```

### 启动系统
```bash
qemu-system-riscv64 \
    -machine virt \
    -cpu rv64 \
    -smp 1 \
    -m 128M \
    -nographic \
    -bios /usr/share/qemu/opensbi-riscv64-generic-fw_dynamic.bin \
    -kernel target/riscv64gc-unknown-none-elf/debug/elinOS \
    -drive file=disk.raw,format=raw,if=none,id=virtio-disk \
    -device virtio-blk-device,drive=virtio-disk
```

## 系统启动

成功启动后，您将看到如下信息：

```
🚀 elinOS Starting...
✅ Console system initialized
🧠 Memory management ready
💾 VirtIO disk ready
✅ FAT32 filesystem mounted
🎉 elinOS initialization complete!

=====================================
       🦀 Welcome to elinOS! 🦀      
=====================================
A RISC-V64 Experimental Operating System
Written in Rust for learning purposes

Type 'help' for available commands
elinOS> 
```

## 常用命令

系统启动后可以使用以下命令：

```bash
elinOS> help        # 显示帮助信息
elinOS> ls          # 列出文件
elinOS> cat <file>  # 查看文件内容
elinOS> version     # 查看系统版本
elinOS> memory      # 查看内存信息
elinOS> devices     # 查看设备列表
elinOS> shutdown    # 关机
```

## 核心特性

### 存储与文件系统
- VirtIO 块设备驱动
- FAT32 文件系统支持
- 文件读取和列表功能

### 系统调用
- Linux 兼容的系统调用接口
- 文件 I/O 操作（open, read, close）
- 系统信息查询

### 内存管理
- 动态内存布局检测
- 页对齐内存分配
- 内存安全保障

## 常见问题

### QEMU 未安装
```bash
# Ubuntu/Debian
sudo apt install qemu-system-misc

# macOS
brew install qemu
```

### 编译错误
```bash
# 确保使用 Rust nightly
rustup default nightly
rustup target add riscv64gc-unknown-none-elf
```

### 磁盘镜像问题
确保 `disk.raw` 文件存在且包含有效的 FAT32 文件系统。

## 进阶学习

- [系统命令参考](commands.md) - 学习所有可用命令
- [技术架构](architecture.md) - 了解实现原理
- [开发指南](development.md) - 参与系统开发

## 📖 完整文档

如需详细的安装、配置和使用说明，请参考英文完整版文档：

- [📖 英文完整版](../en/getting-started.md) - 包含详细步骤和故障排除

## 🤝 参与翻译

如果您愿意帮助改进中文文档：

1. 参考英文版本内容
2. 保持相同的文档结构
3. 翻译技术术语时保持准确性
4. 提交 Pull Request

## 其他中文资源

- [🏠 项目主页](../../README_zh.md) - 项目介绍和概述
- [📁 文档索引](../README.md) - 完整文档组织结构

---

> **提示**: 在中文翻译完成之前，建议参考英文文档获取最新和最完整的信息。 