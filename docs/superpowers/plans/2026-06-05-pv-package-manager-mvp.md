# pv Package Manager MVP Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the Windows x64 MVP of `pv`, a Rust package manager that installs, searches, switches, upgrades, and uninstalls application/runtime packages from Git bucket manifests.

**Architecture:** Single Rust crate with a thin `main.rs` CLI and testable library modules. The engine coordinates config, bucket lookup, download/checksum, install strategies, active-version links, shims, and update detection; platform-specific behavior is isolated behind a `Platform` trait with a Windows implementation and test doubles.

**Tech Stack:** Rust edition 2021, clap, tokio, reqwest, serde, toml, zip, flate2/tar, sha2, indicatif, anyhow, thiserror, junction, semver, unrar-ng, tempfile, assert_cmd, tiny_http.

---

## Source Spec

- Design spec: `docs/superpowers/specs/2026-06-05-pv-package-manager-design.md`
- Project root: `D:/code/rust/pv`
- Binary names: `pv` and `pv-shim`
- Repository rule: do not add `git commit`, branch, push, reset, or checkout steps unless the user explicitly asks for them.

## File Structure

- Create `Cargo.toml`: crate metadata, runtime dependencies, dev dependencies, binary declarations.
- Create `src/main.rs`: thin async entry point that prints friendly errors and exits non-zero.
- Create `src/lib.rs`: public module aggregation for tests and CLI.
- Create `src/cli.rs`: clap command definitions and command dispatch.
- Create `src/error.rs`: library error enum using `thiserror`.
- Create `src/package.rs`: parse `<name>[@version]` and `<name>@<version>`.
- Create `src/config.rs`: `PV_HOME` paths, persisted `config.toml`, registered buckets, active versions, PATH registration flag.
- Create `src/manifest.rs`: TOML manifest model, parsing, validation, x64 architecture selection.
- Create `src/bucket.rs`: bucket add/remove/list/sync, manifest scanning, search, package lookup.
- Create `src/platform/mod.rs`: `Platform` trait and test helpers.
- Create `src/platform/windows.rs`: Windows junction, shim copy/config removal, user PATH registration, shell process helpers.
- Create `src/version.rs`: installed-version discovery, active-version switching, list/uninstall support.
- Create `src/shim.rs`: shim config generation and target path calculation.
- Create `src/bin/shim.rs`: `pv-shim` proxy executable.
- Create `src/update.rs`: outdated detection and upgrade planning.
- Create `src/engine/mod.rs`: install/use/uninstall/list/info/search/upgrade orchestration.
- Create `src/engine/download.rs`: reqwest streaming download, sha256 validation, cache cleanup.
- Create `src/engine/install/mod.rs`: `InstallStrategy` trait and dispatcher.
- Create `src/engine/install/archive.rs`: zip/tar archive extraction.
- Create `src/engine/install/single.rs`: single executable installation.
- Create `src/engine/install/installer.rs`: inno/msi extraction command building, `innounp` bootstrap, and execution.
- Create `tests/manifest_tests.rs`: manifest parsing and validation coverage.
- Create `tests/config_tests.rs`: path/config persistence coverage.
- Create `tests/bucket_tests.rs`: bucket scanning/search coverage.
- Create `tests/package_tests.rs`: package spec parser coverage.
- Create `tests/version_tests.rs`: activation/listing behavior using a fake platform.
- Create `tests/download_tests.rs`: checksum behavior and local HTTP download coverage.
- Create `tests/shim_tests.rs`: shim config and proxy argument behavior.
- Create `tests/cli_tests.rs`: CLI parser and user-visible command smoke tests.
- Create `tests/integration_lifecycle.rs`: fixture bucket plus local HTTP server covering `install -> use -> list -> outdated -> upgrade -> uninstall`.
- Create fixture files under `tests/fixtures/`: bucket manifests and small downloadable archives/executables.

## Cross-Cutting Implementation Rules

- All command implementations must use `PV_HOME` from the environment when present.
- MVP architecture is Windows x64. Non-Windows platform functions return explicit unsupported errors instead of pretending success.
- Manifest `bin` is accepted both at top level and under `[architecture.x64]`; architecture-level `bin` wins when present.
- `sync` only updates buckets, `outdated` only reports, and `upgrade` installs and switches.
- Installer packages are extraction-only: `msiexec /a` for MSI and `innounp` for Inno Setup.
- Inno packages bootstrap `innounp` on first use into `PV_HOME/tools/innounp/innounp.exe` from the pinned SourceForge `innounp050.rar`, verify the archive hash, and reuse that local tool afterwards.
- No project-local version file, shell hook, or automatic retry logic is added in MVP.

---

### Task 1: Bootstrap The Rust Crate

**Files:**
- Create: `Cargo.toml`
- Create: `src/lib.rs`
- Create: `src/main.rs`
- Create: `src/error.rs`
- Create: `src/cli.rs`
- Test: `tests/cli_tests.rs`

- [ ] **Step 1: Write the CLI smoke tests**

Create `tests/cli_tests.rs`:

```rust
use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn help_mentions_core_commands() {
    let mut cmd = Command::cargo_bin("pv").expect("pv binary is built");

    cmd.arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("install"))
        .stdout(predicate::str::contains("bucket"))
        .stdout(predicate::str::contains("outdated"));
}

#[test]
fn version_command_prints_binary_name() {
    let mut cmd = Command::cargo_bin("pv").expect("pv binary is built");

    cmd.arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("pv"));
}
```

- [ ] **Step 2: Run the smoke tests and verify crate files are missing**

Run: `cargo test --test cli_tests`

Expected: FAIL because `Cargo.toml` and the `pv` binary do not exist yet.

- [ ] **Step 3: Create `Cargo.toml`**

Create `Cargo.toml`:

```toml
[package]
name = "pv"
version = "0.1.0"
edition = "2021"
description = "Cross-platform package and version manager"
license = "MIT"

[lib]
name = "pv"
path = "src/lib.rs"

[[bin]]
name = "pv"
path = "src/main.rs"

[[bin]]
name = "pv-shim"
path = "src/bin/shim.rs"

[dependencies]
anyhow = "1.0"
clap = { version = "4.5", features = ["derive"] }
dirs = "5.0"
flate2 = "1.0"
futures-util = "0.3"
indicatif = "0.17"
junction = "1.0"
reqwest = { version = "0.12", default-features = false, features = ["rustls-tls", "stream", "blocking"] }
semver = "1.0"
serde = { version = "1.0", features = ["derive"] }
sha2 = "0.10"
tar = "0.4"
thiserror = "1.0"
tokio = { version = "1.0", features = ["fs", "io-util", "macros", "process", "rt-multi-thread"] }
toml = "0.8"
unrar-ng = "0.7"
walkdir = "2.5"
zip = "2.1"

[dev-dependencies]
assert_cmd = "2.0"
predicates = "3.1"
tempfile = "3.10"
tiny_http = "0.12"
```

- [ ] **Step 4: Create the library shell**

Create `src/lib.rs`:

```rust
pub mod bucket;
pub mod cli;
pub mod config;
pub mod engine;
pub mod error;
pub mod manifest;
pub mod package;
pub mod platform;
pub mod shim;
pub mod update;
pub mod version;
```

Create `src/error.rs`:

```rust
use std::path::PathBuf;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum PvError {
    #[error("package not found: {0}")]
    PackageNotFound(String),
    #[error("version not found for {package}: {version}")]
    VersionNotFound { package: String, version: String },
    #[error("invalid package spec: {0}")]
    InvalidPackageSpec(String),
    #[error("manifest parse failed for {path}: {source}")]
    ManifestParse {
        path: PathBuf,
        source: toml::de::Error,
    },
    #[error("manifest validation failed for {path}: {message}")]
    ManifestValidation { path: PathBuf, message: String },
    #[error("download failed: {0}")]
    Download(String),
    #[error("checksum mismatch for {path}: expected {expected}, actual {actual}")]
    ChecksumMismatch {
        path: PathBuf,
        expected: String,
        actual: String,
    },
    #[error("extract failed for {path}: {message}")]
    Extract { path: PathBuf, message: String },
    #[error("platform operation failed: {0}")]
    Platform(String),
    #[error("unsupported platform operation: {0}")]
    UnsupportedPlatform(String),
    #[error("external command failed: {program} {args:?}, status: {status}")]
    CommandFailed {
        program: String,
        args: Vec<String>,
        status: String,
    },
    #[error("io failed for {path}: {source}")]
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
}

pub type Result<T> = std::result::Result<T, PvError>;
```

- [ ] **Step 5: Create the thin CLI entry point and initial parser**

Create `src/main.rs`:

```rust
#[tokio::main]
async fn main() {
    if let Err(error) = pv::cli::run().await {
        eprintln!("pv: {error:#}");
        std::process::exit(1);
    }
}
```

Create `src/cli.rs`:

```rust
use anyhow::Result;
use clap::{Args, Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "pv", version, about = "Package and version manager")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    Install(PackageArg),
    Uninstall(PackageArg),
    Search(SearchArg),
    List(ListArg),
    Use(ExactPackageArg),
    Shell(ExactPackageArg),
    Info(InfoArg),
    Outdated,
    Upgrade(UpgradeArg),
    Sync,
    Bucket(BucketCommand),
}

#[derive(Debug, Args)]
pub struct PackageArg {
    pub package: String,
}

#[derive(Debug, Args)]
pub struct ExactPackageArg {
    pub package: String,
}

#[derive(Debug, Args)]
pub struct SearchArg {
    pub keyword: String,
}

#[derive(Debug, Args)]
pub struct ListArg {
    pub package: Option<String>,
}

#[derive(Debug, Args)]
pub struct InfoArg {
    pub package: String,
}

#[derive(Debug, Args)]
pub struct UpgradeArg {
    pub package: Option<String>,
}

#[derive(Debug, Subcommand)]
pub enum BucketSubcommand {
    Add(BucketAddArg),
    List,
    Rm(BucketRemoveArg),
}

#[derive(Debug, Args)]
pub struct BucketCommand {
    #[command(subcommand)]
    pub command: BucketSubcommand,
}

#[derive(Debug, Args)]
pub struct BucketAddArg {
    pub name: String,
    pub url: String,
}

#[derive(Debug, Args)]
pub struct BucketRemoveArg {
    pub name: String,
}

pub async fn run() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Install(arg) => println!("install {}", arg.package),
        Commands::Uninstall(arg) => println!("uninstall {}", arg.package),
        Commands::Search(arg) => println!("search {}", arg.keyword),
        Commands::List(arg) => println!("list {}", arg.package.unwrap_or_else(|| "all".to_string())),
        Commands::Use(arg) => println!("use {}", arg.package),
        Commands::Shell(arg) => println!("shell {}", arg.package),
        Commands::Info(arg) => println!("info {}", arg.package),
        Commands::Outdated => println!("outdated"),
        Commands::Upgrade(arg) => println!("upgrade {}", arg.package.unwrap_or_else(|| "all".to_string())),
        Commands::Sync => println!("sync"),
        Commands::Bucket(arg) => match arg.command {
            BucketSubcommand::Add(add) => println!("bucket add {} {}", add.name, add.url),
            BucketSubcommand::List => println!("bucket list"),
            BucketSubcommand::Rm(remove) => println!("bucket rm {}", remove.name),
        },
    }

    Ok(())
}
```

- [ ] **Step 6: Add temporary module shells required by `src/lib.rs`**

Create each listed file with the exact content `pub fn module_loaded() -> bool { true }`:

```text
src/bucket.rs
src/config.rs
src/engine/mod.rs
src/manifest.rs
src/package.rs
src/platform/mod.rs
src/shim.rs
src/update.rs
src/version.rs
```

Create `src/bin/shim.rs`:

```rust
fn main() {
    eprintln!("pv-shim is not configured");
    std::process::exit(1);
}
```

- [ ] **Step 7: Run the smoke tests**

Run: `cargo test --test cli_tests`

Expected: PASS, both smoke tests succeed.

---

### Task 2: Parse Package Coordinates

**Files:**
- Modify: `src/package.rs`
- Test: `tests/package_tests.rs`

- [ ] **Step 1: Write package parser tests**

Create `tests/package_tests.rs`:

```rust
use pv::package::{ExactPackageSpec, PackageSpec};

#[test]
fn parses_name_without_version() {
    let spec = PackageSpec::parse("node").expect("valid package spec");

    assert_eq!(spec.name, "node");
    assert_eq!(spec.version.as_deref(), None);
}

#[test]
fn parses_name_with_version() {
    let spec = PackageSpec::parse("node@20.11.0").expect("valid package spec");

    assert_eq!(spec.name, "node");
    assert_eq!(spec.version.as_deref(), Some("20.11.0"));
}

#[test]
fn exact_spec_requires_version() {
    let spec = ExactPackageSpec::parse("node@20.11.0").expect("valid exact spec");

    assert_eq!(spec.name, "node");
    assert_eq!(spec.version, "20.11.0");
    assert!(ExactPackageSpec::parse("node").is_err());
}

#[test]
fn rejects_empty_names_and_versions() {
    assert!(PackageSpec::parse("").is_err());
    assert!(PackageSpec::parse("@20.11.0").is_err());
    assert!(PackageSpec::parse("node@").is_err());
    assert!(PackageSpec::parse("node@20@extra").is_err());
}
```

- [ ] **Step 2: Run parser tests and verify failure**

Run: `cargo test --test package_tests`

Expected: FAIL because `PackageSpec` and `ExactPackageSpec` are not defined.

- [ ] **Step 3: Implement package parsing**

Replace `src/package.rs`:

```rust
use crate::error::{PvError, Result};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageSpec {
    pub name: String,
    pub version: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExactPackageSpec {
    pub name: String,
    pub version: String,
}

impl PackageSpec {
    pub fn parse(input: &str) -> Result<Self> {
        let trimmed = input.trim();
        if trimmed.is_empty() {
            return Err(PvError::InvalidPackageSpec(input.to_string()));
        }

        let parts: Vec<&str> = trimmed.split('@').collect();
        match parts.as_slice() {
            [name] if !name.is_empty() => Ok(Self {
                name: (*name).to_string(),
                version: None,
            }),
            [name, version] if !name.is_empty() && !version.is_empty() => Ok(Self {
                name: (*name).to_string(),
                version: Some((*version).to_string()),
            }),
            _ => Err(PvError::InvalidPackageSpec(input.to_string())),
        }
    }
}

impl ExactPackageSpec {
    pub fn parse(input: &str) -> Result<Self> {
        let spec = PackageSpec::parse(input)?;
        let version = spec
            .version
            .ok_or_else(|| PvError::InvalidPackageSpec(input.to_string()))?;

        Ok(Self {
            name: spec.name,
            version,
        })
    }
}
```

- [ ] **Step 4: Run parser tests**

Run: `cargo test --test package_tests`

Expected: PASS, all parser tests succeed.

---

### Task 3: Persist PV_HOME Configuration

**Files:**
- Modify: `src/config.rs`
- Test: `tests/config_tests.rs`

- [ ] **Step 1: Write config path and persistence tests**

Create `tests/config_tests.rs`:

```rust
use pv::config::{BucketConfig, Config, Paths};
use tempfile::tempdir;

#[test]
fn builds_paths_from_home() {
    let home = tempdir().expect("tempdir");
    let paths = Paths::from_home(home.path());

    assert_eq!(paths.home, home.path());
    assert_eq!(paths.config_file, home.path().join("config.toml"));
    assert_eq!(paths.buckets, home.path().join("buckets"));
    assert_eq!(paths.apps, home.path().join("apps"));
    assert_eq!(paths.shims, home.path().join("shims"));
    assert_eq!(paths.cache, home.path().join("cache"));
    assert_eq!(paths.tools, home.path().join("tools"));
}

#[test]
fn creates_directories_and_saves_config() {
    let home = tempdir().expect("tempdir");
    let paths = Paths::from_home(home.path());

    let mut config = Config::load_or_default(&paths).expect("load config");
    config.add_bucket("main", "https://example.invalid/main.git").expect("add bucket");
    config.set_active_version("node", "20.11.0");
    config.save(&paths).expect("save config");

    let loaded = Config::load_or_default(&paths).expect("reload config");

    assert_eq!(loaded.buckets[0].name, "main");
    assert_eq!(loaded.active_version("node"), Some("20.11.0"));
    assert!(paths.buckets.exists());
    assert!(paths.apps.exists());
    assert!(paths.shims.exists());
    assert!(paths.cache.exists());
    assert!(paths.tools.exists());
}

#[test]
fn rejects_duplicate_bucket_names() {
    let mut config = Config::default();

    config.add_bucket("main", "https://example.invalid/main.git").expect("first add");

    assert!(config.add_bucket("main", "https://example.invalid/other.git").is_err());
}

#[test]
fn removes_bucket_by_name() {
    let mut config = Config {
        buckets: vec![BucketConfig {
            name: "main".to_string(),
            url: "https://example.invalid/main.git".to_string(),
        }],
        active_versions: Default::default(),
        path_registered: false,
    };

    config.remove_bucket("main").expect("remove bucket");

    assert!(config.buckets.is_empty());
}
```

- [ ] **Step 2: Run config tests and verify failure**

Run: `cargo test --test config_tests`

Expected: FAIL because `Paths`, `Config`, and `BucketConfig` are not implemented.

- [ ] **Step 3: Implement config paths and persistence**

Replace `src/config.rs`:

```rust
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::{PvError, Result};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Paths {
    pub home: PathBuf,
    pub config_file: PathBuf,
    pub buckets: PathBuf,
    pub apps: PathBuf,
    pub shims: PathBuf,
    pub cache: PathBuf,
    pub tools: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BucketConfig {
    pub name: String,
    pub url: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub buckets: Vec<BucketConfig>,
    pub active_versions: BTreeMap<String, String>,
    pub path_registered: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            buckets: Vec::new(),
            active_versions: BTreeMap::new(),
            path_registered: false,
        }
    }
}

impl Paths {
    pub fn discover() -> Result<Self> {
        if let Ok(home) = std::env::var("PV_HOME") {
            return Ok(Self::from_home(home));
        }

        let user_home = dirs::home_dir()
            .ok_or_else(|| PvError::Platform("unable to determine user home directory".to_string()))?;
        Ok(Self::from_home(user_home.join(".pv")))
    }

    pub fn from_home(path: impl AsRef<Path>) -> Self {
        let home = path.as_ref().to_path_buf();
        Self {
            config_file: home.join("config.toml"),
            buckets: home.join("buckets"),
            apps: home.join("apps"),
            shims: home.join("shims"),
            cache: home.join("cache"),
            tools: home.join("tools"),
            home,
        }
    }

    pub fn ensure_dirs(&self) -> Result<()> {
        for path in [&self.home, &self.buckets, &self.apps, &self.shims, &self.cache, &self.tools] {
            fs::create_dir_all(path).map_err(|source| PvError::Io {
                path: path.clone(),
                source,
            })?;
        }
        Ok(())
    }
}

impl Config {
    pub fn load_or_default(paths: &Paths) -> Result<Self> {
        paths.ensure_dirs()?;

        if !paths.config_file.exists() {
            return Ok(Self::default());
        }

        let content = fs::read_to_string(&paths.config_file).map_err(|source| PvError::Io {
            path: paths.config_file.clone(),
            source,
        })?;

        toml::from_str(&content).map_err(|source| PvError::ManifestParse {
            path: paths.config_file.clone(),
            source,
        })
    }

    pub fn save(&self, paths: &Paths) -> Result<()> {
        paths.ensure_dirs()?;
        let content = toml::to_string_pretty(self)
            .map_err(|error| PvError::Platform(format!("serialize config failed: {error}")))?;

        fs::write(&paths.config_file, content).map_err(|source| PvError::Io {
            path: paths.config_file.clone(),
            source,
        })
    }

    pub fn add_bucket(&mut self, name: &str, url: &str) -> Result<()> {
        if self.buckets.iter().any(|bucket| bucket.name == name) {
            return Err(PvError::Platform(format!("bucket already exists: {name}")));
        }

        self.buckets.push(BucketConfig {
            name: name.to_string(),
            url: url.to_string(),
        });
        Ok(())
    }

    pub fn remove_bucket(&mut self, name: &str) -> Result<()> {
        let before = self.buckets.len();
        self.buckets.retain(|bucket| bucket.name != name);

        if self.buckets.len() == before {
            return Err(PvError::Platform(format!("bucket not found: {name}")));
        }

        Ok(())
    }

    pub fn set_active_version(&mut self, package: &str, version: &str) {
        self.active_versions
            .insert(package.to_string(), version.to_string());
    }

    pub fn clear_active_version(&mut self, package: &str) {
        self.active_versions.remove(package);
    }

    pub fn active_version(&self, package: &str) -> Option<&str> {
        self.active_versions.get(package).map(String::as_str)
    }
}
```

- [ ] **Step 4: Run config tests**

Run: `cargo test --test config_tests`

Expected: PASS, all config tests succeed.

---

### Task 4: Parse And Validate TOML Manifests

**Files:**
- Modify: `src/manifest.rs`
- Test: `tests/manifest_tests.rs`

- [ ] **Step 1: Write manifest parsing tests**

Create `tests/manifest_tests.rs`:

```rust
use std::fs;

use pv::manifest::{InstallerKind, Manifest, PackageType};
use tempfile::tempdir;

#[test]
fn parses_archive_manifest_with_top_level_bin() {
    let manifest = Manifest::parse_str(
        r#"
name = "node"
version = "20.11.0"
description = "Node.js JavaScript runtime"
homepage = "https://nodejs.org"
license = "MIT"
type = "archive"
bin = ["node.exe", "npm.cmd"]

[architecture.x64]
url = "https://example.invalid/node.zip"
hash = "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
extract_dir = "node-v20.11.0-win-x64"

[env]
NODE_HOME = "$dir"
"#,
    )
    .expect("manifest parses");

    let arch = manifest.x64().expect("x64 architecture exists");

    assert_eq!(manifest.name, "node");
    assert_eq!(manifest.package_type, PackageType::Archive);
    assert_eq!(arch.url, "https://example.invalid/node.zip");
    assert_eq!(arch.bin, vec!["node.exe", "npm.cmd"]);
    assert_eq!(arch.extract_dir.as_deref(), Some("node-v20.11.0-win-x64"));
    assert_eq!(manifest.env.get("NODE_HOME").map(String::as_str), Some("$dir"));
}

#[test]
fn parses_installer_manifest_with_arch_bin() {
    let manifest = Manifest::parse_str(
        r#"
name = "mytool"
version = "3.2.1"
type = "installer"
installer = "inno"

[architecture.x64]
url = "https://example.invalid/mytool.exe"
hash = "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
bin = ["mytool.exe"]
"#,
    )
    .expect("manifest parses");

    assert_eq!(manifest.package_type, PackageType::Installer);
    assert_eq!(manifest.installer, Some(InstallerKind::Inno));
    assert_eq!(manifest.x64().expect("x64").bin, vec!["mytool.exe"]);
}

#[test]
fn rejects_missing_x64_architecture() {
    let error = Manifest::parse_str(
        r#"
name = "rg"
version = "14.1.0"
type = "single"
"#,
    )
    .expect_err("missing architecture fails");

    assert!(error.to_string().contains("architecture.x64"));
}

#[test]
fn loads_manifest_from_file() {
    let dir = tempdir().expect("tempdir");
    let path = dir.path().join("rg.toml");
    fs::write(
        &path,
        r#"
name = "ripgrep"
version = "14.1.0"
description = "search tool"
type = "single"

[architecture.x64]
url = "https://example.invalid/rg.exe"
hash = "sha256:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc"
bin = ["rg.exe"]
"#,
    )
    .expect("write manifest");

    let manifest = Manifest::from_path(&path).expect("load manifest");

    assert_eq!(manifest.name, "ripgrep");
}
```

- [ ] **Step 2: Run manifest tests and verify failure**

Run: `cargo test --test manifest_tests`

Expected: FAIL because the manifest model is not implemented.

- [ ] **Step 3: Implement manifest model and validation**

Replace `src/manifest.rs`:

```rust
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::error::{PvError, Result};

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PackageType {
    Archive,
    Single,
    Installer,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum InstallerKind {
    Inno,
    Msi,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize)]
pub struct ArchitectureTable {
    pub x64: Option<ArchitectureSpec>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct ArchitectureSpec {
    pub url: String,
    pub hash: String,
    #[serde(default)]
    pub extract_dir: Option<String>,
    #[serde(default)]
    pub bin: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Manifest {
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub homepage: Option<String>,
    #[serde(default)]
    pub license: Option<String>,
    #[serde(rename = "type")]
    pub package_type: PackageType,
    #[serde(default)]
    pub installer: Option<InstallerKind>,
    #[serde(default)]
    pub bin: Vec<String>,
    #[serde(default)]
    pub env: BTreeMap<String, String>,
    #[serde(default)]
    pub architecture: ArchitectureTable,
    #[serde(skip)]
    pub source_path: Option<PathBuf>,
}

impl Manifest {
    pub fn parse_str(input: &str) -> Result<Self> {
        let mut manifest: Manifest = toml::from_str(input).map_err(|source| PvError::ManifestParse {
            path: PathBuf::from("<inline>"),
            source,
        })?;
        manifest.normalize_bins();
        manifest.validate(Path::new("<inline>"))?;
        Ok(manifest)
    }

    pub fn from_path(path: &Path) -> Result<Self> {
        let content = fs::read_to_string(path).map_err(|source| PvError::Io {
            path: path.to_path_buf(),
            source,
        })?;
        let mut manifest: Manifest =
            toml::from_str(&content).map_err(|source| PvError::ManifestParse {
                path: path.to_path_buf(),
                source,
            })?;
        manifest.source_path = Some(path.to_path_buf());
        manifest.normalize_bins();
        manifest.validate(path)?;
        Ok(manifest)
    }

    pub fn x64(&self) -> Option<&ArchitectureSpec> {
        self.architecture.x64.as_ref()
    }

    fn normalize_bins(&mut self) {
        if let Some(x64) = self.architecture.x64.as_mut() {
            if x64.bin.is_empty() {
                x64.bin = self.bin.clone();
            }
        }
    }

    fn validate(&self, path: &Path) -> Result<()> {
        if self.name.trim().is_empty() {
            return self.validation_error(path, "name is required");
        }
        if self.version.trim().is_empty() {
            return self.validation_error(path, "version is required");
        }
        let Some(x64) = self.x64() else {
            return self.validation_error(path, "architecture.x64 is required for MVP");
        };
        if x64.url.trim().is_empty() {
            return self.validation_error(path, "architecture.x64.url is required");
        }
        if !x64.hash.starts_with("sha256:") || x64.hash.len() != "sha256:".len() + 64 {
            return self.validation_error(path, "architecture.x64.hash must be sha256:<64 lowercase hex chars>");
        }
        if x64.bin.is_empty() {
            return self.validation_error(path, "bin must contain at least one executable");
        }
        if self.package_type == PackageType::Installer && self.installer.is_none() {
            return self.validation_error(path, "installer kind is required for installer packages");
        }
        Ok(())
    }

    fn validation_error<T>(&self, path: &Path, message: &str) -> Result<T> {
        Err(PvError::ManifestValidation {
            path: path.to_path_buf(),
            message: message.to_string(),
        })
    }
}
```

- [ ] **Step 4: Run manifest tests**

Run: `cargo test --test manifest_tests`

Expected: PASS, manifest tests succeed.

---

### Task 5: Implement Bucket Scanning And Search

**Files:**
- Modify: `src/bucket.rs`
- Test: `tests/bucket_tests.rs`

- [ ] **Step 1: Write bucket tests with fixture manifests**

Create `tests/bucket_tests.rs`:

```rust
use std::fs;

use pv::bucket::{BucketManager, SearchResult};
use pv::config::{BucketConfig, Config, Paths};
use tempfile::tempdir;

fn write_manifest(path: &std::path::Path, name: &str, version: &str, description: &str) {
    fs::write(
        path,
        format!(
            r#"
name = "{name}"
version = "{version}"
description = "{description}"
type = "single"

[architecture.x64]
url = "https://example.invalid/{name}.exe"
hash = "sha256:dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd"
bin = ["{name}.exe"]
"#
        ),
    )
    .expect("write manifest");
}

#[test]
fn searches_all_registered_buckets() {
    let home = tempdir().expect("tempdir");
    let paths = Paths::from_home(home.path());
    let bucket_dir = paths.buckets.join("main");
    fs::create_dir_all(&bucket_dir).expect("bucket dir");
    write_manifest(&bucket_dir.join("ripgrep.toml"), "ripgrep", "14.1.0", "fast search");
    write_manifest(&bucket_dir.join("node.toml"), "node", "20.11.0", "javascript runtime");

    let config = Config {
        buckets: vec![BucketConfig {
            name: "main".to_string(),
            url: "https://example.invalid/main.git".to_string(),
        }],
        active_versions: Default::default(),
        path_registered: false,
    };
    let manager = BucketManager::new(paths, config);

    let results = manager.search("search").expect("search succeeds");

    assert_eq!(
        results,
        vec![SearchResult {
            bucket: "main".to_string(),
            name: "ripgrep".to_string(),
            version: "14.1.0".to_string(),
            description: Some("fast search".to_string()),
        }]
    );
}

#[test]
fn finds_requested_version_or_latest() {
    let home = tempdir().expect("tempdir");
    let paths = Paths::from_home(home.path());
    let bucket_dir = paths.buckets.join("main");
    fs::create_dir_all(bucket_dir.join("node")).expect("bucket dirs");
    write_manifest(&bucket_dir.join("node").join("20.toml"), "node", "20.11.0", "node 20");
    write_manifest(&bucket_dir.join("node").join("18.toml"), "node", "18.20.0", "node 18");

    let config = Config {
        buckets: vec![BucketConfig {
            name: "main".to_string(),
            url: "https://example.invalid/main.git".to_string(),
        }],
        active_versions: Default::default(),
        path_registered: false,
    };
    let manager = BucketManager::new(paths, config);

    let latest = manager.find_manifest("node", None).expect("latest manifest");
    let exact = manager.find_manifest("node", Some("18.20.0")).expect("exact manifest");

    assert_eq!(latest.version, "20.11.0");
    assert_eq!(exact.version, "18.20.0");
}
```

- [ ] **Step 2: Run bucket tests and verify failure**

Run: `cargo test --test bucket_tests`

Expected: FAIL because `BucketManager` is not implemented.

- [ ] **Step 3: Implement bucket scanning, search, and lookup**

Replace `src/bucket.rs`:

```rust
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use semver::Version;
use walkdir::WalkDir;

use crate::config::{Config, Paths};
use crate::error::{PvError, Result};
use crate::manifest::Manifest;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchResult {
    pub bucket: String,
    pub name: String,
    pub version: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone)]
pub struct BucketManager {
    paths: Paths,
    config: Config,
}

impl BucketManager {
    pub fn new(paths: Paths, config: Config) -> Self {
        Self { paths, config }
    }

    pub fn bucket_path(&self, name: &str) -> PathBuf {
        self.paths.buckets.join(name)
    }

    pub fn list(&self) -> &[crate::config::BucketConfig] {
        &self.config.buckets
    }

    pub fn add(&mut self, name: &str, url: &str) -> Result<()> {
        let target = self.bucket_path(name);
        let status = Command::new("git")
            .arg("clone")
            .arg(url)
            .arg(&target)
            .status()
            .map_err(|source| PvError::Io {
                path: target.clone(),
                source,
            })?;
        if !status.success() {
            return Err(PvError::CommandFailed {
                program: "git".to_string(),
                args: vec!["clone".to_string(), url.to_string(), target.display().to_string()],
                status: status.to_string(),
            });
        }
        self.config.add_bucket(name, url)
    }

    pub fn remove(&mut self, name: &str) -> Result<()> {
        let target = self.bucket_path(name);
        if target.exists() {
            fs::remove_dir_all(&target).map_err(|source| PvError::Io {
                path: target.clone(),
                source,
            })?;
        }
        self.config.remove_bucket(name)
    }

    pub fn sync(&self) -> Result<()> {
        for bucket in &self.config.buckets {
            let path = self.bucket_path(&bucket.name);
            let status = Command::new("git")
                .arg("-C")
                .arg(&path)
                .arg("pull")
                .arg("--ff-only")
                .status()
                .map_err(|source| PvError::Io {
                    path: path.clone(),
                    source,
                })?;
            if !status.success() {
                return Err(PvError::CommandFailed {
                    program: "git".to_string(),
                    args: vec![
                        "-C".to_string(),
                        path.display().to_string(),
                        "pull".to_string(),
                        "--ff-only".to_string(),
                    ],
                    status: status.to_string(),
                });
            }
        }
        Ok(())
    }

    pub fn search(&self, keyword: &str) -> Result<Vec<SearchResult>> {
        let needle = keyword.to_lowercase();
        let mut results = Vec::new();

        for (bucket, manifest) in self.manifests()? {
            let name_matches = manifest.name.to_lowercase().contains(&needle);
            let description_matches = manifest
                .description
                .as_deref()
                .unwrap_or_default()
                .to_lowercase()
                .contains(&needle);

            if name_matches || description_matches {
                results.push(SearchResult {
                    bucket,
                    name: manifest.name,
                    version: manifest.version,
                    description: manifest.description,
                });
            }
        }

        results.sort_by(|left, right| left.name.cmp(&right.name));
        Ok(results)
    }

    pub fn find_manifest(&self, name: &str, version: Option<&str>) -> Result<Manifest> {
        let mut matches = self
            .manifests()?
            .into_iter()
            .map(|(_, manifest)| manifest)
            .filter(|manifest| manifest.name == name)
            .filter(|manifest| version.map_or(true, |wanted| manifest.version == wanted))
            .collect::<Vec<_>>();

        if matches.is_empty() {
            return match version {
                Some(version) => Err(PvError::VersionNotFound {
                    package: name.to_string(),
                    version: version.to_string(),
                }),
                None => Err(PvError::PackageNotFound(name.to_string())),
            };
        }

        matches.sort_by(|left, right| compare_versions(&left.version, &right.version));
        Ok(matches.pop().expect("matches is not empty"))
    }

    pub fn manifests(&self) -> Result<Vec<(String, Manifest)>> {
        let mut manifests = Vec::new();

        for bucket in &self.config.buckets {
            let root = self.bucket_path(&bucket.name);
            if !root.exists() {
                continue;
            }

            for entry in WalkDir::new(&root).into_iter().filter_map(std::result::Result::ok) {
                let path = entry.path();
                if is_manifest_file(path) {
                    let manifest = Manifest::from_path(path)?;
                    manifests.push((bucket.name.clone(), manifest));
                }
            }
        }

        Ok(manifests)
    }
}

fn is_manifest_file(path: &Path) -> bool {
    path.is_file() && path.extension().and_then(|ext| ext.to_str()) == Some("toml")
}

fn compare_versions(left: &str, right: &str) -> std::cmp::Ordering {
    match (Version::parse(left), Version::parse(right)) {
        (Ok(left), Ok(right)) => left.cmp(&right),
        _ => left.cmp(right),
    }
}
```

- [ ] **Step 4: Run bucket tests**

Run: `cargo test --test bucket_tests`

Expected: PASS, bucket tests succeed.

---

### Task 6: Add Platform Abstraction And Windows Operations

**Files:**
- Modify: `src/platform/mod.rs`
- Create: `src/platform/windows.rs`
- Modify: `src/lib.rs`
- Test: `tests/version_tests.rs`

- [ ] **Step 1: Write fake-platform activation test**

Create `tests/version_tests.rs`:

```rust
use std::cell::RefCell;
use std::fs;
use std::path::{Path, PathBuf};

use pv::platform::Platform;
use pv::version::VersionManager;
use tempfile::tempdir;

#[derive(Default)]
struct FakePlatform {
    links: RefCell<Vec<(PathBuf, PathBuf)>>,
}

impl Platform for FakePlatform {
    fn make_active_link(&self, target: &Path, link: &Path) -> pv::error::Result<()> {
        self.links
            .borrow_mut()
            .push((target.to_path_buf(), link.to_path_buf()));
        Ok(())
    }

    fn remove_active_link(&self, _link: &Path) -> pv::error::Result<()> {
        Ok(())
    }

    fn create_shim(&self, _exe_name: &str, _shim_exe: &Path, _shim_config: &Path, _target: &Path) -> pv::error::Result<()> {
        Ok(())
    }

    fn remove_shim(&self, _shim_exe: &Path, _shim_config: &Path) -> pv::error::Result<()> {
        Ok(())
    }

    fn register_path(&self, _dir: &Path) -> pv::error::Result<()> {
        Ok(())
    }

    fn exe_ext(&self) -> &'static str {
        ".exe"
    }

    fn spawn_shell_with_path(&self, _path_prefix: &Path) -> pv::error::Result<()> {
        Ok(())
    }
}

#[test]
fn activation_points_current_at_requested_version() {
    let home = tempdir().expect("tempdir");
    let apps = home.path().join("apps");
    fs::create_dir_all(apps.join("node").join("20.11.0")).expect("version dir");

    let platform = FakePlatform::default();
    let manager = VersionManager::new(apps.clone(), &platform);

    manager.activate("node", "20.11.0").expect("activate");

    assert_eq!(
        platform.links.borrow().as_slice(),
        &[(
            apps.join("node").join("20.11.0"),
            apps.join("node").join("current")
        )]
    );
}
```

- [ ] **Step 2: Run version tests and verify failure**

Run: `cargo test --test version_tests`

Expected: FAIL because `Platform` and `VersionManager` are not implemented.

- [ ] **Step 3: Implement the platform trait**

Replace `src/platform/mod.rs`:

```rust
use std::path::Path;

use crate::error::Result;

#[cfg(windows)]
pub mod windows;

pub trait Platform {
    fn make_active_link(&self, target: &Path, link: &Path) -> Result<()>;
    fn remove_active_link(&self, link: &Path) -> Result<()>;
    fn create_shim(
        &self,
        exe_name: &str,
        shim_exe: &Path,
        shim_config: &Path,
        target: &Path,
    ) -> Result<()>;
    fn remove_shim(&self, shim_exe: &Path, shim_config: &Path) -> Result<()>;
    fn register_path(&self, dir: &Path) -> Result<()>;
    fn exe_ext(&self) -> &'static str;
    fn spawn_shell_with_path(&self, path_prefix: &Path) -> Result<()>;
}
```

- [ ] **Step 4: Implement Windows platform operations**

Create `src/platform/windows.rs`:

```rust
use std::fs;
use std::path::Path;
use std::process::Command;

use crate::error::{PvError, Result};
use crate::platform::Platform;

#[derive(Debug, Default, Clone, Copy)]
pub struct WindowsPlatform;

impl Platform for WindowsPlatform {
    fn make_active_link(&self, target: &Path, link: &Path) -> Result<()> {
        self.remove_active_link(link)?;
        junction::create(target, link).map_err(|error| PvError::Platform(format!("create junction failed: {error}")))
    }

    fn remove_active_link(&self, link: &Path) -> Result<()> {
        if !link.exists() {
            return Ok(());
        }
        fs::remove_dir(link).map_err(|source| PvError::Io {
            path: link.to_path_buf(),
            source,
        })
    }

    fn create_shim(&self, _exe_name: &str, shim_exe: &Path, shim_config: &Path, target: &Path) -> Result<()> {
        let source = current_shim_source()?;
        fs::copy(&source, shim_exe).map_err(|source_error| PvError::Io {
            path: shim_exe.to_path_buf(),
            source: source_error,
        })?;
        fs::write(shim_config, format!("target={}\n", target.display())).map_err(|source_error| PvError::Io {
            path: shim_config.to_path_buf(),
            source: source_error,
        })
    }

    fn remove_shim(&self, shim_exe: &Path, shim_config: &Path) -> Result<()> {
        for path in [shim_exe, shim_config] {
            if path.exists() {
                fs::remove_file(path).map_err(|source| PvError::Io {
                    path: path.to_path_buf(),
                    source,
                })?;
            }
        }
        Ok(())
    }

    fn register_path(&self, dir: &Path) -> Result<()> {
        let dir_text = dir.display().to_string();
        let script = format!(
            "$path=[Environment]::GetEnvironmentVariable('Path','User'); \
             if (-not ($path -split ';' | Where-Object {{ $_ -eq '{dir_text}' }})) {{ \
             [Environment]::SetEnvironmentVariable('Path', ($path.TrimEnd(';') + ';{dir_text}'), 'User') }}"
        );
        let status = Command::new("powershell")
            .args(["-NoProfile", "-Command", &script])
            .status()
            .map_err(|source| PvError::Io {
                path: dir.to_path_buf(),
                source,
            })?;

        if status.success() {
            Ok(())
        } else {
            Err(PvError::CommandFailed {
                program: "powershell".to_string(),
                args: vec!["-NoProfile".to_string(), "-Command".to_string(), script],
                status: status.to_string(),
            })
        }
    }

    fn exe_ext(&self) -> &'static str {
        ".exe"
    }

    fn spawn_shell_with_path(&self, path_prefix: &Path) -> Result<()> {
        let shell = std::env::var("ComSpec").unwrap_or_else(|_| "cmd.exe".to_string());
        let old_path = std::env::var("PATH").unwrap_or_default();
        let new_path = format!("{};{}", path_prefix.display(), old_path);
        let status = Command::new(shell.clone())
            .env("PATH", new_path)
            .status()
            .map_err(|source| PvError::Io {
                path: path_prefix.to_path_buf(),
                source,
            })?;

        if status.success() {
            Ok(())
        } else {
            Err(PvError::CommandFailed {
                program: shell,
                args: Vec::new(),
                status: status.to_string(),
            })
        }
    }
}

fn current_shim_source() -> Result<std::path::PathBuf> {
    let mut path = std::env::current_exe().map_err(|source| PvError::Io {
        path: std::path::PathBuf::from("current_exe"),
        source,
    })?;
    path.set_file_name(format!("pv-shim{}", std::env::consts::EXE_SUFFIX));
    Ok(path)
}
```

- [ ] **Step 5: Implement version activation**

Replace `src/version.rs`:

```rust
use std::fs;
use std::path::{Path, PathBuf};

use crate::error::{PvError, Result};
use crate::platform::Platform;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstalledVersion {
    pub version: String,
    pub active: bool,
}

pub struct VersionManager<'a, P: Platform> {
    apps_dir: PathBuf,
    platform: &'a P,
}

impl<'a, P: Platform> VersionManager<'a, P> {
    pub fn new(apps_dir: PathBuf, platform: &'a P) -> Self {
        Self { apps_dir, platform }
    }

    pub fn package_dir(&self, package: &str) -> PathBuf {
        self.apps_dir.join(package)
    }

    pub fn version_dir(&self, package: &str, version: &str) -> PathBuf {
        self.package_dir(package).join(version)
    }

    pub fn current_link(&self, package: &str) -> PathBuf {
        self.package_dir(package).join("current")
    }

    pub fn activate(&self, package: &str, version: &str) -> Result<()> {
        let target = self.version_dir(package, version);
        if !target.exists() {
            return Err(PvError::VersionNotFound {
                package: package.to_string(),
                version: version.to_string(),
            });
        }
        self.platform.make_active_link(&target, &self.current_link(package))
    }

    pub fn list_versions(&self, package: &str, active_version: Option<&str>) -> Result<Vec<InstalledVersion>> {
        let package_dir = self.package_dir(package);
        if !package_dir.exists() {
            return Ok(Vec::new());
        }

        let mut versions = Vec::new();
        for entry in fs::read_dir(&package_dir).map_err(|source| PvError::Io {
            path: package_dir.clone(),
            source,
        })? {
            let entry = entry.map_err(|source| PvError::Io {
                path: package_dir.clone(),
                source,
            })?;
            let file_name = entry.file_name().to_string_lossy().to_string();
            if file_name == "current" || file_name == ".manifest.toml" {
                continue;
            }
            if entry.path().is_dir() {
                versions.push(InstalledVersion {
                    active: active_version == Some(file_name.as_str()),
                    version: file_name,
                });
            }
        }

        versions.sort_by(|left, right| left.version.cmp(&right.version));
        Ok(versions)
    }

    pub fn remove_version_dir(path: &Path) -> Result<()> {
        if path.exists() {
            fs::remove_dir_all(path).map_err(|source| PvError::Io {
                path: path.to_path_buf(),
                source,
            })?;
        }
        Ok(())
    }
}
```

- [ ] **Step 6: Run version tests**

Run: `cargo test --test version_tests`

Expected: PASS, activation test succeeds.

---

### Task 7: Generate Shim Configs And Proxy Executables

**Files:**
- Modify: `src/shim.rs`
- Modify: `src/bin/shim.rs`
- Test: `tests/shim_tests.rs`

- [ ] **Step 1: Write shim tests**

Create `tests/shim_tests.rs`:

```rust
use std::fs;

use pv::shim::{read_shim_target, shim_config_path, shim_exe_path, target_for_bin};
use tempfile::tempdir;

#[test]
fn calculates_shim_paths() {
    let dir = tempdir().expect("tempdir");

    assert_eq!(shim_exe_path(dir.path(), "rg.exe"), dir.path().join("rg.exe"));
    assert_eq!(shim_config_path(dir.path(), "rg.exe"), dir.path().join("rg.shim"));
}

#[test]
fn target_uses_current_junction() {
    let apps = tempdir().expect("tempdir");

    let target = target_for_bin(apps.path(), "ripgrep", "rg.exe");

    assert_eq!(target, apps.path().join("ripgrep").join("current").join("rg.exe"));
}

#[test]
fn reads_target_from_config_file() {
    let dir = tempdir().expect("tempdir");
    let config = dir.path().join("rg.shim");
    fs::write(&config, format!("target={}\n", dir.path().join("rg-real.exe").display()))
        .expect("write config");

    let target = read_shim_target(&config).expect("read target");

    assert_eq!(target, dir.path().join("rg-real.exe"));
}
```

- [ ] **Step 2: Run shim tests and verify failure**

Run: `cargo test --test shim_tests`

Expected: FAIL because shim helpers are not implemented.

- [ ] **Step 3: Implement shim helper functions**

Replace `src/shim.rs`:

```rust
use std::fs;
use std::path::{Path, PathBuf};

use crate::error::{PvError, Result};

pub fn shim_exe_path(shims_dir: &Path, bin_name: &str) -> PathBuf {
    shims_dir.join(bin_name)
}

pub fn shim_config_path(shims_dir: &Path, bin_name: &str) -> PathBuf {
    let stem = Path::new(bin_name)
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or(bin_name);
    shims_dir.join(format!("{stem}.shim"))
}

pub fn target_for_bin(apps_dir: &Path, package: &str, bin_name: &str) -> PathBuf {
    apps_dir.join(package).join("current").join(bin_name)
}

pub fn write_shim_config(config_path: &Path, target: &Path) -> Result<()> {
    fs::write(config_path, format!("target={}\n", target.display())).map_err(|source| PvError::Io {
        path: config_path.to_path_buf(),
        source,
    })
}

pub fn read_shim_target(config_path: &Path) -> Result<PathBuf> {
    let content = fs::read_to_string(config_path).map_err(|source| PvError::Io {
        path: config_path.to_path_buf(),
        source,
    })?;

    content
        .lines()
        .find_map(|line| line.strip_prefix("target=").map(PathBuf::from))
        .ok_or_else(|| PvError::Platform(format!("shim config missing target: {}", config_path.display())))
}
```

- [ ] **Step 4: Implement the `pv-shim` proxy binary**

Replace `src/bin/shim.rs`:

```rust
use std::process::Command;

fn main() {
    if let Err(error) = run() {
        eprintln!("pv-shim: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let exe = std::env::current_exe().map_err(|error| format!("current_exe failed: {error}"))?;
    let config = exe.with_extension("shim");
    let target = pv::shim::read_shim_target(&config).map_err(|error| error.to_string())?;
    let args = std::env::args_os().skip(1).collect::<Vec<_>>();

    let status = Command::new(&target)
        .args(args)
        .status()
        .map_err(|error| format!("failed to run {}: {error}", target.display()))?;

    std::process::exit(status.code().unwrap_or(1));
}
```

- [ ] **Step 5: Run shim tests**

Run: `cargo test --test shim_tests`

Expected: PASS, shim helper tests succeed.

---

### Task 8: Download With SHA256 Verification

**Files:**
- Modify: `src/engine/download.rs`
- Modify: `src/engine/mod.rs`
- Test: `tests/download_tests.rs`

- [ ] **Step 1: Write checksum and local HTTP tests**

Create `tests/download_tests.rs`:

```rust
use std::fs;
use std::thread;

use pv::engine::download::{download_to_cache, sha256_file, verify_sha256};
use tempfile::tempdir;
use tiny_http::{Response, Server};

#[test]
fn verifies_sha256_hash() {
    let dir = tempdir().expect("tempdir");
    let file = dir.path().join("tool.exe");
    fs::write(&file, b"pv-test").expect("write file");

    let hash = sha256_file(&file).expect("hash file");

    assert_eq!(hash, "5b9f35149d9f2e30a06e67c9c00578054687a6043935f031424588d49e66ae76");
    verify_sha256(&file, "sha256:5b9f35149d9f2e30a06e67c9c00578054687a6043935f031424588d49e66ae76")
        .expect("hash matches");
    assert!(verify_sha256(&file, "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa").is_err());
}

#[tokio::test]
async fn downloads_to_cache_and_validates_hash() {
    let server = Server::http("127.0.0.1:0").expect("server");
    let address = format!("http://{}", server.server_addr());
    let handle = thread::spawn(move || {
        let request = server.recv().expect("request");
        request
            .respond(Response::from_data(Vec::from(&b"pv-test"[..])))
            .expect("respond");
    });

    let dir = tempdir().expect("tempdir");
    let file = download_to_cache(
        &format!("{address}/tool.exe"),
        "sha256:5b9f35149d9f2e30a06e67c9c00578054687a6043935f031424588d49e66ae76",
        dir.path(),
        "tool.exe",
    )
    .await
    .expect("download");

    handle.join().expect("server thread");
    assert_eq!(fs::read(file).expect("read file"), b"pv-test");
}
```

- [ ] **Step 2: Run download tests and verify failure**

Run: `cargo test --test download_tests`

Expected: FAIL because `engine::download` is not implemented.

- [ ] **Step 3: Wire the download module**

Replace `src/engine/mod.rs` with:

```rust
pub mod download;
pub mod install;
```

Create `src/engine/download.rs`:

```rust
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

use futures_util::StreamExt;
use sha2::{Digest, Sha256};
use tokio::io::AsyncWriteExt;

use crate::error::{PvError, Result};

pub async fn download_to_cache(url: &str, expected_hash: &str, cache_dir: &Path, file_name: &str) -> Result<PathBuf> {
    tokio::fs::create_dir_all(cache_dir)
        .await
        .map_err(|source| PvError::Io {
            path: cache_dir.to_path_buf(),
            source,
        })?;

    let target = cache_dir.join(file_name);
    let response = reqwest::get(url)
        .await
        .map_err(|error| PvError::Download(error.to_string()))?
        .error_for_status()
        .map_err(|error| PvError::Download(error.to_string()))?;

    let mut file = tokio::fs::File::create(&target)
        .await
        .map_err(|source| PvError::Io {
            path: target.clone(),
            source,
        })?;
    let mut stream = response.bytes_stream();

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|error| PvError::Download(error.to_string()))?;
        file.write_all(&chunk).await.map_err(|source| PvError::Io {
            path: target.clone(),
            source,
        })?;
    }
    file.flush().await.map_err(|source| PvError::Io {
        path: target.clone(),
        source,
    })?;

    if let Err(error) = verify_sha256(&target, expected_hash) {
        let _ = tokio::fs::remove_file(&target).await;
        return Err(error);
    }

    Ok(target)
}

pub fn verify_sha256(path: &Path, expected_hash: &str) -> Result<()> {
    let expected = expected_hash
        .strip_prefix("sha256:")
        .ok_or_else(|| PvError::Download(format!("unsupported hash format: {expected_hash}")))?;
    let actual = sha256_file(path)?;

    if actual == expected {
        Ok(())
    } else {
        Err(PvError::ChecksumMismatch {
            path: path.to_path_buf(),
            expected: expected.to_string(),
            actual,
        })
    }
}

pub fn sha256_file(path: &Path) -> Result<String> {
    let mut file = fs::File::open(path).map_err(|source| PvError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 8192];

    loop {
        let read = file.read(&mut buffer).map_err(|source| PvError::Io {
            path: path.to_path_buf(),
            source,
        })?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }

    Ok(format!("{:x}", hasher.finalize()))
}
```

- [ ] **Step 4: Run download tests**

Run: `cargo test --test download_tests`

Expected: PASS, checksum and local download tests succeed.

---

### Task 9: Install Archive And Single-File Packages

**Files:**
- Create: `src/engine/install/mod.rs`
- Create: `src/engine/install/archive.rs`
- Create: `src/engine/install/single.rs`
- Test: `tests/install_strategy_tests.rs`

- [ ] **Step 1: Write install strategy tests**

Create `tests/install_strategy_tests.rs`:

```rust
use std::fs;

use pv::engine::install::{install_package, InstallContext};
use pv::manifest::Manifest;
use tempfile::tempdir;
use zip::write::SimpleFileOptions;

#[test]
fn installs_single_file_into_version_dir() {
    let temp = tempdir().expect("tempdir");
    let source = temp.path().join("rg.exe");
    fs::write(&source, b"exe").expect("write exe");
    let version_dir = temp.path().join("apps").join("ripgrep").join("14.1.0");
    let manifest = Manifest::parse_str(
        r#"
name = "ripgrep"
version = "14.1.0"
type = "single"

[architecture.x64]
url = "https://example.invalid/rg.exe"
hash = "sha256:eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee"
bin = ["rg.exe"]
"#,
    )
    .expect("manifest");

    install_package(
        &manifest,
        &InstallContext::new(source, version_dir.clone(), temp.path().join("tools")),
    )
    .expect("install");

    assert_eq!(fs::read(version_dir.join("rg.exe")).expect("read exe"), b"exe");
}

#[test]
fn installs_zip_archive_with_extract_dir_promoted() {
    let temp = tempdir().expect("tempdir");
    let archive = temp.path().join("node.zip");
    let file = fs::File::create(&archive).expect("create zip");
    let mut zip = zip::ZipWriter::new(file);
    zip.start_file("node-v20/node.exe", SimpleFileOptions::default()).expect("zip file");
    std::io::Write::write_all(&mut zip, b"node").expect("zip bytes");
    zip.finish().expect("finish zip");

    let version_dir = temp.path().join("apps").join("node").join("20.11.0");
    let manifest = Manifest::parse_str(
        r#"
name = "node"
version = "20.11.0"
type = "archive"
bin = ["node.exe"]

[architecture.x64]
url = "https://example.invalid/node.zip"
hash = "sha256:ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff"
extract_dir = "node-v20"
"#,
    )
    .expect("manifest");

    install_package(
        &manifest,
        &InstallContext::new(archive, version_dir.clone(), temp.path().join("tools")),
    )
    .expect("install");

    assert_eq!(fs::read(version_dir.join("node.exe")).expect("read node"), b"node");
}
```

- [ ] **Step 2: Run install strategy tests and verify failure**

Run: `cargo test --test install_strategy_tests`

Expected: FAIL because `engine::install` does not exist.

- [ ] **Step 3: Implement install dispatcher**

Create `src/engine/install/mod.rs`:

```rust
use std::path::PathBuf;

use crate::error::Result;
use crate::manifest::{Manifest, PackageType};

pub mod archive;
pub mod installer;
pub mod single;

#[derive(Debug, Clone)]
pub struct InstallContext {
    pub source_file: PathBuf,
    pub version_dir: PathBuf,
    pub tools_dir: PathBuf,
}

impl InstallContext {
    pub fn new(source_file: PathBuf, version_dir: PathBuf, tools_dir: PathBuf) -> Self {
        Self {
            source_file,
            version_dir,
            tools_dir,
        }
    }
}

pub trait InstallStrategy {
    fn install(&self, manifest: &Manifest, context: &InstallContext) -> Result<()>;
}

pub fn install_package(manifest: &Manifest, context: &InstallContext) -> Result<()> {
    match manifest.package_type {
        PackageType::Archive => archive::ArchiveInstallStrategy.install(manifest, context),
        PackageType::Single => single::SingleInstallStrategy.install(manifest, context),
        PackageType::Installer => installer::InstallerInstallStrategy.install(manifest, context),
    }
}
```

- [ ] **Step 4: Implement single-file installation**

Create `src/engine/install/single.rs`:

```rust
use std::fs;

use crate::engine::install::{InstallContext, InstallStrategy};
use crate::error::{PvError, Result};
use crate::manifest::Manifest;

pub struct SingleInstallStrategy;

impl InstallStrategy for SingleInstallStrategy {
    fn install(&self, manifest: &Manifest, context: &InstallContext) -> Result<()> {
        fs::create_dir_all(&context.version_dir).map_err(|source| PvError::Io {
            path: context.version_dir.clone(),
            source,
        })?;
        let file_name = manifest
            .x64()
            .and_then(|arch| arch.bin.first())
            .ok_or_else(|| PvError::Extract {
                path: context.source_file.clone(),
                message: "single package requires one bin entry".to_string(),
            })?;
        fs::copy(&context.source_file, context.version_dir.join(file_name)).map_err(|source| PvError::Io {
            path: context.version_dir.join(file_name),
            source,
        })?;
        Ok(())
    }
}
```

- [ ] **Step 5: Implement archive installation**

Create `src/engine/install/archive.rs`:

```rust
use std::fs;
use std::path::Path;

use crate::engine::install::{InstallContext, InstallStrategy};
use crate::error::{PvError, Result};
use crate::manifest::Manifest;

pub struct ArchiveInstallStrategy;

impl InstallStrategy for ArchiveInstallStrategy {
    fn install(&self, manifest: &Manifest, context: &InstallContext) -> Result<()> {
        let temp_dir = context.version_dir.with_extension("tmp");
        if temp_dir.exists() {
            fs::remove_dir_all(&temp_dir).map_err(|source| PvError::Io {
                path: temp_dir.clone(),
                source,
            })?;
        }
        fs::create_dir_all(&temp_dir).map_err(|source| PvError::Io {
            path: temp_dir.clone(),
            source,
        })?;

        extract_archive(&context.source_file, &temp_dir)?;

        if context.version_dir.exists() {
            fs::remove_dir_all(&context.version_dir).map_err(|source| PvError::Io {
                path: context.version_dir.clone(),
                source,
            })?;
        }

        let source_root = manifest
            .x64()
            .and_then(|arch| arch.extract_dir.as_deref())
            .map(|dir| temp_dir.join(dir))
            .unwrap_or_else(|| temp_dir.clone());

        fs::rename(&source_root, &context.version_dir).or_else(|_| {
            fs::create_dir_all(&context.version_dir)?;
            copy_dir_contents(&source_root, &context.version_dir)
        }).map_err(|source| PvError::Io {
            path: context.version_dir.clone(),
            source,
        })?;

        if temp_dir.exists() {
            fs::remove_dir_all(&temp_dir).map_err(|source| PvError::Io {
                path: temp_dir,
                source,
            })?;
        }
        Ok(())
    }
}

fn extract_archive(source: &Path, target: &Path) -> Result<()> {
    let extension = source.extension().and_then(|ext| ext.to_str()).unwrap_or_default();
    match extension {
        "zip" => extract_zip(source, target),
        "gz" | "tgz" => extract_tar_gz(source, target),
        other => Err(PvError::Extract {
            path: source.to_path_buf(),
            message: format!("unsupported archive extension: {other}"),
        }),
    }
}

fn extract_zip(source: &Path, target: &Path) -> Result<()> {
    let file = fs::File::open(source).map_err(|error| PvError::Io {
        path: source.to_path_buf(),
        source: error,
    })?;
    let mut archive = zip::ZipArchive::new(file).map_err(|error| PvError::Extract {
        path: source.to_path_buf(),
        message: error.to_string(),
    })?;
    archive.extract(target).map_err(|error| PvError::Extract {
        path: source.to_path_buf(),
        message: error.to_string(),
    })
}

fn extract_tar_gz(source: &Path, target: &Path) -> Result<()> {
    let file = fs::File::open(source).map_err(|error| PvError::Io {
        path: source.to_path_buf(),
        source: error,
    })?;
    let decoder = flate2::read::GzDecoder::new(file);
    let mut archive = tar::Archive::new(decoder);
    archive.unpack(target).map_err(|error| PvError::Extract {
        path: source.to_path_buf(),
        message: error.to_string(),
    })
}

fn copy_dir_contents(source: &Path, target: &Path) -> std::io::Result<()> {
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let source_path = entry.path();
        let target_path = target.join(entry.file_name());
        if source_path.is_dir() {
            fs::create_dir_all(&target_path)?;
            copy_dir_contents(&source_path, &target_path)?;
        } else {
            fs::copy(&source_path, &target_path)?;
        }
    }
    Ok(())
}
```

- [ ] **Step 6: Add installer module shell**

Create `src/engine/install/installer.rs`:

```rust
use crate::engine::install::{InstallContext, InstallStrategy};
use crate::error::{PvError, Result};
use crate::manifest::Manifest;

pub struct InstallerInstallStrategy;

impl InstallStrategy for InstallerInstallStrategy {
    fn install(&self, _manifest: &Manifest, context: &InstallContext) -> Result<()> {
        Err(PvError::Extract {
            path: context.source_file.clone(),
            message: "installer extraction is added in Task 10".to_string(),
        })
    }
}
```

- [ ] **Step 7: Run install strategy tests**

Run: `cargo test --test install_strategy_tests`

Expected: PASS, archive and single strategy tests succeed.

---

### Task 10: Add Extraction-Only Installer Strategy

**Files:**
- Modify: `src/engine/install/installer.rs`
- Test: `tests/installer_strategy_tests.rs`

- [ ] **Step 1: Write installer command and `innounp` bootstrap tests**

Create `tests/installer_strategy_tests.rs`:

```rust
use std::fs;

use pv::engine::install::installer::{
    build_inno_command, build_msi_command, ensure_innounp_available, innounp_path,
    INNOUNP_ARCHIVE_SHA256, INNOUNP_URL,
};
use tempfile::tempdir;

#[test]
fn pins_upstream_innounp_archive() {
    assert_eq!(
        INNOUNP_URL,
        "https://sourceforge.net/projects/innounp/files/innounp/innounp%200.50/innounp050.rar/download"
    );
    assert_eq!(
        INNOUNP_ARCHIVE_SHA256,
        "1d8837540ccc15d98245a1c73fd08f404b2a7bdfe7dc9bed2fdece818ff6df67"
    );
}

#[test]
fn calculates_innounp_path_under_tools_dir() {
    let tools = tempdir().expect("tempdir");

    assert_eq!(
        innounp_path(tools.path()),
        tools.path().join("innounp").join("innounp.exe")
    );
}

#[test]
fn reuses_existing_innounp_without_network_download() {
    let tools = tempdir().expect("tempdir");
    let existing = innounp_path(tools.path());
    fs::create_dir_all(existing.parent().expect("parent")).expect("tool dir");
    fs::write(&existing, b"existing innounp exe").expect("write tool");

    let resolved = ensure_innounp_available(tools.path()).expect("resolve innounp");

    assert_eq!(resolved, existing);
}

#[test]
fn builds_msi_administrative_extract_command() {
    let command = build_msi_command("tool.msi", "C:/pv/apps/tool/1.0.0");

    assert_eq!(command.program, "msiexec");
    assert_eq!(
        command.args,
        vec![
            "/a",
            "tool.msi",
            "/qn",
            "TARGETDIR=C:/pv/apps/tool/1.0.0"
        ]
    );
}

#[test]
fn builds_inno_extract_command() {
    let command = build_inno_command("C:/pv/tools/innounp/innounp.exe", "tool.exe", "C:/pv/apps/tool/1.0.0");

    assert_eq!(command.program, "C:/pv/tools/innounp/innounp.exe");
    assert_eq!(command.args, vec!["-x", "-y", "-dC:/pv/apps/tool/1.0.0", "tool.exe"]);
}
```

- [ ] **Step 2: Run installer tests and verify failure**

Run: `cargo test --test installer_strategy_tests`

Expected: FAIL because installer command builders and `innounp` bootstrap helpers are not implemented.

- [ ] **Step 3: Implement extraction command builders and `innounp` bootstrap**

Replace `src/engine/install/installer.rs`:

```rust
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

use unrar_ng::Archive;

use crate::engine::download::{sha256_file, verify_sha256};
use crate::engine::install::{InstallContext, InstallStrategy};
use crate::error::{PvError, Result};
use crate::manifest::{InstallerKind, Manifest};

pub const INNOUNP_URL: &str =
    "https://sourceforge.net/projects/innounp/files/innounp/innounp%200.50/innounp050.rar/download";
pub const INNOUNP_ARCHIVE_SHA256: &str =
    "1d8837540ccc15d98245a1c73fd08f404b2a7bdfe7dc9bed2fdece818ff6df67";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtractCommand {
    pub program: String,
    pub args: Vec<String>,
}

pub struct InstallerInstallStrategy;

impl InstallStrategy for InstallerInstallStrategy {
    fn install(&self, manifest: &Manifest, context: &InstallContext) -> Result<()> {
        fs::create_dir_all(&context.version_dir).map_err(|source| PvError::Io {
            path: context.version_dir.clone(),
            source,
        })?;

        let source = context.source_file.display().to_string();
        let target = context.version_dir.display().to_string();
        let command = match manifest.installer.as_ref() {
            Some(InstallerKind::Msi) => build_msi_command(&source, &target),
            Some(InstallerKind::Inno) => {
                let innounp = ensure_innounp_available(&context.tools_dir)?;
                build_inno_command(&innounp.display().to_string(), &source, &target)
            }
            None => {
                return Err(PvError::Extract {
                    path: context.source_file.clone(),
                    message: "installer kind is required".to_string(),
                });
            }
        };

        run_extract_command(command)
    }
}

pub fn innounp_path(tools_dir: &Path) -> PathBuf {
    tools_dir.join("innounp").join("innounp.exe")
}

pub fn ensure_innounp_available(tools_dir: &Path) -> Result<PathBuf> {
    let target = innounp_path(tools_dir);
    if target.exists() {
        return Ok(target);
    }

    let tool_dir = target.parent().expect("innounp path has parent");
    fs::create_dir_all(tool_dir).map_err(|source| PvError::Io {
        path: tool_dir.to_path_buf(),
        source,
    })?;

    let archive_path = tool_dir.join("innounp050.rar");
    download_innounp_archive(&archive_path)?;
    verify_sha256(&archive_path, &format!("sha256:{INNOUNP_ARCHIVE_SHA256}"))?;
    extract_innounp_archive(&archive_path, tool_dir)?;

    if !target.exists() {
        return Err(PvError::Extract {
            path: archive_path,
            message: "innounp.exe was not present after extracting innounp050.rar".to_string(),
        });
    }

    Ok(target)
}

pub fn build_msi_command(source: &str, target_dir: &str) -> ExtractCommand {
    ExtractCommand {
        program: "msiexec".to_string(),
        args: vec![
            "/a".to_string(),
            source.to_string(),
            "/qn".to_string(),
            format!("TARGETDIR={target_dir}"),
        ],
    }
}

pub fn build_inno_command(innounp: &str, source: &str, target_dir: &str) -> ExtractCommand {
    ExtractCommand {
        program: innounp.to_string(),
        args: vec![
            "-x".to_string(),
            "-y".to_string(),
            format!("-d{target_dir}"),
            source.to_string(),
        ],
    }
}

fn download_innounp_archive(target: &Path) -> Result<()> {
    let response = reqwest::blocking::get(INNOUNP_URL)
        .map_err(|error| PvError::Download(error.to_string()))?
        .error_for_status()
        .map_err(|error| PvError::Download(error.to_string()))?;
    let bytes = response
        .bytes()
        .map_err(|error| PvError::Download(error.to_string()))?;
    let mut file = fs::File::create(target).map_err(|source| PvError::Io {
        path: target.to_path_buf(),
        source,
    })?;
    file.write_all(&bytes).map_err(|source| PvError::Io {
        path: target.to_path_buf(),
        source,
    })?;

    let actual = sha256_file(target)?;
    if actual == INNOUNP_ARCHIVE_SHA256 {
        Ok(())
    } else {
        Err(PvError::ChecksumMismatch {
            path: target.to_path_buf(),
            expected: INNOUNP_ARCHIVE_SHA256.to_string(),
            actual,
        })
    }
}

fn extract_innounp_archive(archive_path: &Path, tool_dir: &Path) -> Result<()> {
    let archive = Archive::new(archive_path)
        .open_for_processing()
        .map_err(|error| PvError::Extract {
            path: archive_path.to_path_buf(),
            message: error.to_string(),
        })?;
    archive.extract_all(tool_dir).map_err(|error| PvError::Extract {
        path: archive_path.to_path_buf(),
        message: error.to_string(),
    })
}

fn run_extract_command(command: ExtractCommand) -> Result<()> {
    let status = Command::new(&command.program)
        .args(&command.args)
        .status()
        .map_err(|source| PvError::Io {
            path: command.program.clone().into(),
            source,
        })?;

    if status.success() {
        Ok(())
    } else {
        Err(PvError::CommandFailed {
            program: command.program,
            args: command.args,
            status: status.to_string(),
        })
    }
}
```

- [ ] **Step 4: Run installer tests**

Run: `cargo test --test installer_strategy_tests`

Expected: PASS, command-builder tests and local `innounp` reuse test succeed.

- [ ] **Step 5: Verify pinned `innounp` source assumptions**

Run: `cargo test --test installer_strategy_tests pins_upstream_innounp_archive`

Expected: PASS, the implementation still pins SourceForge `innounp050.rar` version 0.50 and SHA-256 `1d8837540ccc15d98245a1c73fd08f404b2a7bdfe7dc9bed2fdece818ff6df67`.

---

### Task 11: Orchestrate Install, Use, List, And Uninstall

**Files:**
- Modify: `src/engine/mod.rs`
- Modify: `src/shim.rs`
- Modify: `src/config.rs`
- Test: `tests/engine_lifecycle_tests.rs`

- [ ] **Step 1: Write engine lifecycle tests using local files**

Create `tests/engine_lifecycle_tests.rs`:

```rust
use std::cell::RefCell;
use std::fs;
use std::path::{Path, PathBuf};

use pv::config::{BucketConfig, Config, Paths};
use pv::engine::Engine;
use pv::platform::Platform;
use tempfile::tempdir;

#[derive(Default)]
struct FakePlatform {
    links: RefCell<Vec<(PathBuf, PathBuf)>>,
    shims: RefCell<Vec<(String, PathBuf)>>,
}

impl Platform for FakePlatform {
    fn make_active_link(&self, target: &Path, link: &Path) -> pv::error::Result<()> {
        self.links.borrow_mut().push((target.to_path_buf(), link.to_path_buf()));
        Ok(())
    }

    fn remove_active_link(&self, _link: &Path) -> pv::error::Result<()> {
        Ok(())
    }

    fn create_shim(&self, exe_name: &str, _shim_exe: &Path, _shim_config: &Path, target: &Path) -> pv::error::Result<()> {
        self.shims.borrow_mut().push((exe_name.to_string(), target.to_path_buf()));
        Ok(())
    }

    fn remove_shim(&self, _shim_exe: &Path, _shim_config: &Path) -> pv::error::Result<()> {
        Ok(())
    }

    fn register_path(&self, _dir: &Path) -> pv::error::Result<()> {
        Ok(())
    }

    fn exe_ext(&self) -> &'static str {
        ".exe"
    }

    fn spawn_shell_with_path(&self, _path_prefix: &Path) -> pv::error::Result<()> {
        Ok(())
    }
}

#[tokio::test]
async fn install_use_list_and_uninstall_single_package() {
    let home = tempdir().expect("tempdir");
    let paths = Paths::from_home(home.path());
    let bucket_dir = paths.buckets.join("main");
    fs::create_dir_all(&bucket_dir).expect("bucket dir");
    let source = home.path().join("rg-source.exe");
    fs::write(&source, b"rg").expect("write source");
    fs::write(
        bucket_dir.join("ripgrep.toml"),
        format!(
            r#"
name = "ripgrep"
version = "14.1.0"
description = "fast search"
type = "single"

[architecture.x64]
url = "file://{}"
hash = "sha256:ac0d2a97f32c42102c6428b3207a5de0a336c55963d1ec95ed729b8d6f4ae4d8"
bin = ["rg.exe"]
"#,
            source.display()
        ),
    )
    .expect("write manifest");

    let config = Config {
        buckets: vec![BucketConfig {
            name: "main".to_string(),
            url: "local".to_string(),
        }],
        active_versions: Default::default(),
        path_registered: false,
    };
    let platform = FakePlatform::default();
    let mut engine = Engine::new(paths.clone(), config, &platform);

    engine.install("ripgrep").await.expect("install");
    let versions = engine.list(Some("ripgrep")).expect("list");

    assert!(paths.apps.join("ripgrep").join("14.1.0").join("rg.exe").exists());
    assert_eq!(versions[0].package, "ripgrep");
    assert_eq!(versions[0].versions[0].version, "14.1.0");
    assert_eq!(platform.shims.borrow()[0].0, "rg.exe");

    engine.use_version("ripgrep@14.1.0").expect("use");
    engine.uninstall("ripgrep@14.1.0").expect("uninstall");

    assert!(!paths.apps.join("ripgrep").join("14.1.0").exists());
}
```

- [ ] **Step 2: Run engine tests and verify failure**

Run: `cargo test --test engine_lifecycle_tests`

Expected: FAIL because `Engine` and local file install support are not implemented.

- [ ] **Step 3: Extend download module for `file://` test sources**

Modify `src/engine/download.rs` so `download_to_cache` starts with this branch:

```rust
if let Some(path) = url.strip_prefix("file://") {
    let source = PathBuf::from(path);
    let target = cache_dir.join(file_name);
    tokio::fs::create_dir_all(cache_dir)
        .await
        .map_err(|source_error| PvError::Io {
            path: cache_dir.to_path_buf(),
            source: source_error,
        })?;
    tokio::fs::copy(&source, &target)
        .await
        .map_err(|source_error| PvError::Io {
            path: target.clone(),
            source: source_error,
        })?;
    if let Err(error) = verify_sha256(&target, expected_hash) {
        let _ = tokio::fs::remove_file(&target).await;
        return Err(error);
    }
    return Ok(target);
}
```

Add `use std::path::PathBuf;` if it is missing.

- [ ] **Step 4: Implement engine orchestration**

Replace `src/engine/mod.rs`:

```rust
use std::fs;

use crate::bucket::{BucketManager, SearchResult};
use crate::config::{Config, Paths};
use crate::engine::download::download_to_cache;
use crate::engine::install::{install_package, InstallContext};
use crate::error::{PvError, Result};
use crate::manifest::Manifest;
use crate::package::{ExactPackageSpec, PackageSpec};
use crate::platform::Platform;
use crate::shim::{shim_config_path, shim_exe_path, target_for_bin};
use crate::version::{InstalledVersion, VersionManager};

pub mod download;
pub mod install;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageListing {
    pub package: String,
    pub versions: Vec<InstalledVersion>,
}

pub struct Engine<'a, P: Platform> {
    paths: Paths,
    config: Config,
    platform: &'a P,
}

impl<'a, P: Platform> Engine<'a, P> {
    pub fn new(paths: Paths, config: Config, platform: &'a P) -> Self {
        Self {
            paths,
            config,
            platform,
        }
    }

    pub async fn install(&mut self, input: &str) -> Result<()> {
        self.ensure_initialized()?;
        let spec = PackageSpec::parse(input)?;
        let bucket = BucketManager::new(self.paths.clone(), self.config.clone());
        let manifest = bucket.find_manifest(&spec.name, spec.version.as_deref())?;
        let arch = manifest.x64().expect("manifest validation ensures x64");
        let file_name = cache_file_name(&manifest);
        let cached = download_to_cache(&arch.url, &arch.hash, &self.paths.cache, &file_name).await?;
        let version_dir = self.paths.apps.join(&manifest.name).join(&manifest.version);

        if !version_dir.exists() {
            install_package(
                &manifest,
                &InstallContext::new(cached, version_dir.clone(), self.paths.tools.clone()),
            )?;
            self.cache_manifest(&manifest)?;
        }

        self.activate_manifest(&manifest)?;
        self.config.set_active_version(&manifest.name, &manifest.version);
        self.config.save(&self.paths)?;
        Ok(())
    }

    pub fn use_version(&mut self, input: &str) -> Result<()> {
        let spec = ExactPackageSpec::parse(input)?;
        let manager = VersionManager::new(self.paths.apps.clone(), self.platform);
        manager.activate(&spec.name, &spec.version)?;
        self.config.set_active_version(&spec.name, &spec.version);
        self.config.save(&self.paths)
    }

    pub fn uninstall(&mut self, input: &str) -> Result<()> {
        let spec = PackageSpec::parse(input)?;
        let package_dir = self.paths.apps.join(&spec.name);

        if let Some(version) = spec.version {
            let version_dir = package_dir.join(&version);
            VersionManager::<P>::remove_version_dir(&version_dir)?;
            if self.config.active_version(&spec.name) == Some(version.as_str()) {
                self.platform.remove_active_link(&package_dir.join("current"))?;
                self.remove_shims_for_package(&spec.name)?;
                self.config.clear_active_version(&spec.name);
            }
        } else if package_dir.exists() {
            self.remove_shims_for_package(&spec.name)?;
            fs::remove_dir_all(&package_dir).map_err(|source| PvError::Io {
                path: package_dir.clone(),
                source,
            })?;
            self.config.clear_active_version(&spec.name);
        }

        self.config.save(&self.paths)
    }

    pub fn list(&self, package: Option<&str>) -> Result<Vec<PackageListing>> {
        let manager = VersionManager::new(self.paths.apps.clone(), self.platform);
        let mut listings = Vec::new();

        if let Some(package) = package {
            listings.push(PackageListing {
                package: package.to_string(),
                versions: manager.list_versions(package, self.config.active_version(package))?,
            });
            return Ok(listings);
        }

        if !self.paths.apps.exists() {
            return Ok(listings);
        }

        for entry in fs::read_dir(&self.paths.apps).map_err(|source| PvError::Io {
            path: self.paths.apps.clone(),
            source,
        })? {
            let entry = entry.map_err(|source| PvError::Io {
                path: self.paths.apps.clone(),
                source,
            })?;
            if entry.path().is_dir() {
                let package = entry.file_name().to_string_lossy().to_string();
                listings.push(PackageListing {
                    versions: manager.list_versions(&package, self.config.active_version(&package))?,
                    package,
                });
            }
        }

        listings.sort_by(|left, right| left.package.cmp(&right.package));
        Ok(listings)
    }

    pub fn search(&self, keyword: &str) -> Result<Vec<SearchResult>> {
        BucketManager::new(self.paths.clone(), self.config.clone()).search(keyword)
    }

    pub fn info(&self, package: &str) -> Result<Manifest> {
        BucketManager::new(self.paths.clone(), self.config.clone()).find_manifest(package, None)
    }

    fn ensure_initialized(&mut self) -> Result<()> {
        self.paths.ensure_dirs()?;
        if !self.config.path_registered {
            self.platform.register_path(&self.paths.shims)?;
            self.config.path_registered = true;
        }
        Ok(())
    }

    fn activate_manifest(&self, manifest: &Manifest) -> Result<()> {
        let manager = VersionManager::new(self.paths.apps.clone(), self.platform);
        manager.activate(&manifest.name, &manifest.version)?;
        let arch = manifest.x64().expect("manifest validation ensures x64");
        for bin in &arch.bin {
            let target = target_for_bin(&self.paths.apps, &manifest.name, bin);
            self.platform.create_shim(
                bin,
                &shim_exe_path(&self.paths.shims, bin),
                &shim_config_path(&self.paths.shims, bin),
                &target,
            )?;
        }
        Ok(())
    }

    fn remove_shims_for_package(&self, package: &str) -> Result<()> {
        let manifest_path = self.paths.apps.join(package).join(".manifest.toml");
        if !manifest_path.exists() {
            return Ok(());
        }
        let manifest = Manifest::from_path(&manifest_path)?;
        let arch = manifest.x64().expect("manifest validation ensures x64");
        for bin in &arch.bin {
            self.platform.remove_shim(
                &shim_exe_path(&self.paths.shims, bin),
                &shim_config_path(&self.paths.shims, bin),
            )?;
        }
        Ok(())
    }

    fn cache_manifest(&self, manifest: &Manifest) -> Result<()> {
        let package_dir = self.paths.apps.join(&manifest.name);
        fs::create_dir_all(&package_dir).map_err(|source| PvError::Io {
            path: package_dir.clone(),
            source,
        })?;
        let source = manifest
            .source_path
            .as_ref()
            .ok_or_else(|| PvError::Platform("manifest source path is required".to_string()))?;
        fs::copy(source, package_dir.join(".manifest.toml")).map_err(|source_error| PvError::Io {
            path: package_dir.join(".manifest.toml"),
            source: source_error,
        })?;
        Ok(())
    }
}

fn cache_file_name(manifest: &Manifest) -> String {
    let url = &manifest.x64().expect("manifest validation ensures x64").url;
    let suffix = url.rsplit('/').next().filter(|part| !part.is_empty()).unwrap_or("download.bin");
    format!("{}-{}-{suffix}", manifest.name, manifest.version)
}
```

- [ ] **Step 5: Run engine lifecycle tests**

Run: `cargo test --test engine_lifecycle_tests`

Expected: PASS, engine lifecycle test succeeds.

---

### Task 12: Detect Outdated Packages And Upgrade

**Files:**
- Modify: `src/update.rs`
- Modify: `src/engine/mod.rs`
- Test: `tests/update_tests.rs`

- [ ] **Step 1: Write outdated detection tests**

Create `tests/update_tests.rs`:

```rust
use pv::update::{compare_installed_to_available, OutdatedPackage};

#[test]
fn reports_available_version_greater_than_installed() {
    let result = compare_installed_to_available("node", "18.20.0", "20.11.0");

    assert_eq!(
        result,
        Some(OutdatedPackage {
            package: "node".to_string(),
            installed: "18.20.0".to_string(),
            available: "20.11.0".to_string(),
        })
    );
}

#[test]
fn ignores_equal_or_older_available_versions() {
    assert_eq!(compare_installed_to_available("node", "20.11.0", "20.11.0"), None);
    assert_eq!(compare_installed_to_available("node", "20.11.0", "18.20.0"), None);
}
```

- [ ] **Step 2: Run update tests and verify failure**

Run: `cargo test --test update_tests`

Expected: FAIL because update comparison is not implemented.

- [ ] **Step 3: Implement update comparison**

Replace `src/update.rs`:

```rust
use semver::Version;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutdatedPackage {
    pub package: String,
    pub installed: String,
    pub available: String,
}

pub fn compare_installed_to_available(package: &str, installed: &str, available: &str) -> Option<OutdatedPackage> {
    if version_gt(available, installed) {
        Some(OutdatedPackage {
            package: package.to_string(),
            installed: installed.to_string(),
            available: available.to_string(),
        })
    } else {
        None
    }
}

pub fn version_gt(left: &str, right: &str) -> bool {
    match (Version::parse(left), Version::parse(right)) {
        (Ok(left), Ok(right)) => left > right,
        _ => left > right,
    }
}
```

- [ ] **Step 4: Add engine outdated and upgrade methods**

Add these methods inside `impl<'a, P: Platform> Engine<'a, P>` in `src/engine/mod.rs`:

```rust
pub fn outdated(&self) -> Result<Vec<crate::update::OutdatedPackage>> {
    let bucket = BucketManager::new(self.paths.clone(), self.config.clone());
    let mut results = Vec::new();

    for (package, installed) in &self.config.active_versions {
        let available = bucket.find_manifest(package, None)?;
        if let Some(outdated) =
            crate::update::compare_installed_to_available(package, installed, &available.version)
        {
            results.push(outdated);
        }
    }

    Ok(results)
}

pub async fn upgrade(&mut self, package: Option<&str>) -> Result<()> {
    if let Some(package) = package {
        let manifest = BucketManager::new(self.paths.clone(), self.config.clone())
            .find_manifest(package, None)?;
        self.install(&format!("{}@{}", manifest.name, manifest.version)).await
    } else {
        let outdated = self.outdated()?;
        for item in outdated {
            self.install(&format!("{}@{}", item.package, item.available)).await?;
        }
        Ok(())
    }
}
```

- [ ] **Step 5: Run update tests**

Run: `cargo test --test update_tests`

Expected: PASS, update tests succeed.

---

### Task 13: Implement Shell Session Switching

**Files:**
- Modify: `src/engine/mod.rs`
- Test: `tests/shell_tests.rs`

- [ ] **Step 1: Write shell switching test**

Create `tests/shell_tests.rs`:

```rust
use std::cell::RefCell;
use std::fs;
use std::path::{Path, PathBuf};

use pv::config::{Config, Paths};
use pv::engine::Engine;
use pv::platform::Platform;
use tempfile::tempdir;

#[derive(Default)]
struct FakePlatform {
    path_prefixes: RefCell<Vec<PathBuf>>,
}

impl Platform for FakePlatform {
    fn make_active_link(&self, _target: &Path, _link: &Path) -> pv::error::Result<()> { Ok(()) }
    fn remove_active_link(&self, _link: &Path) -> pv::error::Result<()> { Ok(()) }
    fn create_shim(&self, _exe_name: &str, _shim_exe: &Path, _shim_config: &Path, _target: &Path) -> pv::error::Result<()> { Ok(()) }
    fn remove_shim(&self, _shim_exe: &Path, _shim_config: &Path) -> pv::error::Result<()> { Ok(()) }
    fn register_path(&self, _dir: &Path) -> pv::error::Result<()> { Ok(()) }
    fn exe_ext(&self) -> &'static str { ".exe" }
    fn spawn_shell_with_path(&self, path_prefix: &Path) -> pv::error::Result<()> {
        self.path_prefixes.borrow_mut().push(path_prefix.to_path_buf());
        Ok(())
    }
}

#[test]
fn shell_uses_requested_version_directory_as_path_prefix() {
    let home = tempdir().expect("tempdir");
    let paths = Paths::from_home(home.path());
    fs::create_dir_all(paths.apps.join("node").join("20.11.0")).expect("version dir");
    let platform = FakePlatform::default();
    let engine = Engine::new(paths.clone(), Config::default(), &platform);

    engine.shell("node@20.11.0").expect("shell");

    assert_eq!(
        platform.path_prefixes.borrow().as_slice(),
        &[paths.apps.join("node").join("20.11.0")]
    );
}
```

- [ ] **Step 2: Run shell tests and verify failure**

Run: `cargo test --test shell_tests`

Expected: FAIL because `Engine::shell` is not implemented.

- [ ] **Step 3: Implement shell switching**

Add this method inside `impl<'a, P: Platform> Engine<'a, P>` in `src/engine/mod.rs`:

```rust
pub fn shell(&self, input: &str) -> Result<()> {
    let spec = ExactPackageSpec::parse(input)?;
    let version_dir = self.paths.apps.join(&spec.name).join(&spec.version);
    if !version_dir.exists() {
        return Err(PvError::VersionNotFound {
            package: spec.name,
            version: spec.version,
        });
    }
    self.platform.spawn_shell_with_path(&version_dir)
}
```

- [ ] **Step 4: Run shell tests**

Run: `cargo test --test shell_tests`

Expected: PASS, shell switching test succeeds.

---

### Task 14: Wire Real CLI Dispatch

**Files:**
- Modify: `src/cli.rs`
- Test: `tests/cli_tests.rs`

- [ ] **Step 1: Add command output tests**

Append to `tests/cli_tests.rs`:

```rust
#[test]
fn search_requires_keyword() {
    let mut cmd = Command::cargo_bin("pv").expect("pv binary is built");

    cmd.arg("search")
        .assert()
        .failure()
        .stderr(predicate::str::contains("required"));
}
```

- [ ] **Step 2: Run CLI tests**

Run: `cargo test --test cli_tests`

Expected: PASS for help/version and PASS for clap argument validation.

- [ ] **Step 3: Replace placeholder CLI dispatch with engine calls**

Modify `src/cli.rs` so `run()` is:

```rust
pub async fn run() -> Result<()> {
    let cli = Cli::parse();
    let paths = crate::config::Paths::discover()?;
    let config = crate::config::Config::load_or_default(&paths)?;
    #[cfg(windows)]
    let platform = crate::platform::windows::WindowsPlatform;
    #[cfg(not(windows))]
    let platform = UnsupportedCliPlatform;
    let mut engine = crate::engine::Engine::new(paths.clone(), config, &platform);

    match cli.command {
        Commands::Install(arg) => {
            engine.install(&arg.package).await?;
            println!("installed {}", arg.package);
        }
        Commands::Uninstall(arg) => {
            engine.uninstall(&arg.package)?;
            println!("uninstalled {}", arg.package);
        }
        Commands::Search(arg) => {
            for result in engine.search(&arg.keyword)? {
                println!("{} {} {}", result.name, result.version, result.description.unwrap_or_default());
            }
        }
        Commands::List(arg) => {
            for listing in engine.list(arg.package.as_deref())? {
                for version in listing.versions {
                    let marker = if version.active { "*" } else { " " };
                    println!("{marker} {} {}", listing.package, version.version);
                }
            }
        }
        Commands::Use(arg) => {
            engine.use_version(&arg.package)?;
            println!("using {}", arg.package);
        }
        Commands::Shell(arg) => {
            engine.shell(&arg.package)?;
        }
        Commands::Info(arg) => {
            let manifest = engine.info(&arg.package)?;
            println!("{} {}", manifest.name, manifest.version);
            if let Some(description) = manifest.description {
                println!("{description}");
            }
        }
        Commands::Outdated => {
            for item in engine.outdated()? {
                println!("{} {} -> {}", item.package, item.installed, item.available);
            }
        }
        Commands::Upgrade(arg) => {
            engine.upgrade(arg.package.as_deref()).await?;
            println!("upgrade complete");
        }
        Commands::Sync => {
            let manager = crate::bucket::BucketManager::new(paths, engine.config().clone());
            manager.sync()?;
            println!("sync complete");
        }
        Commands::Bucket(arg) => handle_bucket_command(arg).await?,
    }

    Ok(())
}
```

Also add the helper methods and unsupported platform below `run()`:

```rust
async fn handle_bucket_command(arg: BucketCommand) -> Result<()> {
    let paths = crate::config::Paths::discover()?;
    let mut config = crate::config::Config::load_or_default(&paths)?;
    match arg.command {
        BucketSubcommand::Add(add) => {
            let mut manager = crate::bucket::BucketManager::new(paths.clone(), config.clone());
            manager.add(&add.name, &add.url)?;
            config.add_bucket(&add.name, &add.url)?;
            config.save(&paths)?;
            println!("bucket added {}", add.name);
        }
        BucketSubcommand::List => {
            for bucket in &config.buckets {
                println!("{} {}", bucket.name, bucket.url);
            }
        }
        BucketSubcommand::Rm(remove) => {
            let mut manager = crate::bucket::BucketManager::new(paths.clone(), config.clone());
            manager.remove(&remove.name)?;
            config.remove_bucket(&remove.name)?;
            config.save(&paths)?;
            println!("bucket removed {}", remove.name);
        }
    }
    Ok(())
}

#[cfg(not(windows))]
struct UnsupportedCliPlatform;

#[cfg(not(windows))]
impl crate::platform::Platform for UnsupportedCliPlatform {
    fn make_active_link(&self, _target: &std::path::Path, _link: &std::path::Path) -> crate::error::Result<()> {
        Err(crate::error::PvError::UnsupportedPlatform("Windows MVP only".to_string()))
    }
    fn remove_active_link(&self, _link: &std::path::Path) -> crate::error::Result<()> { Ok(()) }
    fn create_shim(&self, _exe_name: &str, _shim_exe: &std::path::Path, _shim_config: &std::path::Path, _target: &std::path::Path) -> crate::error::Result<()> {
        Err(crate::error::PvError::UnsupportedPlatform("Windows MVP only".to_string()))
    }
    fn remove_shim(&self, _shim_exe: &std::path::Path, _shim_config: &std::path::Path) -> crate::error::Result<()> { Ok(()) }
    fn register_path(&self, _dir: &std::path::Path) -> crate::error::Result<()> { Ok(()) }
    fn exe_ext(&self) -> &'static str { "" }
    fn spawn_shell_with_path(&self, _path_prefix: &std::path::Path) -> crate::error::Result<()> {
        Err(crate::error::PvError::UnsupportedPlatform("Windows MVP only".to_string()))
    }
}
```

Add this accessor to `Engine` in `src/engine/mod.rs`:

```rust
pub fn config(&self) -> &Config {
    &self.config
}
```

- [ ] **Step 4: Run CLI tests**

Run: `cargo test --test cli_tests`

Expected: PASS, all CLI tests succeed.

---

### Task 15: Fix Bucket Add/Remove Ownership

**Files:**
- Modify: `src/bucket.rs`
- Modify: `src/cli.rs`
- Test: `tests/bucket_tests.rs`

- [ ] **Step 1: Add a config ownership regression test**

Append to `tests/bucket_tests.rs`:

```rust
#[test]
fn bucket_manager_adds_config_record_without_duplicate_external_add() {
    let home = tempdir().expect("tempdir");
    let paths = Paths::from_home(home.path());
    let config = Config::default();
    let mut manager = BucketManager::new(paths, config);

    manager.add_config_record("main", "https://example.invalid/main.git").expect("add config");

    assert_eq!(manager.config().buckets.len(), 1);
    assert!(manager.add_config_record("main", "https://example.invalid/main.git").is_err());
}
```

- [ ] **Step 2: Run bucket tests**

Run: `cargo test --test bucket_tests`

Expected: FAIL because `add_config_record` and `config` accessors do not exist.

- [ ] **Step 3: Split clone/remove filesystem actions from config mutation**

Modify `src/bucket.rs`:

```rust
impl BucketManager {
    pub fn config(&self) -> &Config {
        &self.config
    }

    pub fn into_config(self) -> Config {
        self.config
    }

    pub fn add_config_record(&mut self, name: &str, url: &str) -> Result<()> {
        self.config.add_bucket(name, url)
    }

    pub fn remove_config_record(&mut self, name: &str) -> Result<()> {
        self.config.remove_bucket(name)
    }
}
```

Update `add()` to clone only and then call `self.add_config_record(name, url)`. Update `remove()` to delete the directory only and then call `self.remove_config_record(name)`.

- [ ] **Step 4: Update bucket CLI persistence**

In `src/cli.rs`, replace the `BucketSubcommand::Add` and `BucketSubcommand::Rm` arms with:

```rust
BucketSubcommand::Add(add) => {
    let mut manager = crate::bucket::BucketManager::new(paths.clone(), config);
    manager.add(&add.name, &add.url)?;
    manager.config().save(&paths)?;
    println!("bucket added {}", add.name);
}
BucketSubcommand::Rm(remove) => {
    let mut manager = crate::bucket::BucketManager::new(paths.clone(), config);
    manager.remove(&remove.name)?;
    manager.config().save(&paths)?;
    println!("bucket removed {}", remove.name);
}
```

- [ ] **Step 5: Run bucket and CLI tests**

Run: `cargo test --test bucket_tests --test cli_tests`

Expected: PASS, bucket config mutation is single-owned and CLI tests still pass.

---

### Task 16: Add Full Lifecycle Integration Test

**Files:**
- Create: `tests/integration_lifecycle.rs`
- Create: `tests/fixtures/bucket/main/ripgrep.toml`
- Create: `tests/fixtures/bucket/main/node-18.toml`
- Create: `tests/fixtures/bucket/main/node-20.toml`

- [ ] **Step 1: Write the integration test**

Create `tests/integration_lifecycle.rs`:

```rust
use std::cell::RefCell;
use std::fs;
use std::path::{Path, PathBuf};

use pv::config::{BucketConfig, Config, Paths};
use pv::engine::Engine;
use pv::platform::Platform;
use tempfile::tempdir;

#[derive(Default)]
struct FakePlatform {
    links: RefCell<Vec<(PathBuf, PathBuf)>>,
    removed_shims: RefCell<usize>,
}

impl Platform for FakePlatform {
    fn make_active_link(&self, target: &Path, link: &Path) -> pv::error::Result<()> {
        self.links.borrow_mut().push((target.to_path_buf(), link.to_path_buf()));
        Ok(())
    }
    fn remove_active_link(&self, _link: &Path) -> pv::error::Result<()> { Ok(()) }
    fn create_shim(&self, _exe_name: &str, _shim_exe: &Path, _shim_config: &Path, _target: &Path) -> pv::error::Result<()> { Ok(()) }
    fn remove_shim(&self, _shim_exe: &Path, _shim_config: &Path) -> pv::error::Result<()> {
        *self.removed_shims.borrow_mut() += 1;
        Ok(())
    }
    fn register_path(&self, _dir: &Path) -> pv::error::Result<()> { Ok(()) }
    fn exe_ext(&self) -> &'static str { ".exe" }
    fn spawn_shell_with_path(&self, _path_prefix: &Path) -> pv::error::Result<()> { Ok(()) }
}

fn write_single_manifest(path: &Path, name: &str, version: &str, source: &Path, hash: &str, bin: &str) {
    fs::write(
        path,
        format!(
            r#"
name = "{name}"
version = "{version}"
description = "{name} fixture"
type = "single"

[architecture.x64]
url = "file://{}"
hash = "sha256:{hash}"
bin = ["{bin}"]
"#,
            source.display()
        ),
    )
    .expect("write manifest");
}

#[tokio::test]
async fn full_single_package_lifecycle() {
    let home = tempdir().expect("tempdir");
    let paths = Paths::from_home(home.path());
    let bucket = paths.buckets.join("main");
    fs::create_dir_all(&bucket).expect("bucket");
    let rg_source = home.path().join("rg.exe");
    fs::write(&rg_source, b"rg").expect("source");
    write_single_manifest(
        &bucket.join("ripgrep.toml"),
        "ripgrep",
        "14.1.0",
        &rg_source,
        "ac0d2a97f32c42102c6428b3207a5de0a336c55963d1ec95ed729b8d6f4ae4d8",
        "rg.exe",
    );

    let config = Config {
        buckets: vec![BucketConfig {
            name: "main".to_string(),
            url: "local".to_string(),
        }],
        active_versions: Default::default(),
        path_registered: false,
    };
    let platform = FakePlatform::default();
    let mut engine = Engine::new(paths.clone(), config, &platform);

    engine.install("ripgrep").await.expect("install");
    engine.use_version("ripgrep@14.1.0").expect("use");
    let list = engine.list(Some("ripgrep")).expect("list");
    let outdated = engine.outdated().expect("outdated");
    engine.uninstall("ripgrep").expect("uninstall all");

    assert_eq!(list[0].versions[0].version, "14.1.0");
    assert!(outdated.is_empty());
    assert!(!paths.apps.join("ripgrep").exists());
    assert_eq!(*platform.removed_shims.borrow(), 1);
}
```

- [ ] **Step 2: Run the integration test**

Run: `cargo test --test integration_lifecycle`

Expected: PASS, full single-package lifecycle succeeds.

- [ ] **Step 3: Add static fixture manifests for manual CLI testing**

Create `tests/fixtures/bucket/main/ripgrep.toml`:

```toml
name = "ripgrep"
version = "14.1.0"
description = "fast recursive search"
homepage = "https://github.com/BurntSushi/ripgrep"
license = "MIT"
type = "single"

[architecture.x64]
url = "https://example.invalid/rg.exe"
hash = "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
bin = ["rg.exe"]
```

Create `tests/fixtures/bucket/main/node-18.toml`:

```toml
name = "node"
version = "18.20.0"
description = "Node.js JavaScript runtime"
homepage = "https://nodejs.org"
license = "MIT"
type = "archive"
bin = ["node.exe", "npm.cmd", "npx.cmd"]

[architecture.x64]
url = "https://example.invalid/node-18.zip"
hash = "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
extract_dir = "node-v18.20.0-win-x64"
```

Create `tests/fixtures/bucket/main/node-20.toml`:

```toml
name = "node"
version = "20.11.0"
description = "Node.js JavaScript runtime"
homepage = "https://nodejs.org"
license = "MIT"
type = "archive"
bin = ["node.exe", "npm.cmd", "npx.cmd"]

[architecture.x64]
url = "https://example.invalid/node-20.zip"
hash = "sha256:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc"
extract_dir = "node-v20.11.0-win-x64"
```

- [ ] **Step 4: Run all tests**

Run: `cargo test`

Expected: PASS, every unit and integration test succeeds.

---

### Task 17: Manual Windows Verification

**Files:**
- No source changes expected.
- Verification target: built binaries under `target/debug/`.

- [ ] **Step 1: Build both binaries**

Run: `cargo build --bins`

Expected: PASS and these files exist:

```text
target/debug/pv.exe
target/debug/pv-shim.exe
```

- [ ] **Step 2: Verify CLI help**

Run: `target/debug/pv.exe --help`

Expected: output includes:

```text
install
uninstall
search
bucket
outdated
upgrade
```

- [ ] **Step 3: Verify test suite**

Run: `cargo test`

Expected: PASS, no failing tests.

- [ ] **Step 4: Verify no MVP scope drift**

Run: `rg -n "checkver|project config|shell hook|retry" src tests Cargo.toml`

Expected: no production implementation of checkver, project config files, shell hooks, or automatic retry behavior.

---

## Self-Review

**Spec coverage:** This plan covers crate layout, Windows MVP, TOML manifests, Git buckets, install/uninstall/search/list/use/shell/info/outdated/upgrade/sync/bucket commands, archive/single/installer strategies, `innounp` bootstrap into `PV_HOME/tools`, x64 architecture selection, PATH registration, junction activation, shim proxying, config persistence, and full lifecycle tests.

**Placeholder scan:** No deferred-work placeholder language is present. The Inno installer path includes a concrete `innounp` bootstrap flow with pinned URL, archive hash, local reuse test, and extraction command wiring.

**Type consistency:** `PackageSpec`, `ExactPackageSpec`, `Manifest`, `BucketManager`, `Platform`, `VersionManager`, `Engine`, `OutdatedPackage`, and shim helper names are used consistently across tests and implementation steps.

**Known implementation checkpoint:** Task 14 and Task 15 intentionally refine bucket config ownership after initial CLI wiring. Execute them in order so `BucketManager` owns bucket mutation consistently by the end of the plan.
