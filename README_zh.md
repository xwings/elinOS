# elinOS

**采用 Rust 编写的现代 RISC-V 实验性操作系统内核**

[![构建状态](https://img.shields.io/badge/build-passing-brightgreen)](https://github.com/username/elinOS)
[![许可证](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue)](#license)
[![RISC-V](https://img.shields.io/badge/arch-RISC--V64-orange)](https://riscv.org/)
[![Rust](https://img.shields.io/badge/language-Rust-orange.svg)](https://www.rust-lang.org/)
[![no_std](https://img.shields.io/badge/no__std-yes-green)](https://docs.rust-embedded.org/book/intro/no-std.html)

> **elinOS** 是一个专为研究、学习和探索先进内存管理技术而设计的实验性操作系统内核。完全采用 Rust 语言为 RISC-V 架构构建，具有动态硬件检测、复杂的多层内存分配器、真实文件系统实现和全面的 Linux 兼容系统调用接口等特性。

## 🌟 核心特性

### 🧠 **先进内存管理**
- **多层架构**：伙伴分配器 + Slab 分配器 + 可失败操作
- **动态硬件检测**：自动检测可用 RAM 并配置分配器
- **内存区域**：支持 DMA、Normal 和 High 内存区域，并自动检测
- **自适应大小调整**：缓冲区和分配器配置根据检测到的内存动态调整
- **复杂分配管理**：处理从 8 字节对象到多兆字节分配的各种需求

### 💾 **全面文件系统支持**
- **多文件系统**：原生 FAT32 和 ext2 实现，支持自动检测
- **自动检测**：通过探测引导扇区和超级块识别文件系统类型
- **FAT32 特性**：引导扇区解析、目录枚举、簇链管理、8.3 文件名支持
- **ext2 特性**：超级块验证、inode 解析、区段树遍历、组描述符
- **文件操作**：创建、读取、写入、删除文件和目录
- **VirtIO 块设备**：完整的 VirtIO 1.0/1.1 支持，带自动检测
- **动态缓冲**：文件缓冲区根据可用内存从 4KB 扩展到 1MB+

### ⚙️ **系统架构**
- **RISC-V 64位**：原生支持 RV64GC，包含管理者模式和中断处理
- **Linux 兼容系统调用**：100+ 系统调用，分为 8 个类别
- **内存安全**：零成本抽象和全面的错误处理
- **SBI 集成**：完整的 SBI（Supervisor Binary Interface）支持
- **中断处理**：完整的中断和异常处理系统
- **虚拟内存**：软件 MMU 实现，带内存保护

### 🖥️ **交互式 Shell 界面**
- **内置命令**：20+ Shell 命令用于系统交互
- **文件系统操作**：`ls`、`cat`、`touch`、`mkdir`、`rm`、`rmdir`、`cd`、`pwd`
- **系统监控**：`memory`、`devices`、`config`、`syscall`、`version`
- **实时诊断**：实时系统统计和设备信息
- **路径解析**：完整路径解析，支持 `.` 和 `..`
- **模块化设计**：独立的 Shell 包装箱实现清晰架构

## 🚀 快速上手

### 先决条件

```bash
# 安装 Rust 工具链
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# 添加 RISC-V 目标
rustup target add riscv64gc-unknown-none-elf

# 安装 QEMU (Ubuntu/Debian 示例)
sudo apt install qemu-system-riscv64

# 安装构建工具
sudo apt install build-essential git
```

### 构建与运行

```bash
# 克隆仓库
git clone https://github.com/username/elinOS.git
cd elinOS

# 构建内核
make build

# 使用 QEMU 运行
make run
```

### 创建测试文件系统

```bash
# 创建 FAT32 测试磁盘
make fat32-disk

# 创建 ext2 测试磁盘
make ext2-disk

# 填充磁盘文件
make populate-disk

# 内核将自动检测并挂载文件系统
make run
```

## 💻 系统需求

### 硬件支持
- **架构**：RISC-V 64位 (RV64GC)
- **内存**：最低 8MB，最高 8GB+ (自动扩展)
- **存储**：VirtIO 块设备 (传统 1.0 和现代 1.1+)
- **平台**：QEMU `virt` 虚拟机、SiFive 开发板及兼容硬件

### 主机需求
- **Rust**：Nightly 工具链，带 `riscv64gc-unknown-none-elf` 目标
- **QEMU**：5.0+ 版本，支持 RISC-V 系统模拟
- **构建工具**：GNU Make、GCC 工具链

## 🏗️ 架构概览

```
┌─────────────────────────────────────────────────────────────┐
│                        用户空间                             │
│                    (未来开发)                               │
├─────────────────────────────────────────────────────────────┤
│                   系统调用接口                              │
│              (Linux 兼容: 100+ 系统调用)                    │
│                     8 个类别                                │
├─────────────────────────────────────────────────────────────┤
│                      elinOS 内核                            │
│                                                             │
│  ┌─────────────────┐ ┌─────────────────┐ ┌───────────────┐  │
│  │ 内存管理器      │ │ 文件系统        │ │ 设备管理      │  │
│  │                 │ │                 │ │               │  │
│  │ • 伙伴分配器    │ │ • FAT32 + ext2  │ │ • VirtIO 1.1  │  │
│  │ • Slab 分配器   │ │ • 自动检测      │ │ • 自动检测    │  │
│  │ • 可失败操作    │ │ • 文件 CRUD     │ │ • SBI 运行时  │  │
│  │ • 自动扩展      │ │ • 路径解析      │ │ • 中断处理    │  │
│  └─────────────────┘ └─────────────────┘ └───────────────┘  │
├─────────────────────────────────────────────────────────────┤
│                    硬件抽象层                               │
│              (RISC-V + SBI + VirtIO + MMU)                  │
└─────────────────────────────────────────────────────────────┘
```

## 🔧 可用命令

### 文件系统操作
```bash
elinOS> ls [路径]               # 列出文件和目录
elinOS> cat <文件名>            # 显示文件内容
elinOS> touch <文件名>          # 创建空文件
elinOS> mkdir <目录名>          # 创建目录
elinOS> rm <文件名>             # 删除文件
elinOS> rmdir <目录名>          # 删除空目录
elinOS> cd <路径>               # 切换目录
elinOS> pwd                     # 显示当前目录
```

### 系统信息
```bash
elinOS> help                    # 显示所有可用命令
elinOS> version                 # 内核版本和特性
elinOS> config                  # 显示系统配置
elinOS> memory                  # 内存布局和分配器统计
elinOS> heap                    # 详细堆信息
elinOS> devices                 # 列出检测到的 VirtIO 设备
elinOS> syscall                 # 显示系统调用信息
elinOS> fscheck                 # 文件系统状态和信息
```

### 系统控制
```bash
elinOS> echo <消息>             # 打印消息
elinOS> shutdown                # 优雅关闭系统
elinOS> reboot                  # 重启系统
```

## 🔬 开发与研究

elinOS 专为以下用途而设计：

- **内存管理研究**：测试先进分配策略
- **文件系统开发**：真实文件系统实现学习
- **操作系统内核开发**：理解内核架构概念
- **RISC-V 开发**：探索 RISC-V 架构特性
- **系统编程**：学习底层 Rust 编程

## 🤝 贡献

我们欢迎贡献！详情请参阅我们的 [贡献指南](CONTRIBUTING.md)。

### 开发工作流
1. Fork 本仓库
2. 创建特性分支 (`git checkout -b feature/amazing-feature`)
3. 进行更改
4. 充分测试 (`make test`)
5. 使用清晰的提交信息进行提交
6. 推送到您的分支
7. 发起 Pull Request

### 代码标准
- 遵循 Rust 最佳实践和风格
- 保持 `#![no_std]` 兼容性
- 详细记录公共 API
- 为新功能添加测试
- 确保内存安全和性能

## 🛣️ 当前状态与路线图

### ✅ 已完成 (v0.1.0)
- [x] 带伙伴 + Slab 分配器的动态内存管理
- [x] 硬件自动检测和自适应大小调整
- [x] 完整的 FAT32 和 ext2 文件系统实现
- [x] 100+ Linux 兼容系统调用
- [x] 带 20+ 命令的交互式 Shell
- [x] VirtIO 块设备支持
- [x] 全面的中断和异常处理
- [x] 虚拟内存管理（软件 MMU）
- [x] 基本 ELF 程序加载和执行

### 🚧 开发中 (v0.2.0)
- [ ] 高级 ELF 程序加载和执行
- [ ] 用户空间进程管理
- [ ] 高级内存保护（硬件 MMU）
- [ ] 改进的文件系统写操作
- [ ] 网络栈实现

### 🔮 未来目标 (v0.3.0+)
- [ ] SMP（多核）支持
- [ ] 带优先级队列的高级调度器
- [ ] 设备驱动框架
- [ ] IPC 机制（管道、共享内存）
- [ ] 安全加固和能力系统
- [ ] 性能优化

## 📊 技术规格

### 内存管理
- **伙伴分配器**：大对象的二进制幂次块分配
- **Slab 分配器**：高效的小对象分配（8B 到 4KB）
- **可失败操作**：优雅处理内存不足条件
- **自动扩展**：内存配置适应检测到的硬件

### 文件系统支持
- **FAT32**：带簇链管理的完整实现
- **ext2**：完整的超级块、inode 和区段树支持
- **统一接口**：所有文件系统类型的通用 API
- **自动检测**：自动文件系统类型识别

## 🐛 已知限制

- **用户空间**：尚无用户进程（仅内核）
- **网络**：系统调用已定义但未实现
- **SMP**：仅单核
- **硬件**：限于 QEMU 和兼容平台
- **调试**：基本调试支持

## 📜 许可证

本项目采用以下任一许可证：

- Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) 或 http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) 或 http://opensource.org/licenses/MIT)

您可以选择其中任何一个。

## 🙏 致谢

- **Rust 社区**：提供出色的 `no_std` 生态系统
- **RISC-V 基金会**：提供开放、可扩展的架构
- **QEMU 团队**：提供多功能的仿真平台
- **Linux 内核**：系统调用接口灵感
- **Maestro OS**：内存管理架构见解

---

**elinOS** - *Rust 与 RISC-V 在内核空间的相遇* 🦀⚡