# elinOS

基于 Rust 语言开发的 RISC-V64 实验性操作系统，支持现代系统调用架构和 VirtIO 设备。

## 核心特性

### 系统架构
- **RISC-V64 内核** - 原生 64 位 RISC-V 架构实现
- **Rust 语言开发** - 内存安全保证，零开销抽象
- **Linux 兼容系统调用** - 标准系统调用接口，便于上手

### 存储与文件系统
- **VirtIO 块设备** - 现代准虚拟化存储接口
- **FAT32 文件系统** - 完整支持 FAT32 卷读取
- **文件操作** - 支持文件列表、读取和显示

### 内存管理
- **动态内存布局** - 智能检测内核大小
- **多级内存分配** - 高效的内存管理机制
- **页对齐操作** - 确保内存对齐和安全性

### 设备支持
- **VirtIO MMIO 传输** - 兼容传统和现代 VirtIO
- **UART 串口控制台** - 串口交互界面
- **QEMU 虚拟机** - 针对 QEMU 平台优化

## 快速上手

### 环境要求
- Rust nightly 工具链
- QEMU RISC-V 系统模拟器
- 交叉编译工具链

### 编译构建
```bash
cargo build --target riscv64gc-unknown-none-elf
```

### 启动运行
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

## 可用命令

### 文件操作
- `ls` - 列出文件系统中的文件
- `cat <文件名>` - 显示文件内容
- `echo <内容>` - 输出文本

### 系统信息
- `help` - 显示帮助信息
- `version` - 显示内核版本
- `memory` - 显示内存使用情况
- `devices` - 列出系统设备
- `syscall` - 显示系统调用信息

### 系统控制
- `shutdown` - 关机
- `reboot` - 重启

## 系统调用接口

elinOS 实现了与 Linux 兼容的系统调用，用于实验研究：

### 文件 I/O 操作
- `SYS_WRITE (64)` - 写入文件描述符
- `SYS_READ (63)` - 从文件描述符读取
- `SYS_OPENAT (56)` - 打开文件（现代 Linux openat）
- `SYS_CLOSE (57)` - 关闭文件描述符
- `SYS_GETDENTS64 (61)` - 列出目录项

### 系统信息
- `SYS_GETMEMINFO (960)` - 获取内存信息
- `SYS_GETDEVICES (950)` - 获取设备信息
- `SYS_ELINOS_VERSION (902)` - 获取系统版本
- `SYS_ELINOS_SHUTDOWN (903)` - 系统关机
- `SYS_ELINOS_REBOOT (904)` - 系统重启

## 技术架构

### I/O 调用栈
```
用户命令 → 系统调用 → 文件系统 → VirtIO → QEMU
```

### 内存布局
- 动态检测内核大小
- 智能堆内存分配
- 页对齐的内存区域
- 区域间安全隔离

### VirtIO 集成
- MMIO 传输层
- 支持传统 VirtIO 1.0
- 兼容现代 VirtIO 1.1+
- 高效的描述符链管理

## 实验目标

elinOS 专为实验研究而设计：
- 现代操作系统开发实践
- Rust 系统编程技术
- VirtIO 设备驱动开发
- 系统调用机制实现
- 内存管理策略研究

## 开源协议

采用 MIT 开源协议 - 详见 LICENSE 文件。

## 参与贡献

欢迎提交代码！请阅读贡献指南并通过 Pull Request 提交改进建议。 