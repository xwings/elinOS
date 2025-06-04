# elinOS Documentation

Documentation for the RISC-V64 experimental operating system featuring VirtIO block device, FAT32 filesystem, and Linux-compatible system calls.

## 📁 Structure

```
docs/
├── en/           # English Documentation  
│   ├── getting-started.md   # Setup and compilation
│   ├── commands.md          # System command reference
│   ├── architecture.md      # Technical architecture
│   ├── development.md       # Development guide
│   ├── debugging.md         # Debugging techniques
│   └── syscalls.md          # System call interface
└── zh/           # Chinese Documentation
    ├── getting-started.md   # 安装与编译指南
    ├── commands.md          # 系统命令参考
    ├── architecture.md      # 技术架构文档
    └── development.md       # 开发指南
```

## 🌐 Languages

### English Documentation ✅
Comprehensive documentation available in the `en/` folder covering:
- VirtIO block device architecture
- FAT32 filesystem implementation
- Linux-compatible system call interface
- Memory management strategies
- Development and debugging workflows

### Chinese Documentation 🚧
Chinese documentation available in the `zh/` folder:
- ✅ **getting-started.md** - Setup and basic usage
- ✅ **commands.md** - Command reference
- ✅ **architecture.md** - System architecture overview
- 🚧 **development.md** - Development guide (in progress)

## 📖 Key Topics Covered

### System Architecture
- RISC-V64 kernel implementation
- VirtIO MMIO transport layer
- Legacy and modern VirtIO support
- Memory layout and management

### Storage & Filesystem
- VirtIO block device integration
- FAT32 filesystem implementation
- File I/O operations
- System call interface

### Development
- Rust cross-compilation setup
- QEMU virtual machine configuration
- Debugging techniques
- Testing strategies

## 🤝 Contributing

We welcome contributions to improve documentation:

1. **Updates**: Keep docs current with code changes
2. **Translations**: Expand Chinese documentation
3. **Examples**: Add practical usage examples
4. **Clarifications**: Improve technical explanations

For development contributions, see the main [README](../README.md). 