# pv — 跨平台多版本包管理器 · 设计规格

- **日期**：2026-06-05
- **状态**：设计已确认，待编写实现计划
- **项目根**：`D:\code\rust\pv`
- **工具名 / 可执行文件名**：`pv`

---

## 1. 概述与定位

`pv`（**p**ackage + **v**ersion）是一个用 Rust 编写的**独立自研**包管理器，融合 scoop（应用安装）与 mise（运行时多版本管理）的理念，但**不依赖**两者运行，拥有自己的清单格式、仓库（bucket）和安装引擎。

**核心能力**：
- 安装 / 卸载包
- 搜索包
- 多版本并存与切换（永久切换 + 临时会话切换）
- 检测版本更新

**统一模型**：同时管理「普通应用 / CLI 工具」（如 ripgrep、7zip，装一次即用）和「语言运行时 / 开发工具」（如 node、python，需多版本并存）。两类包用同一套清单与安装模型表达。

---

## 2. 关键设计决策

| 决策点 | 选择 | 说明 |
|--------|------|------|
| 定位 | 独立自研引擎 | scoop/mise 仅作理念参考，运行时不依赖 |
| 平台 | 跨平台架构，MVP 先实现 Windows | 用 trait 隔离平台差异，预留 macOS/Linux |
| 包类型 | 统一模型（应用 + 运行时） | 同一清单模型表达两类包 |
| 版本切换 | 全局激活（junction + 中央 shims）+ 临时会话切换 | **无项目配置文件**，手动切换 |
| 包源 | Git 仓库 bucket（scoop 风格） | 去中心、社区可贡献，`git pull` 同步 |
| 清单格式 | TOML | 人类友好、可注释、serde+toml 解析 |
| 更新检测 | 分阶段：MVP 比较清单版本；预留 checkver | checkver（主动探测上游）为 Phase 2 |
| 安装类型 | 压缩包 + 单文件 exe（MVP）；安装器 msi/inno（Phase 2） | 见 §7 关于安装器的权衡 |

### 技术选型（均为社区主流方案）

> ⚠️ 选型独立判断，**不**参考 `D:\code\rust` 下其他公司项目（kx-uploader、rust-study）的依赖。代码风格沿用 edition 2021 + 中文注释。

| 用途 | 选型 |
|------|------|
| CLI 解析 | clap（derive） |
| 异步运行时 | tokio |
| HTTP 下载 | reqwest（流式） |
| 清单解析 | serde + toml |
| 解压 | zip + flate2/tar |
| 校验 | sha2（sha256） |
| 进度显示 | indicatif |
| 错误处理 | anyhow（应用层）+ thiserror（库层） |
| Windows junction | junction crate（免管理员权限） |
| Git 同步 | 调用系统 `git`（std::process），MVP 免 libgit2 编译依赖 |

---

## 3. 架构与模块划分

单 crate，`lib.rs` 核心 + `main.rs` 薄 CLI 壳，便于集成测试直接驱动核心引擎。

```
src/
├── main.rs          # 薄 CLI 入口：解析参数 → 调用 lib
├── lib.rs           # 对外 API 聚合
├── cli.rs           # clap 命令定义与分发
├── config.rs        # PV_HOME 路径、config.toml、已注册 bucket
├── manifest.rs      # TOML 清单结构体 + 解析 + 校验
├── bucket.rs        # bucket 的 git 同步、清单查找、搜索
├── engine/
│   ├── mod.rs       # 安装/卸载流程编排
│   ├── download.rs  # reqwest 下载 + 进度 + sha256 校验
│   └── install/     # 安装策略（trait InstallStrategy + 多实现）
│       ├── archive.rs    # 压缩包解压        (MVP)
│       ├── single.rs     # 单文件 exe        (MVP)
│       └── installer.rs  # msi/inno 安装器    (Phase 2)
├── version.rs       # 多版本存储、激活版本、junction 重指向
├── shim.rs          # shim 代理生成
├── update.rs        # 更新检测（清单比较；预留 checkver trait）
├── platform/
│   ├── mod.rs       # Platform trait（平台差异抽象点）
│   └── windows.rs   # Windows 实现（MVP 唯一实现）
└── error.rs         # PvError 错误类型

src/bin/
└── shim.rs          # 独立 shim 代理程序，产出 pv-shim.exe
```

**SOLID 落地**：
- `InstallStrategy` trait 隔离三种安装类型——新增类型不改现有代码（O）
- `Platform` trait 隔离 OS 差异——加 macOS/Linux 只新增实现文件（O/D）
- 每个模块单一职责（S）

---

## 4. 磁盘布局

默认根目录 `%USERPROFILE%\.pv`，可由环境变量 `PV_HOME` 覆盖。

```
%PV_HOME%/
├── config.toml          # 全局设置 + 已注册 bucket 列表 + 各包激活版本记录
├── buckets/
│   └── <bucket名>/       # 每个 bucket 是一个 git clone
├── apps/
│   └── <包名>/
│       ├── 18.20.0/      # 多版本并存，各自独立目录
│       ├── 20.11.0/
│       ├── current ──→   # 目录 junction，指向当前激活版本
│       └── .manifest.toml# 缓存安装时的清单（卸载/info 用）
├── shims/                # 唯一加入用户 PATH 的目录（永不变动）
│   ├── node.exe / node.shim
│   └── npm.exe  / npm.shim
└── cache/                # 下载的压缩包缓存（可清理）
```

---

## 5. 清单（manifest）格式

bucket 里每个包一个 `.toml` 文件。

**压缩包类型**（`node.toml`）：

```toml
name = "node"
version = "20.11.0"
description = "Node.js JavaScript 运行时"
homepage = "https://nodejs.org"
license = "MIT"

type = "archive"               # archive | single | installer

[architecture.x64]             # 按架构提供下载源，MVP 先支持 x64
url  = "https://nodejs.org/dist/v20.11.0/node-v20.11.0-win-x64.zip"
hash = "sha256:abc123..."
extract_dir = "node-v20.11.0-win-x64"   # 解压内层目录提升为版本根（可选）

bin = ["node.exe", "npm.cmd", "npx.cmd"] # 暴露为 shim 的可执行文件

[env]                          # 可选：激活时注入的环境变量（$dir = 版本目录）
# NODE_HOME = "$dir"

# ── 预留 / Phase 2，MVP 不实现 ──
# [checkver]
# github = "nodejs/node"
# [installer]
# silent_args = ["/S", "/DIR=$dir"]
# uninstall   = ["$dir/uninstall.exe", "/S"]
```

**单文件 exe 类型**（`ripgrep.toml`）：

```toml
name = "ripgrep"
version = "14.1.0"
type = "single"
[architecture.x64]
url  = "https://github.com/BurntSushi/ripgrep/releases/download/14.1.0/rg.exe"
hash = "sha256:def456..."
bin  = ["rg.exe"]
```

字段语义：
- `type` 决定走哪个 `InstallStrategy`
- `bin` 决定生成哪些 shim
- `[architecture.x64]` 分节为将来 arm64/x86 预留扩展位

---

## 6. 命令集

| 命令 | 作用 |
|------|------|
| `pv install <包>[@版本]` | 安装；不带版本装清单最新版，带版本装指定版（并设为激活） |
| `pv uninstall <包>[@版本]` | 带版本只删该版本；不带版本删该包**所有版本** + shim |
| `pv search <关键词>` | 在所有 bucket 清单里搜索包 |
| `pv list [包]` | 列已安装包（标注激活版本）；带包名列该包所有已装版本 |
| `pv use <包>@<版本>` | **永久**切换全局激活版本（重指 junction + 刷新 shim） |
| `pv shell <包>@<版本>` | **临时**在当前终端会话切换（启动子 shell，关闭即失效） |
| `pv info <包>` | 显示清单详情、可用版本、已装版本 |
| `pv outdated` | **检测更新**：列出有新版本的已安装包（只报告，不安装） |
| `pv upgrade [包]` | 升级到最新（装新版 + 切换）；不带包名升级全部 |
| `pv sync` | `git pull` 同步所有 bucket，刷新本地清单库 |
| `pv bucket add <名> <url>` | 添加 git bucket 源 |
| `pv bucket list` / `pv bucket rm <名>` | 列出 / 移除 bucket |

**命令语义拆分（单一职责）**：`sync` 只同步 bucket；`outdated` 只检测并报告；`upgrade` 才安装新版 + 切换。三者解耦，比 scoop 把 sync/upgrade 合进 `scoop update` 更清晰。

---

## 7. 核心流程

### 7.1 安装 `pv install node@20.11.0`

1. 解析 `包名@版本`（不带版本 → 用清单声明的 `version`）
2. 遍历 `buckets/` 清单库，找到匹配清单
3. 若该版本已安装 → 跳过下载，直接进入切换步骤
4. 选架构（x64）→ reqwest 流式下载到 `cache/`（indicatif 进度条）
5. sha256 校验（不匹配 → 删缓存并报错）
6. 按 `type` 派发 `InstallStrategy`：
   - `archive`：解压到临时目录 →（有 `extract_dir` 则提升内层）→ 移到 `apps/<包>/<版本>/`
   - `single`：exe 直接放入 `apps/<包>/<版本>/`
   - `installer`：(Phase 2) 静默安装到版本目录
7. 写 `.manifest.toml` 缓存
8. 重指 junction：`apps/<包>/current ──→ 该版本`
9. 为 `bin` 列表生成 shim 到 `shims/`
10. 完成

### 7.2 卸载 `pv uninstall <包>[@版本]`

- 带版本：删该版本目录；若是激活版本，junction 重指到剩余某版本（无剩余则清除 junction + 移除 shim）
- 不带版本：删整个 `apps/<包>`，移除该包所有 shim，清 config 记录

### 7.3 永久切换 `pv use <包>@<版本>`

1. 校验该版本已安装
2. 重建 junction `current ──→ 版本`（先删旧 junction 再建新）
3. 若 bin 集合变化则刷新 shim
4. 更新 `config.toml` 激活记录

### 7.4 临时切换 `pv shell <包>@<版本>`

Windows 无法改父进程 PATH，故**启动一个子 shell 进程**（cmd/powershell），将该版本 bin 目录前置插入子 shell 的 PATH。退出子 shell 即恢复。这是 mise/asdf 在无 shell hook 时的标准做法，可靠且无副作用。

### 7.5 检测更新 `pv outdated`

遍历已安装包，比较 `.manifest.toml` 记录的版本 vs bucket 清单最新 `version`，列出 `installed < available` 的包（只报告）。

### 7.6 搜索 `pv search <关键词>`

扫描所有 bucket 的 `.toml`，对 `name` / `description` 做子串匹配（MVP 线性扫描，清单量不大足够）。

### 7.7 首次初始化

首次运行自动把 `%PV_HOME%\shims` 注册进用户 PATH（写 `HKCU\Environment` + 广播 `WM_SETTINGCHANGE`），仅执行一次。

---

## 8. shim 机制（Windows）

采用**独立 shim 代理程序**（主流可靠方案）：

- 本 crate 编译第二个 bin `src/bin/shim.rs` → 产出 `pv-shim.exe`
- 安装时把它**复制为 `<名>.exe`**，并写 `<名>.shim` 配置（记录目标可执行文件路径）
- 目标路径指向 `apps/<包>/current/bin/<exe>`（指向 `current` junction）

**关键收益**：shim 指向 `current` junction，`pv use` 切换版本时**只动 junction，shim 无需重建**（除非 bin 集合变化）。切换瞬间生效。

> 不采用 `.cmd/.bat` 转发：会弹黑框、影响管道、被作为 exe 调用时不兼容。

---

## 9. 平台抽象

```rust
/// 平台差异隔离点：未来加 macOS/Linux 只需新增实现
trait Platform {
    fn make_active_link(&self, target: &Path, link: &Path) -> Result<()>; // Win=junction
    fn create_shim(&self, name: &str, target: &Path) -> Result<()>;
    fn remove_shim(&self, name: &str) -> Result<()>;
    fn register_path(&self, dir: &Path) -> Result<()>;                    // 注册 shims 到 PATH
    fn exe_ext(&self) -> &str;                                            // Win=".exe"
}
```

MVP 仅实现 `WindowsPlatform`：junction crate + pv-shim.exe 复制 + 注册表改用户 PATH。

---

## 10. 错误处理

- **库层**：`thiserror` 定义 `PvError`（`NotFound` / `ManifestParse` / `Download` / `ChecksumMismatch` / `Extract` / `Junction` 等）
- **应用层**：`anyhow` 兜底，`main` 统一打印友好错误 + 非零退出码
- 校验失败删缓存并报错；下载失败 MVP 直接报错（不做自动重试，YAGNI）

---

## 11. 测试策略

- **单元测试**：清单解析、版本号比较（semver-like）、shim 路径计算
- **集成测试**：`tests/fixtures/` 放本地 fixture bucket + 本地 HTTP server（如 `tiny_http`）模拟下载源；`PV_HOME` 指向 `tempfile` 临时目录；跑完整 `install → use → list → outdated → uninstall` 全链路
- 这正是"单 crate lib/bin 分离"的价值——核心引擎可被集成测试直接驱动

---

## 12. 范围与权衡（重要）

### MVP 范围
- 平台：Windows（x64）
- 安装类型：`archive` + `single`
- 命令：install / uninstall / search / list / use / shell / info / outdated / upgrade / sync / bucket
- 更新检测：比较清单版本
- 多版本：并存 + 永久切换 + 临时会话切换

### Phase 2（预留接口，MVP 不实现）
- `checkver`：主动探测上游最新版本
- `installer` 类型（msi/inno）
- macOS / Linux 平台实现
- arm64 / x86 架构

### ⚠️ 关于安装器（msi/inno）的权衡
用户最初希望 MVP 支持安装器类型，但**安装器与「多版本并存 + 绿色可卸载」的核心理念存在根本冲突**：安装器通常往系统全局写注册表、装到 `Program Files`，难以做到多版本并存与干净卸载。

**设计决定**：将 `installer` 类型**降级为 Phase 2**，用受限策略实现（下载 → 静默安装到版本目录 → 记录卸载命令），并明确告知用户此类包可能不支持完整多版本能力。MVP 先把 `archive` 与 `single` 这两种「真·多版本」类型做扎实。

> 此项为设计期主动提出的权衡，待用户最终确认是否接受降级。
