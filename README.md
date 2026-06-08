# pv

> 跨平台包管理器与版本管理器 — 轻量、快速、易用

[![CI](https://github.com/KochabStar/pv/actions/workflows/ci.yml/badge.svg)](https://github.com/KochabStar/pv/actions/workflows/ci.yml)

**pv** 是一个用 Rust 编写的命令行工具，帮助你安装、切换、管理开发工具的不同版本。灵感来源于 [asdf](https://asdf-vm.com/) 和 [scoop](https://scoop.sh/)，通过 bucket 机制统一管理软件源。

---

## 特性

- **跨平台** — 支持 Windows / Linux / macOS (x64 & ARM64)
- **Bucket 管理** — 类似 scoop 的 bucket 系统，添加远程源即可获取数千软件包
- **版本切换** — 通过 shim 机制无缝切换不同版本，支持 `pv use <package>@<version>`
- **轻量快速** — Rust 实现，单二进制文件，无运行时依赖
- **一键安装** — 提供 PowerShell 安装脚本，自动配置环境变量

## 快速开始

### Windows（PowerShell）

```powershell
irm https://raw.githubusercontent.com/KochabStar/pv/main/scripts/install.ps1 | iex
```

安装完成后重启终端，即可使用：

```powershell
pv install git
pv use node@20.11.0
pv list
```

### Linux / macOS

从 [GitHub Releases](https://github.com/KochabStar/pv/releases) 下载对应平台的二进制文件：

```bash
# 以 Linux x64 为例
curl -L https://github.com/KochabStar/pv/releases/latest/download/pv-x86_64-unknown-linux-gnu.tar.gz -o pv.tar.gz
tar xzf pv.tar.gz
sudo cp pv-x86_64-unknown-linux-gnu/pv /usr/local/bin/
sudo cp pv-x86_64-unknown-linux-gnu/pv-shim /usr/local/bin/
```

## 命令参考

### 包管理

| 命令 | 说明 |
|---|---|
| `pv install <package>` | 安装最新版本 |
| `pv install <package>@<version>` | 安装指定版本 |
| `pv uninstall <package>` | 卸载包 |
| `pv use <package>@<version>` | 切换到指定版本 |
| `pv list [package]` | 列出已安装的包及版本 |
| `pv search <keyword>` | 搜索可用包 |
| `pv info <package>` | 查看包详情 |
| `pv outdated` | 检查可更新的包 |
| `pv upgrade [package]` | 升级包到最新版本 |
| `pv shell <package>@<version>` | 在当前 Shell 会话中使用指定版本 |

### Bucket 管理

| 命令 | 说明 |
|---|---|
| `pv bucket add <name> <url>` | 添加包源 |
| `pv bucket remove <name>` | 移除包源 |
| `pv bucket list` | 列出已添加的源 |
| `pv sync` | 同步所有 bucket 的最新清单 |

### 版本选择

`<package>` 支持多种版本指定方式：

```bash
pv install node              # 最新稳定版
pv install node@20           # 20.x 最新版
pv install node@20.11.0      # 精确版本
pv use node@20               # 切换到 20.x
```

## 目录结构

```
~/.pv/
├── buckets/          # bucket 仓库缓存
│   └── main/         # 默认 bucket
├── apps/             # 各包的各版本安装位置
│   └── node/
│       └── 20.11.0/
├── shims/            # shim 可执行文件
├── cache/            # 下载缓存
└── config.toml       # 全局配置
```

## 配置

`~/.pv/config.toml`：

```toml
[[buckets]]
name = "main"
url = "https://github.com/KochabStar/pv-bucket.git"

# 也可添加自定义 bucket
[[buckets]]
name = "company"
url = "git@github.com:my-org/pv-bucket.git"
```

## 构建

```bash
cargo build --release
```

产物在 `target/release/pv` 和 `target/release/pv-shim`。

## CI/CD

每次推送 `v*` 标签时自动构建并发布到 GitHub Releases，覆盖以下平台：

- Windows x64
- Linux x64
- macOS x64
- macOS ARM64

详见 [`.github/workflows/ci.yml`](.github/workflows/ci.yml)。

## 许可

[MIT](LICENSE)
