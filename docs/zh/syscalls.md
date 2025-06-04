# elinOS 系统调用参考

## 概述

elinOS 实现了一个系统调用接口，在适当情况下借鉴 Linux 以实现兼容性，但也包含特定于操作系统的调用。本文档详细介绍了可用的系统调用、它们的编号以及当前的实现状态。

系统调用通过 `src/syscall/mod.rs` 中的中央 `syscall_handler` 进行分发。此处列出的编号和类别反映了当前被主动路由和处理的内容。

## 系统调用类别与分发

elinOS 中的系统调用大致分类。主分发器将系统调用号路由到这些类别的处理程序：

*   **文件 I/O 操作**：由 `syscall::file` 处理
*   **目录操作**：由 `syscall::directory` 处理
*   **设备和 I/O 管理**：由 `syscall::device` 处理
*   **进程管理**：由 `syscall::process` 处理
*   **时间和定时器操作**：由 `syscall::time` 处理
*   **系统信息**：由 `syscall::sysinfo` 处理
*   **网络操作**：由 `syscall::network` 处理
*   **内存管理**：由 `syscall::memory` 处理
*   **elinOS 特定操作**：由 `syscall::elinos` 处理

**关于系统调用号的说明：** 虽然许多系统调用常量在不同模块中定义（例如 `file.rs` 中的 `SYS_OPENAT`），但内核识别的实际系统调用号由 `src/syscall/mod.rs` 中的主分发器以及每个类别处理程序中的显式 `match` 分支确定。模块内编号的差异或未处理的常量是持续开发的领域。

## 已实现的系统调用

### 文件 I/O 操作
(处理程序: `syscall::file::handle_file_syscall`)

| 编号 | 名称 (常量)      | 描述                                   | 状态    |
|------|--------------------|----------------------------------------|---------|
| 35   | `SYS_UNLINK`       | 取消链接/删除文件 (unlinkat)           | 存根    |
| 45   | `SYS_TRUNCATE`     | 按路径截断文件                         | 存根    |
| 46   | `SYS_FTRUNCATE`    | 按文件描述符截断文件                   | 存根    |
| 56   | `SYS_OPENAT`       | 打开文件                               | 已实现  |
| 57   | `SYS_CLOSE`        | 关闭文件描述符                         | 已实现  |
| 61   | `SYS_GETDENTS64`   | 获取目录条目                           | 已实现  |
| 62   | `SYS_LSEEK`        | 重定位文件偏移量                       | 存根    |
| 63   | `SYS_READ`         | 从文件描述符读取                       | 已实现  |
| 64   | `SYS_WRITE`        | 写入文件描述符 (标准输出/错误输出，文件写入为存根) | 已实现  |
| 79   | `SYS_NEWFSTATAT`   | 获取文件状态 (stat)                    | 存根    |
| 81   | `SYS_SYNC`         | 同步文件系统                           | 存根    |
| 82   | `SYS_FSYNC`        | 按文件描述符同步文件数据               | 存根    |

*(其他常量如 `SYS_READV`, `SYS_WRITEV`, `SYS_FSTAT` 等在 `file.rs` 中定义，但目前未在主分发器中通过这些编号显式处理或路由。)*

### 目录操作
(处理程序: `syscall::directory::handle_directory_syscall`)

*注意：主分发器将编号 34 (原用于 `mkdirat`) 和 49-55 路由到此处理程序。但是，`directory.rs` 中的处理程序目前仅对其自己的常量 51-54 有显式匹配。*

| 编号 | 名称 (常量)   | 描述                            | 状态   |
|------|-----------------|---------------------------------|--------|
| 51   | `SYS_MKDIR`     | 创建目录                        | 存根   |
| 52   | `SYS_RMDIR`     | 删除目录                        | 存根   |
| 53   | `SYS_CHDIR`     | 更改当前目录                    | 存根   |
| 54   | `SYS_GETCWD`    | 获取当前工作目录                | 存根   |

### 设备和 I/O 管理
(处理程序: `syscall::device::handle_device_syscall`)

| 编号 | 名称 (常量)           | 描述                              | 状态   |
|------|-------------------------|-----------------------------------|--------|
| 23   | `SYS_DUP`               | 复制文件描述符                    | 存根   |
| 24   | `SYS_DUP3`              | 复制文件描述符                    | 存根   |
| 25   | `SYS_FCNTL`             | 文件控制                          | 存根   |
| 26   | `SYS_INOTIFY_INIT1`     | 初始化 inotify 实例               | 存根   |
| 27   | `SYS_INOTIFY_ADD_WATCH` |向 inotify 实例添加监视            | 存根   |
| 28   | `SYS_INOTIFY_RM_WATCH`  | 从 inotify 移除监视               | 存根   |
| 29   | `SYS_IOCTL`             | I/O 控制                          | 存根   |
| 30   | `SYS_IOPRIO_SET`        | 设置 I/O 优先级                   | 存根   |
| 31   | `SYS_IOPRIO_GET`        | 获取 I/O 优先级                   | 存根   |
| 32   | `SYS_FLOCK`             | 应用或移除劝告锁                  | 存根   |
| 33   | `SYS_MKNODAT`           | 创建设备特殊文件                  | 存根   |
| 59   | `SYS_PIPE2`             | 创建管道                          | 存根   |
| 950  | `SYS_GETDEVICES`        | 获取设备信息 (elinOS 特定)        | 已实现 |

### 进程管理
(处理程序: `syscall::process::handle_process_syscall`)

| 编号 | 名称 (常量)         | 描述                              | 状态   |
|------|-----------------------|-----------------------------------|--------|
| 93   | `SYS_EXIT`            | 终止当前进程                      | 已实现 |
| 94   | `SYS_EXIT_GROUP`      | 终止组内所有线程                  | 已实现 |
| 95   | `SYS_WAITID`          | 等待子进程状态改变                | 存根   |
| 129  | `SYS_KILL`            | 向进程发送信号                    | 存根   |
| 130  | `SYS_TKILL`           | 向线程发送信号                    | 存根   |
| 131  | `SYS_TGKILL`          | 向组内线程发送信号                | 存根   |
| 134  | `SYS_RT_SIGACTION`    | 检查/更改信号动作                 | 存根   |
| 172  | `SYS_GETPID`          | 获取进程 ID                       | 已实现 |
| 173  | `SYS_GETPPID`         | 获取父进程 ID                     | 已实现 |
| 174  | `SYS_GETUID`          | 获取真实用户 ID                   | 已实现 |
| 176  | `SYS_GETGID`          | 获取真实组 ID                     | 已实现 |
| 178  | `SYS_GETTID`          | 获取线程 ID                       | 已实现 |
| 220  | `SYS_CLONE`           | 创建子进程 (fork)                 | 存根   |
| 221  | `SYS_EXECVE`          | 执行程序                          | 存根   |

*(许多其他与进程相关的常量，如 `SYS_SET_TID_ADDRESS`, `SYS_UNSHARE`, `SYS_FUTEX`, 各种信号调用, `SYS_REBOOT`(142), UID/GID 设置调用在 `process.rs` 中定义，但在当前的 `handle_process_syscall` 中未通过这些编号显式处理，或已被 elinOS 特定版本取代。)*

### 时间和定时器操作
(处理程序: `syscall::time::handle_time_syscall`)

*注意：主分发器将 101-115 路由到此处理程序。`time.rs` 中的处理程序是一个通用存根，并不显式匹配这些编号。像 `SYS_NANOSLEEP` (101), `SYS_GETITIMER` (102), `SYS_SETITIMER` (103) 这样的常量在 `process.rs` 中定义（并根据 Linux 约定用于此范围）。像 `SYS_TIME` (271), `SYS_GETTIMEOFDAY` (272) 这样在 `time.rs` 中定义的常量目前未由主分发器通过这些编号路由到此处理程序。*

| 编号范围 | 描述                                            | 状态   |
|----------|-------------------------------------------------|--------|
| 101-115  | 用于时间/定时器操作 (例如 nanosleep, get/setitimer) | 存根   |

### 系统信息
(处理程序: `syscall::sysinfo::handle_sysinfo_syscall`)

*注意：主分发器将 160-168, 169-171, 179 路由到此处理程序。`sysinfo.rs` 中的处理程序是一个通用存根。像 `SYS_UNAME` (301), `SYS_SYSINFO` (302) 这样在 `sysinfo.rs` 中定义的常量目前未通过这些编号路由到此处理程序。*

| 编号范围            | 描述                                      | 状态   |
|---------------------|-------------------------------------------|--------|
| 160-168, 169-171, 179 | 用于系统信息操作 (例如 uname, sysinfo)    | 存根   |

### 网络操作
(处理程序: `syscall::network::handle_network_syscall`)

*注意：主分发器将 198-213 路由到此处理程序。`network.rs` 中的处理程序是一个通用存根。像 `SYS_SOCKET` (221), `SYS_BIND` (222) 这样在 `network.rs` 中定义的常量目前未通过这些编号路由到此处理程序。*

| 编号范围 | 描述                                      | 状态   |
|----------|-------------------------------------------|--------|
| 198-213  | 用于网络操作 (例如 socket, bind)          | 存根   |


### 内存管理
(处理程序: `syscall::memory::handle_memory_syscall`)

| 编号 | 名称 (常量)       | 描述                                | 状态        |
|------|---------------------|-------------------------------------|-------------|
| 214  | `SYS_BRK`           | 更改数据段大小                      | 已实现      |
| 215  | `SYS_MUNMAP`        | 取消文件或设备到内存的映射          | 已实现      |
| 216  | `SYS_MREMAP`        | 重映射虚拟内存地址                  | 存根        |
| 222  | `SYS_MMAP`          | 将文件或设备映射到内存              | 已实现 (仅匿名) |
| 226  | `SYS_MPROTECT`      | 设置内存区域的保护                  | 存根        |
| 227  | `SYS_MSYNC`         | 同步文件与内存映射                  | 存根        |
| 228  | `SYS_MLOCK`         | 锁定内存                            | 存根        |
| 229  | `SYS_MUNLOCK`       | 解锁内存                            | 存根        |
| 230  | `SYS_MLOCKALL`      | 锁定进程映射的所有内存              | 存根        |
| 231  | `SYS_MUNLOCKALL`    | 解锁进程映射的所有内存              | 存根        |
| 232  | `SYS_MINCORE`       | 判断页面的内存驻留情况              | 存根        |
| 233  | `SYS_MADVISE`       | 提供关于内存使用的建议              | 存根        |
| 960  | `SYS_GETMEMINFO`    | 获取内存信息 (elinOS 特定)          | 已实现      |

*(其他常量如 `SYS_ADD_KEY`, `SYS_SWAPON` 等在 `memory.rs` 中定义但目前未显式处理。`SYS_ALLOC_TEST` (961) 和 `SYS_BUDDY_STATS` (962) 在 `memory.rs` 中定义和处理，但未由主分发器路由到那里；它们将属于 elinOS 特定范围 900-999，其处理程序不调用它们。)*


### elinOS 特定操作
(处理程序: `syscall::elinos::handle_elinos_syscall`)

| 编号 | 名称 (常量)           | 描述                                 | 状态        |
|------|-------------------------|--------------------------------------|-------------|
| 900  | `SYS_ELINOS_DEBUG`      | 向控制台打印调试消息                 | 已实现      |
| 901  | `SYS_ELINOS_STATS`      | (主分发器没有到实现的特定处理程序)   | 仅定义      |
| 902  | `SYS_ELINOS_VERSION`    | 显示 elinOS 版本信息                 | 已实现      |
| 903  | `SYS_ELINOS_SHUTDOWN`   | 通过 SBI 关闭系统                    | 已实现      |
| 904  | `SYS_ELINOS_REBOOT`     | 通过 SBI 重启系统                    | 已实现      |
| 905  | `SYS_LOAD_ELF`          | 加载 ELF 二进制文件 (elinOS 特定)    | 已实现      |
| 906  | `SYS_EXEC_ELF`          | “执行”已加载的 ELF (elinOS 特定)     | 已实现 (模拟) |
| 907  | `SYS_ELF_INFO`          | 获取 ELF 二进制信息 (elinOS 特定)    | 已实现      |


---

*elinOS 系统调用接口提供了一个熟悉的 Linux 兼容 API，同时展示了现代内核设计原则，并作为理解操作系统内部原理的优秀实验资源。* 