# 为 elinOS 创建用户程序

> **🚧 翻译进行中** - 本文档正在翻译中，详细内容请参考 [英文完整版](../en/development.md)。

由于 elinOS 具有内置的 ELF 加载器，您可以创建和编译 C 程序在其上运行！本指南展示如何创建 elinOS 可以加载和执行的 RISC-V 二进制文件。

## 用户程序开发先决条件

- **RISC-V GCC 工具链**：安装 `riscv64-linux-gnu-gcc` 或 `riscv64-unknown-elf-gcc`
- **基础 C 知识**：用于编写简单程序
- **ELF 知识**：了解可执行文件格式（可选）

## 安装 RISC-V GCC 工具链

### Ubuntu/Debian:
```bash
sudo apt update
sudo apt install gcc-riscv64-linux-gnu
```

### Arch Linux:
```bash
sudo pacman -S riscv64-linux-gnu-gcc
```

## 创建 Hello World 程序

创建一个可以在 elinOS 上运行的简单 C 程序：

**hello.c:**
```c
// elinOS 的简单 Hello World
// 此程序演示在 elinOS 上的基本执行

// elinOS 的简单系统调用接口
static inline long syscall_write(const char* msg, int len) {
    register long a0 asm("a0") = 1;        // stdout fd
    register long a1 asm("a1") = (long)msg; // buffer
    register long a2 asm("a2") = len;      // length
    register long a7 asm("a7") = 1;        // SYS_WRITE
    register long result asm("a0");
    
    asm volatile ("ecall"
                  : "=r" (result)
                  : "r" (a0), "r" (a1), "r" (a2), "r" (a7)
                  : "memory");
    return result;
}

// 字符串长度函数
int strlen(const char* str) {
    int len = 0;
    while (str[len]) len++;
    return len;
}

// 主函数 - 入口点
int main(void) {
    const char* message = "Hello, World from elinOS user program!\n";
    syscall_write(message, strlen(message));
    
    const char* info = "This C program is running via ELF loader!\n";
    syscall_write(info, strlen(info));
    
    return 0;
}
```

## 编译程序

将您的 C 程序编译为 RISC-V ELF 二进制文件：

```bash
# 将 hello.c 编译为 RISC-V ELF
riscv64-linux-gnu-gcc \
    -march=rv64gc \
    -mabi=lp64d \
    -static \
    -nostdlib \
    -nostartfiles \
    -fno-stack-protector \
    -o hello.elf \
    hello.c
```

## 编译选项说明

- **`-march=rv64gc`**：目标 RISC-V 64 位标准扩展
- **`-mabi=lp64d`**：使用 64 位 ABI 和双精度浮点
- **`-static`**：创建静态链接可执行文件
- **`-nostdlib`**：不链接标准库（我们提供自己的系统调用）
- **`-nostartfiles`**：不使用标准启动文件

## 验证您的 ELF 二进制文件

检查您编译的程序是否为有效的 RISC-V ELF：

```bash
# 检查 ELF 头
file hello.elf
# 输出：hello.elf: ELF 64-bit LSB executable, UCB RISC-V, ...

# 检查 ELF 详细信息
readelf -h hello.elf
# 应显示 Machine: RISC-V
```

## 将程序添加到 elinOS

要在 elinOS 中使您的程序可用，请在初始化期间将其添加到文件系统：

### 选项 1：添加到 src/filesystem.rs

编辑 `src/filesystem.rs` 中的 `new()` 函数：

```rust
// 添加您编译的 ELF 二进制文件
let hello_elf = include_bytes!("../hello.elf");
let _ = fs.create_file("hello.elf", hello_elf);
```

## 在 elinOS 中测试您的程序

添加程序到文件系统并重新构建 elinOS 后：

```bash
# 使用您的程序重新构建 elinOS
./build.sh

# 运行 elinOS
./run.sh
```

在 elinOS 命令行中：

```
elinOS> ls
Files:
  hello.txt (30 bytes)
  hello.elf (8432 bytes)

elinOS> elf-info hello.elf
ELF Binary Information:
  Class: ELF64
  Data: Little-endian
  Machine: RISC-V
  Type: Executable
  Entry point: 0x10078

elinOS> elf-load hello.elf
ELF loaded successfully!
Entry point: 0x10078
```

## 系统调用接口

elinOS 为用户程序提供以下系统调用：

### 文件 I/O 操作 (1-50)
- `SYS_WRITE (1)` - 写入文件描述符
- `SYS_READ (2)` - 从文件描述符读取
- `SYS_OPEN (3)` - 打开文件

### 进程管理 (121-170)
- `SYS_EXIT (121)` - 退出进程

### elinOS 特定 (900-999)
- `SYS_ELINOS_VERSION (902)` - 获取操作系统版本

## 当前限制

⚠️ **重要说明**：
- **尚未实际执行**：elinOS 可以加载和解析 ELF 文件，但尚未执行它们
- **无虚拟内存**：程序需要适当的内存管理才能执行
- **无进程隔离**：当前实现缺乏进程上下文切换
- **有限的系统调用**：仅实现了基本系统调用

## 📖 完整文档

详细的开发指南、高级示例和系统调用接口，请参考英文完整版：

- [📖 英文完整版](../en/development.md) - 包含完整的开发指南和示例

## 🤝 参与翻译

如果您愿意帮助完善此文档的中文翻译：

1. 参考英文版本的详细技术内容
2. 翻译代码示例和技术说明
3. 保持技术术语的准确性
4. 提交 Pull Request

---

> **提示**: 详细的开发工作流程和高级示例请参考英文文档。 