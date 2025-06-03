# elinOS 命令参考手册

本手册介绍 elinOS 交互式命令行中的所有可用命令。

## 概述

elinOS 启动后，您可以使用功能完整的交互式命令行，包含以下几类命令：

- **文件操作** - FAT32 文件系统相关操作
- **系统信息** - 查看系统状态和配置  
- **系统控制** - 关机、重启等系统管理

## 文件操作命令

### `ls`
列出 FAT32 文件系统中的文件和目录。

```bash
elinOS> ls
```

**输出示例**：
```
📁 FAT32 Filesystem contents (VirtIO disk):
Boot signature: 0xaa55
Total sectors: 131072
Bytes per sector: 512

  FILE       12 bytes  HELLO.TXT (cluster: 3)
  FILE      256 bytes  README.MD (cluster: 4)
  
Total files: 2 (FAT32 on VirtIO)
```

### `cat <文件名>`
显示文件内容。使用系统调用 `SYS_OPENAT`, `SYS_READ`, `SYS_CLOSE`。

```bash
elinOS> cat HELLO.TXT
```

**输出示例**：
```
📖 Reading file: HELLO.TXT (from FAT32 VirtIO disk)
Content:
Hello World!
This is a test file on FAT32 filesystem.
```

### `echo <消息>`
向控制台输出消息。

```bash
elinOS> echo "Hello elinOS!"
```

## 系统信息命令

### `help`
显示所有可用命令及其说明。

```bash
elinOS> help
```

**输出示例**：
```
📖 elinOS Commands
===============================================

🗂️  File Operations (via VirtIO block device):
  ls              - List files in filesystem
  cat <file>      - Display file contents
  echo <message>  - Echo a message

📊 System Information:
  help            - Show this help message
  version         - Show kernel version
  memory          - Show memory information
  devices         - List VirtIO and other devices
  syscall         - Show system call information

⚙️  System Control:
  shutdown        - Shutdown the system
  reboot          - Reboot the system
```

### `version`
显示 elinOS 版本和系统信息。

```bash
elinOS> version
```

**输出示例**：
```
elinOS Version Information:
===============================================

🦀 elinOS v0.1.0
RISC-V Experimental Operating System
Written in Rust for research and development

Architecture:
  Target: riscv64gc-unknown-none-elf
  Memory Model: sv39 (future)
  Privilege Level: Machine Mode

Features:
  ✅ VirtIO Block Device Support
  ✅ FAT32 Filesystem
  ✅ Linux-Compatible System Calls
  ✅ Memory Management
  ✅ Interactive Shell
```

### `memory`
显示内存布局和使用情况。

```bash
elinOS> memory
```

### `devices`
列出系统设备，包括 VirtIO 设备。

```bash
elinOS> devices
```

**输出示例**：
```
🔍 System Device Information:
  VirtIO Block Device: ✅ Initialized
  Capacity: 131072 sectors (64 MB)
  Transport: MMIO at 0x10008000
  Version: Legacy VirtIO 1.0
  
  UART Console: ✅ Active
  Base Address: 0x10000000
```

### `syscall`
显示系统调用架构和已实现的系统调用。

```bash
elinOS> syscall
```

**输出示例**：
```
System Call Information:

Currently Implemented System Calls:
  File I/O Operations:
    SYS_WRITE (64)     - Write to file descriptor
    SYS_READ (63)      - Read from file descriptor
    SYS_OPENAT (56)    - Open file (modern Linux openat)
    SYS_CLOSE (57)     - Close file descriptor
    SYS_GETDENTS64 (61) - List directory entries

  Memory Management:
    SYS_GETMEMINFO (960) - Memory information (elinOS)

  elinOS-Specific (System Control):
    SYS_ELINOS_VERSION (902)  - Show version
    SYS_ELINOS_SHUTDOWN (903) - Shutdown system
    SYS_ELINOS_REBOOT (904)   - Reboot system
```

## 系统控制命令

### `shutdown`
使用 `SYS_ELINOS_SHUTDOWN` 优雅关闭 elinOS 并退出 QEMU。

```bash
elinOS> shutdown
```

### `reboot`
使用 `SYS_ELINOS_REBOOT` 重启系统。

```bash
elinOS> reboot
```

## 系统调用流程

所有命令都通过标准的系统调用接口工作：

```
用户命令 → 系统调用 → 文件系统 → VirtIO → QEMU
```

### 文件操作流程
1. 用户输入 `cat filename`
2. 调用 `SYS_OPENAT` 打开文件
3. 调用 `SYS_READ` 读取内容
4. FAT32 文件系统处理请求
5. VirtIO 块设备执行磁盘 I/O
6. 调用 `SYS_CLOSE` 关闭文件
7. 显示文件内容给用户

## 错误处理

如果命令执行失败，系统会显示相应的错误信息：

- **文件未找到**: `Failed to read file`
- **权限错误**: `Command failed: <错误详情>`
- **系统调用失败**: 显示具体的系统调用错误

## 使用示例

### 基本操作流程
```bash
elinOS> help           # 查看所有命令
elinOS> ls             # 列出文件
elinOS> cat README.MD  # 读取说明文件
elinOS> version        # 查看版本信息
elinOS> shutdown       # 关闭系统
```

### 调试信息
命令执行时会显示详细的调试信息，有助于理解系统内部工作原理：

- VirtIO 设备操作日志
- FAT32 文件系统解析过程
- 系统调用执行跟踪
- 内存分配和释放信息

## 命令实现原理

所有命令都是作为用户空间程序实现，通过系统调用与内核交互。命令行程序会：

1. **解析**用户输入
2. **分发**到相应的命令函数
3. **执行**命令（通过系统调用）
4. **报告**结果给用户

## 完整会话示例

```bash
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
elinOS> help
📖 elinOS Commands
===============================================

🗂️  File Operations (via VirtIO block device):
  ls              - List files in filesystem
  cat <file>      - Display file contents
  echo <message>  - Echo a message

📊 System Information:
  help            - Show this help message
  version         - Show kernel version
  memory          - Show memory information
  devices         - List VirtIO and other devices
  syscall         - Show system call information

⚙️  System Control:
  shutdown        - Shutdown the system
  reboot          - Reboot the system

elinOS> ls
📁 FAT32 Filesystem contents (VirtIO disk):
Boot signature: 0xaa55
Total sectors: 131072
Bytes per sector: 512

  FILE       12 bytes  HELLO.TXT (cluster: 3)
  FILE      256 bytes  README.MD (cluster: 4)
  
Total files: 2 (FAT32 on VirtIO)

elinOS> cat HELLO.TXT
📖 Reading file: HELLO.TXT (from FAT32 VirtIO disk)
Content:
Hello World!
This is a test file on FAT32 filesystem.

elinOS> version
elinOS Version Information:
===============================================

🦀 elinOS v0.1.0
RISC-V Experimental Operating System
Written in Rust for research and development

Architecture:
  Target: riscv64gc-unknown-none-elf
  Memory Model: sv39 (future)
  Privilege Level: Machine Mode

Features:
  ✅ VirtIO Block Device Support
  ✅ FAT32 Filesystem
  ✅ Linux-Compatible System Calls
  ✅ Memory Management
  ✅ Interactive Shell

elinOS> shutdown
System shutdown requested with status: 0
```

## 进阶学习

- [技术架构](architecture.md) - 了解系统调用实现细节
- [开发指南](development.md) - 学习如何扩展命令
- [快速上手](getting-started.md) - 系统安装和运行

## 📖 完整文档

详细的命令说明、参数和示例，请参考英文完整版：

- [📖 英文完整版](../en/commands.md) - 包含所有命令的详细说明和示例

---

> **提示**: 完整的命令参考和详细示例请参考英文文档。 