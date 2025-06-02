# elinOS 命令参考

> **🚧 翻译进行中** - 本文档正在翻译中，详细内容请参考 [英文完整版](../en/commands.md)。

本指南介绍 elinOS 交互式命令行中的所有可用命令。

## 概述

elinOS 启动后，您可以使用包含以下类别的交互式命令行：

- **系统信息** - 检查系统状态和配置
- **文件系统操作** - 管理文件和目录
- **ELF 操作** - 加载和分析 ELF 二进制文件
- **系统控制** - 关机、重启和清屏

## 基础命令

### `help`
显示可用命令及其说明。

```
elinOS> help
```

### `version`
显示 elinOS 版本信息。

```
elinOS> version
```

### `memory`
显示内存区域信息。

```
elinOS> memory
```

### `ls`
列出所有文件及其大小。

```
elinOS> ls
```

### `cat <文件名>`
显示文件内容。

```
elinOS> cat hello.txt
```

## ELF 相关命令

### `elf-info <文件名>`
分析 ELF 二进制文件结构。

```
elinOS> elf-info hello.elf
```

### `elf-load <文件名>`
将 ELF 二进制文件加载到内存。

```
elinOS> elf-load hello.elf
```

## 系统控制

### `shutdown`
优雅关闭 elinOS。

```
elinOS> shutdown
```

### `clear`
清除屏幕。

```
elinOS> clear
```

## 📖 完整文档

详细的命令说明、参数和示例，请参考英文完整版：

- [📖 英文完整版](../en/commands.md) - 包含所有命令的详细说明和示例

## 🤝 参与翻译

如果您愿意帮助完善此文档的中文翻译：

1. 参考英文版本的完整内容
2. 翻译命令说明和示例
3. 保持技术术语的准确性
4. 提交 Pull Request

---

> **提示**: 完整的命令参考和详细示例请参考英文文档。 