# elinOS 文档

RISC-V64 实验性操作系统文档，特性包括 VirtIO 块设备、FAT32 文件系统和 Linux 兼容的系统调用。

## 📁 结构

```
docs/
├── en/           # 英文文档
│   ├── getting-started.md   # 设置和编译
│   ├── commands.md          # 系统命令参考
│   ├── architecture.md      # 技术架构
│   ├── development.md       # 开发指南
│   ├── debugging.md         # 调试技术
│   └── syscalls.md          # 系统调用接口
└── zh/           # 中文文档
    ├── getting-started.md   # 安装与编译指南
    ├── commands.md          # 系统命令参考
    ├── architecture.md      # 技术架构文档
    └── development.md       # 开发指南
```

## 🌐 语言

### 英文文档 ✅
`en/` 文件夹中提供全面的英文文档，涵盖：
- VirtIO 块设备架构
- FAT32 文件系统实现
- Linux 兼容的系统调用接口
- 内存管理策略
- 开发和调试工作流程

### 中文文档 🚧
`zh/` 文件夹中提供中文文档：
- ✅ **getting-started.md** - 设置和基本用法
- ✅ **commands.md** - 命令参考
- ✅ **architecture.md** - 系统架构概述
- 🚧 **development.md** - 开发指南 (进行中)

## 📖 主要涵盖主题

### 系统架构
- RISC-V64 内核实现
- VirtIO MMIO 传输层
- 旧版和现代 VirtIO 支持
- 内存布局和管理

### 存储与文件系统
- VirtIO 块设备集成
- FAT32 文件系统实现
- 文件 I/O 操作
- 系统调用接口

### 开发
- Rust 交叉编译设置
- QEMU 虚拟机配置
- 调试技术
- 测试策略

## 🤝 贡献

我们欢迎对改进文档的贡献：

1. **更新**：保持文档与代码更改同步
2. **翻译**：扩展中文文档
3. **示例**：添加实际用法示例
4. **阐明**：改进技术解释

有关开发贡献，请参阅主 [README](../README.md)。 