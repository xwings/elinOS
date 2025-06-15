# elinOS Shell 命令

本指南涵盖 elinOS 交互式 Shell 中的所有可用命令。

## 概述

elinOS 启动后，您将可以访问一个交互式 Shell，其中包含组织成几个类别的全面命令：

- **系统信息** - 检查系统状态和配置
- **文件系统操作** - 管理文件和目录
- **系统控制** - 关闭、重启

## 系统信息命令

### `help`
显示可用命令及其描述。

**用法：**
```
elinOS> help
```

### `version`
显示 elinOS 版本和特性。

**用法：**
```
elinOS> version
```

### `memory`
显示检测到的内存区域和分配器统计信息。

**用法：**
```
elinOS> memory
```

### `devices`
列出检测到的 VirtIO 设备。

**用法：**
```
elinOS> devices
```

### `syscall`
显示系统调用信息（例如总数、架构）。

**用法：**
```
elinOS> syscall
```

### `fscheck`
检查活动文件系统的状态以及超级块/元数据信息。用于在操作后验证文件系统完整性。

**用法：**
```
elinOS> fscheck
```

### `config`
显示动态系统配置，包括检测到的硬件参数和内核设置。

**用法：**
```
elinOS> config
```

## 文件系统操作

注意：大多数文件系统命令现在都能正确处理相对路径和绝对路径。当前工作目录由内部管理。

### `ls [path]`
列出文件和目录。如果提供了 `[path]`，则列出该路径的内容。否则，列出当前工作目录的内容。

**用法：**
```
elinOS> ls
elinOS> ls /some/directory
elinOS> ls ../another_dir
```

### `cat <path>`
显示指定 `path` 的文件内容。

**用法：**
```
elinOS> cat myfile.txt
elinOS> cat /path/to/another_file.txt
```

### `echo [message]`
将指定的 `[message]` 打印到控制台。如果没有提供消息，则打印一个换行符。

**用法：**
```
elinOS> echo Hello World
elinOS> echo
```

### `pwd`
打印当前工作目录。

**用法：**
```
elinOS> pwd
```

### `touch <path>`
在指定的 `path` 创建一个新的空文件。

**用法：**
```
elinOS> touch newfile.txt
elinOS> touch /some/dir/another_new_file.txt
```

### `mkdir <path>`
在指定的 `path` 创建一个新目录。

**用法：**
```
elinOS> mkdir new_directory
elinOS> mkdir /some/path/another_dir
```

### `rm <path>`
删除指定 `path` 的文件。

**用法：**
```
elinOS> rm oldfile.txt
elinOS> rm /some/dir/file_to_delete.txt
```

### `rmdir <path>`
删除指定 `path` 的空目录。

**用法：**
```
elinOS> rmdir empty_directory
elinOS> rmdir /some/path/empty_dir_to_remove
```

### `cd [path]`
更改当前工作目录。如果提供了 `[path]`，则更改到该路径。如果没有提供路径，或路径无效，它可能会默认到根目录或打印错误。`cd /` 更改到根目录。`cd ..` 更改到父目录。

**用法：**
```
elinOS> cd /my/new_directory
elinOS> cd ..
elinOS> cd
```

## 系统控制命令

### `shutdown`
通过 SBI 优雅地关闭系统。

**用法：**
```
elinOS> shutdown
```

### `reboot`
通过 SBI 重启系统。

**用法：**
```
elinOS> reboot
```

## 示例会话

这是一个完整的示例会话，展示了各种命令：

```
elinOS v0.1.0 - RISC-V kernel
Starting interactive shell...

elinOS> help
Available commands:
  help       - Show this help
  memory     - Show memory information
  ext2check  - Check embedded ext2 filesystem
  disktest   - Test filesystem operations
  ls         - List files
  cat <file> - Show file contents
  touch <file> - Create empty file
  rm <file>  - Delete file
  clear      - Clear screen
  syscall    - Show system call info
  categories - Show syscall categories
  version    - Show elinOS version
  elf-info <file>  - Show ELF binary information
  elf-load <file>  - Load ELF binary into memory
  elf-exec <file>  - Execute ELF binary (simulated)
  elf-demo   - ELF loader demonstration
  shutdown   - Shutdown the system
  reboot     - Reboot the system

elinOS> version
elinOS v0.1.0 - RISC-V kernel
Built with Rust and proper syscall architecture
Organized syscalls inspired by Qiling framework

elinOS> memory
Memory regions:
  Region 0: 0x80000000 - 0x88000000 (128 MB) RAM

elinOS> ls
Files:
  hello.txt (28 bytes)
  readme.md (45 bytes)
  lost+found (0 bytes)

elinOS> cat hello.txt
Contents of hello.txt:
Hello from elinOS filesystem!
--- End of file ---

elinOS> elf-info hello.elf
ELF Binary Information:
  Class: ELF64
  Data: Little-endian
  Machine: RISC-V
  Type: Executable
  Entry point: 0x10000
  Program header offset: 0x40
  Program header count: 1
  Section header offset: 0x0
  Section header count: 0

elinOS> ext2check
EXT2 Filesystem Check
====================

✅ EXT2 filesystem is active and healthy!

ℹ️ Superblock Information:
   Magic: 0xef53 ✅
   Inodes: 65536
   Blocks: 65536
   Block size: 4096 bytes
   Volume: elinOS

elinOS> disktest
Filesystem Test
```
*(注意: 上述示例会话中的命令输出和一些特定文本（如 "Hello from elinOS filesystem!"）保留英文，以反映真实的 Shell 交互。命令描述已翻译。)*

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
ℹ️ elinOS Starting...
✅ Console system initialized
ℹ️ Memory management ready
💾 VirtIO disk ready
✅ FAT32 filesystem mounted
✅ elinOS initialization complete!

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

ℹ️ System Information:
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