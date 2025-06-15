# elinOS 🦀

**一个采用 Rust 语言编写的现代 RISC-V 实验性内核**

[![构建状态](https://img.shields.io/badge/build-passing-brightgreen)](https://github.com/username/elinOS)
[![许可证](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue)](#license)
[![RISC-V](https://img.shields.io/badge/arch-RISC--V64-orange)](https://riscv.org/)
[![Rust](https://img.shields.io/badge/language-Rust-orange.svg)](https://www.rust-lang.org/)
[![no_std](https://img.shields.io/badge/no__std-yes-green)](https://docs.rust-embedded.org/book/intro/no-std.html)

> **elinOS** 是一个实验性的操作系统内核，专为研究、实验和探索先进内存管理技术而设计。它完全采用 Rust 语言为 RISC-V 架构从头构建，具有动态硬件检测、多层内存分配器和真实文件系统实现等特性。

## 🌟 主要特性

### ℹ️ **高级内存管理**
- **多层架构**：伙伴分配器 + Slab 分配器 + 可失败操作
- **动态硬件检测**：通过 SBI 自动检测 RAM 大小，并相应地调整所有分配
- **零硬编码值**：从 8MB 到 8GB+ 系统均可无缝扩展
- **受 Maestro OS 启发**：实现带事务回滚的可失败分配模式
- **内存区域**：支持 DMA、Normal 和 High 内存区域，并自动检测
- **性能**：小块分配速度提高约 10 倍，大块分配速度提高约 3 倍，碎片减少约 5 倍

### 💾 **真实文件系统支持**
- **多文件系统**：原生 FAT32 和 ext2 实现，带真实解析
- **自动检测**：探测引导扇区和超级块以识别文件系统类型
- **FAT32 特性**：引导扇区解析、目录枚举、簇链跟踪、8.3 文件名
- **ext2 特性**：超级块验证、inode 解析、区段树遍历、组描述符
- **VirtIO 块设备**：完整 VirtIO 1.0/1.1 支持，带自动检测和队列管理
- **动态缓冲区大小**：文件缓冲区根据可用内存动态调整（4KB → 1MB）

### ℹ️ **系统架构**
- **RISC-V 64位**：原生支持 RV64GC，包括机器模式和中断处理
- **Linux 兼容系统调用**：50+ 系统调用，包括文件 I/O、内存管理和进程控制
- **Rust 安全性**：内存安全的内核，具有零成本抽象和全面的错误处理
- **SBI 集成**：完整的 SBI（Supervisor Binary Interface）支持，用于硬件抽象

### 🛠️ **开发者体验**
- **交互式 Shell**：内置命令行界面，包含 15+ 命令
- **全面诊断**：实时系统监控、内存统计和设备信息
- **全面文档**：广泛的技术文档和架构图
- **专注于实验**：清晰的代码结构，便于学习操作系统开发概念

## ℹ️ 快速上手

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

# 在 QEMU 中以图形模式运行
make run-graphics

# 在控制台模式运行
make run
```

### 创建测试文件系统

```bash
# 创建一个带文件的 FAT32 测试磁盘
make create-disk
echo "来自 FAT32 的问候!" > hello.txt
mcopy -i disk.img hello.txt ::

# 创建一个带文件的 ext2 测试磁盘
make create-ext2
sudo mount -o loop disk.img /mnt
echo "来自 ext2 的问候!" | sudo tee /mnt/hello.txt
sudo umount /mnt

# 内核将自动检测并挂载文件系统
make run
```

## 📚 文档 (Documentation)

- **[中文文档主页](docs/zh/)**
- **[内存管理](docs/zh/memory.md)** - 先进内存子系统详情
- **[文件系统支持](docs/zh/filesystem.md)** - 存储和文件系统实现
- **[系统调用](docs/zh/syscalls.md)** - API 参考和 Linux 兼容性
- **[构建与开发](docs/zh/development.md)** - 开发者设置和工作流程
- **[可用命令](docs/zh/commands.md)** - 可用 Shell 命令列表
- **[调试指南](docs/zh/debugging.md)** - 调试技巧和技术
- **[翻译指南](docs/zh/translation.md)** - 文档翻译指南

## ℹ️ 系统需求

### 硬件支持
- **架构**：RISC-V 64位 (RV64GC)
- **内存**：最低 8MB，最高 8GB+ (自动扩展)
- **存储**：VirtIO 块设备 (传统 1.0 和现代 1.1+)
- **平台**：QEMU `virt` 虚拟机、SiFive 开发板及兼容硬件

### 主机需求
- **Rust**：Nightly 工具链，带 `riscv64gc-unknown-none-elf` 目标
- **QEMU**：5.0+ 版本，支持 RISC-V 系统模拟
- **构建工具**：GNU Make, GCC 工具链

## ℹ️架构概览

```
┌─────────────────────────────────────────────────────────────┐
│                        用户空间 (User Space)                │
│                    (未来开发 Future Development)            │
├─────────────────────────────────────────────────────────────┤
│                   系统调用接口 (System Call Interface)        │
│              (Linux 兼容: 50+ 系统调用)                       │
├─────────────────────────────────────────────────────────────┤
│                      elinOS 内核 (elinOS Kernel)              │
│                                                             │
│  ┌─────────────────┐ ┌─────────────────┐ ┌───────────────┐  │
│  │ 内存管理器      │ │ 文件系统        │ │ 设备管理      │  │
│  │ (Memory Manager)│ │ (Filesystem)    │ │ (Device Mgmt) │  │
│  │                 │ │                 │ │               │  │
│  │ • 伙伴分配器    │ │ • 真实 FAT32    │ │ • VirtIO 1.1  │  │
│  │ • Slab 分配器   │ │ • 真实 ext2     │ │ • 自动检测    │  │
│  │ • 可失败操作    │ │ • 自动检测      │ │ • SBI 运行时  │  │
│  │ • 事务          │ │ • 引导扇区      │ │ • MMIO 队列   │  │
│  └─────────────────┘ └─────────────────┘ └───────────────┘  │
├─────────────────────────────────────────────────────────────┤
│                    硬件抽象层 (Hardware Abstraction)        │
│              (RISC-V + SBI + VirtIO)                        │
└─────────────────────────────────────────────────────────────┘
```

## ℹ️ 可用命令

```bash
elinOS> help                    # 显示所有可用命令
elinOS> config                  # 显示动态系统配置
elinOS> memory                  # 显示内存布局和分配器统计信息
elinOS> devices                 # 列出检测到的 VirtIO 设备
elinOS> ls                      # 列出文件 (自动检测 FAT32/ext2)
elinOS> cat filename.txt        # 读取文件系统中的文件内容
elinOS> filesystem             # 显示文件系统类型和挂载状态
elinOS> syscall                 # 显示系统调用信息
elinOS> version                 # 内核版本和特性
elinOS> shutdown               # 通过 SBI 优雅关闭系统
elinOS> reboot                 # 通过 SBI 重启系统
```

## 🔬 研究应用

elinOS 专为以下研究设计：

- **内存管理研究**：测试高级分配策略和可失败操作
- **文件系统开发**：实现和测试新的文件系统类型
- **操作系统实验**：通过真实实现学习内核开发概念
- **硬件启动**：移植到新的 RISC-V 平台和设备
- **性能分析**：对内核子系统和分配模式进行基准测试

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

## ℹ️ 性能特性

| 指标                     | 性能表现                     | 实现方式                       |
|--------------------------|------------------------------|--------------------------------|
| **小块分配 (≤1KB)**      | 比简单堆快约 10 倍           | 带大小类的 Slab 分配器         |
| **大块分配 (≥4KB)**      | 比简单堆快约 3 倍            | 带合并功能的伙伴分配器         |
| **内存碎片**             | 比简单堆减少约 5 倍          | 多层分配策略                   |
| **启动时间**             | <100毫秒至交互式 Shell      | 优化的初始化过程               |
| **内存开销**             | 内核占用总 RAM <5%           | 高效的数据结构                 |
| **文件系统检测**         | <10毫秒检测 FAT32/ext2      | 直接解析引导扇区/超级块        |

## 🛣️ 路线图

### 当前重点 (v0.2.0)
- [ ] SMP (多核) 支持，带每 CPU 分配器
- [ ] 使用 VirtIO-net 实现网络栈
- [ ] 带优先级队列的高级调度器
- [ ] 使用虚拟内存实现内存保护 (MMU/分页)

### 未来目标 (v0.3.0+)
- [ ] 带热插拔支持的设备驱动框架
- [ ] 带 ELF 加载的用户空间进程
- [ ] IPC 机制 (管道、共享内存)
- [ ] 安全加固和能力系统

### 文件系统增强
- [ ] 文件缓存和缓冲区管理


## 📄 许可证

本项目采用双重许可证：

- **MIT 许可证** ([LICENSE-MIT](LICENSE-MIT) 或 http://opensource.org/licenses/MIT)

---

**elinOS** - *硬件与软件安全高效融合之所* 🦀✨