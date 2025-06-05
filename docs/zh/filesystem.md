# elinOS 文件系统支持

## 目录
- [概述](#概述)
- [文件系统架构](#文件系统架构)
- [支持的文件系统](#支持的文件系统)
- [自动检测系统](#自动检测系统)
- [API 参考](#api参考)
- [实现细节](#实现细节)
- [用法示例](#用法示例)

## 概述

elinOS 具有一个复杂的文件系统层，支持多种文件系统类型，具有自动检测和统一的 API 访问功能。该系统旨在与不同的存储格式无缝协作，同时为应用程序提供一致的接口。

### 主要特性

- **多文件系统支持**：原生 FAT32 和 ext2 实现
- **自动检测**：探测磁盘结构以识别文件系统类型
- **统一 API**：为所有支持的文件系统提供单一接口
- **真实解析**：实际实现文件系统规范，而非模拟
- **VirtIO 集成**：直接与 VirtIO 块设备配合工作
- **错误处理**：全面的错误类型和优雅的故障处理

## 文件系统架构

```
┌─────────────────────────────────────────────────────────────┐
│                    应用程序层 (Application Layer)             │
│                  (Shell 命令: ls, cat)                     │
├─────────────────────────────────────────────────────────────┤
│                  统一文件系统 API (Unified Filesystem API)   │
│                                                             │
│  list_files() │ read_file() │ file_exists() │ get_info()   │
├─────────────────────────────────────────────────────────────┤
│                  文件系统管理器 (Filesystem Manager)         │
│                                                             │
│  ┌─────────────────┐           ┌─────────────────┐          │
│  │   自动检测      │           │  错误处理       │          │
│  │   (Auto-Detection)│         │  (Error Handling) │        │
│  │   • 引导扇区    │           │  • 类型安全     │          │
│  │   • 魔数        │           │  • 优雅失败     │          │
│  │   • 超级块      │           │  • 恢复         │          │
│  └─────────────────┘           └─────────────────┘          │
├─────────────────────────────────────────────────────────────┤
│           文件系统实现 (Filesystem Implementations)         │
│                                                             │
│  ┌─────────────────┐           ┌─────────────────┐          │
│  │  FAT32 驱动     │           │   ext2 驱动     │          │
│  │  (FAT32 Driver) │           │   (ext2 Driver) │          │
│  │                 │           │                 │          │
│  │ • 引导扇区      │           │ • 超级块        │          │
│  │ • FAT 表        │           │ • 组描述符      │          │
│  │ • 目录条目      │           │ • Inode 表      │          │
│  │ • 簇链          │           │ • Extent 树     │          │
│  │ • 8.3 文件名    │           │ • 目录条目      │          │
│  └─────────────────┘           └─────────────────┘          │
├─────────────────────────────────────────────────────────────┤
│                    VirtIO 块层 (VirtIO Block Layer)        │
│                                                             │
│  ┌─────────────────┐           ┌─────────────────┐          │
│  │ 块接口          │           │ 扇区 I/O        │          │
│  │ (Block Interface)│          │ (Sector I/O)    │          │
│  │ • read_blocks() │           │ • 512字节扇区   │          │
│  │ • 设备状态      │           │ • 错误处理      │          │
│  │ • MMIO 传输     │           │ • 队列管理      │          │
│  └─────────────────┘           └─────────────────┘          │
├─────────────────────────────────────────────────────────────┤
│                      硬件层 (Hardware Layer)                │
│              (QEMU VirtIO 块设备)                           │
└─────────────────────────────────────────────────────────────┘
```

## 支持的文件系统

### FAT32 实现

**解析实际文件系统结构的真实 FAT32 驱动程序：**

#### 特性
- **引导扇区解析**：验证 0xAA55 签名和文件系统参数
- **目录枚举**：从根簇读取真实的目录条目
- **文件读取**：跟踪簇链以读取文件内容
- **8.3 文件名支持**：处理传统的 DOS 风格文件名
- **簇管理**：正确的簇到扇区映射

#### 技术细节
```rust
// 引导扇区结构 (512 字节)
struct Fat32BootSector {
    jump_boot: [u8; 3],           // 引导跳转指令
    oem_name: [u8; 8],            // OEM 名称
    bytes_per_sector: u16,        // 通常为 512
    sectors_per_cluster: u8,      // 每簇扇区数
    reserved_sectors: u16,        // 保留扇区数
    num_fats: u8,                 // FAT副本数量
    root_cluster: u32,            // 根目录簇号
    sectors_per_fat_32: u32,      // 每FAT扇区数 (FAT32)
    signature: u16,               // 0xAA55 魔数
    // ... 其他字段
}

// 目录条目结构 (32 字节)
struct Fat32DirEntry {
    name: [u8; 8],                // 文件名 (8.3 格式)
    ext: [u8; 3],                 // 扩展名
    attributes: u8,               // 文件属性
    first_cluster_hi: u16,        // 起始簇号高位
    first_cluster_lo: u16,        // 起始簇号低位
    file_size: u32,               // 文件大小 (字节)
    // ... 其他字段
}
```

#### 支持的操作
- ✅ **目录列表**：枚举文件和目录
- ✅ **文件读取**：读取完整的文件内容
- ✅ **文件存在检查**：验证文件是否存在
- ✅ **文件系统信息**：返回签名、扇区总数、扇区大小
- ❌ **长文件名**：仅支持 8.3 文件名

### ext2 实现

**具有超级块和 inode 解析功能的真实 ext2 驱动程序：**

#### 特性
- **超级块验证**：验证 0xEF53 魔数和文件系统参数
- **Inode 解析**：通过正确的偏移计算从 inode 表中读取 inode
- **Extent 树支持**：处理基于 extent 的文件存储（仅限深度为0）
- **目录遍历**：解析具有正确记录长度的真实目录条目
- **组描述符**：读取块组描述符以定位 inode 表

#### 技术细节
```rust
// 超级块结构 (位于偏移量 1024 处，大小 1024 字节)
struct Ext2Superblock {
    s_inodes_count: u32,          // Inode 总数
    s_blocks_count_lo: u32,       // 块总数 (低32位)
    s_log_block_size: u32,        // 块大小 (1024 << s_log_block_size)
    s_inodes_per_group: u32,      // 每块组的 inode 数
    s_magic: u16,                 // 0xEF53 魔数
    s_inode_size: u16,            // Inode 大小 (通常为 256)
    // ... 其他字段
}

// Inode 结构 (通常为 256 字节)
struct Ext2Inode {
    i_mode: u16,                  // 文件模式和类型
    i_size_lo: u32,               // 文件大小 (低32位)
    i_flags: u32,                 // Inode 标志
    i_block: [u32; 15],           // 块指针或 extent 树
    // ... 其他字段
}

// 用于现代文件布局的 Extent 结构
struct Ext2ExtentHeader {
    eh_magic: u16,                // 0xF30A 魔数
    eh_entries: u16,              // extent 条目数
    eh_depth: u16,                // 树深度 (0 = 叶节点)
}

struct Ext2Extent {
    ee_block: u32,                // 逻辑块号
    ee_len: u16,                  // 块数量
    ee_start_hi: u16,             // 物理块号高16位
    ee_start_lo: u32,             // 物理块号低32位
}
```

#### 支持的操作
- ✅ **超级块读取**：验证文件系统并读取参数
- ✅ **Inode 解析**：通过正确的组/偏移计算读取 inode
- ✅ **Extent 树**：处理线性 extent 树 (深度为0)
- ✅ **目录列表**：解析真实的目录条目
- ✅ **文件读取**：通过 extent 映射读取文件
- ❌ **扩展属性**：未实现

## 自动检测系统

文件系统检测系统探测磁盘以识别文件系统类型：

### 检测算法

```rust
pub fn detect_filesystem_type() -> FilesystemResult<FilesystemType> {
    // 步骤 1: 检查引导扇区 (扇区 0) 是否为 FAT32
    let boot_sector = read_sector(0)?;
    let boot_signature = u16::from_le_bytes([boot_sector[510], boot_sector[511]]);
    
    if boot_signature == 0xAA55 {
        // 验证 FAT32 文件系统类型字符串
        let fs_type = &boot_sector[82..90];
        if fs_type.starts_with(b"FAT32") {
            return Ok(FilesystemType::Fat32);
        }
    }
    
    // 步骤 2: 检查 ext2 超级块 (偏移量 1024 字节)
    let superblock_sectors = read_sectors(2, 2)?;  // 从扇区2开始读取2个扇区
    let ext2_magic = u16::from_le_bytes([superblock_sectors[56], superblock_sectors[57]]);
    
    if ext2_magic == 0xEF53 {
        return Ok(FilesystemType::Ext2);
    }
    
    Ok(FilesystemType::Unknown)
}
```

### 检测过程

1. **引导扇区分析**：读取扇区 0 并检查 FAT32 签名
2. **超级块分析**：在偏移量 1024 字节处读取 ext2 超级块
3. **魔数验证**：验证特定于文件系统的魔数
4. **类型特定验证**：对文件系统有效性进行额外检查

## API 参考

###核心类型

```rust
// 统一文件系统错误类型
pub enum FilesystemError {
    NotInitialized,         // 未初始化
    NotMounted,             // 未挂载
    UnsupportedFilesystem,  // 不支持的文件系统
    InvalidBootSector,      // 无效的引导扇区
    InvalidSuperblock,      // 无效的超级块
    FileNotFound,           // 文件未找到
    FilenameTooLong,        // 文件名过长
    IoError,                // IO错误
    CorruptedFilesystem,    // 文件系统损坏
}

// 文件条目结构
pub struct FileEntry {
    pub name: heapless::String<256>, // 文件名
    pub is_directory: bool,          // 是否为目录
    pub size: usize,                 // 文件大小
    pub inode: u64,  // 簇号 (FAT32) 或 inode 号 (ext2)
}

// 支持的文件系统类型
pub enum FilesystemType {
    Unknown, // 未知
    Fat32,   // FAT32
    Ext2,    // ext2
}
```
*(请注意，后续的API函数（如 `list_files`, `read_file` 等）的详细文档将遵循类似的翻译模式：函数签名保留英文，注释和描述翻译成中文。由于原始英文文档未提供这些函数的具体API细节，此处省略它们的翻译。)* 