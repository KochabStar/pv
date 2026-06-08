# Terminal Progress And Aria2 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Improve install/download terminal feedback for `pv`, add progress to the bootstrap installer script, and support optional aria2 downloads.

**Architecture:** Add a small terminal rendering module and extend the existing download module with explicit options and aria2 command construction. Keep package installation behavior in `src/engine/mod.rs`, but move low-level progress and downloader selection out of install logic.

**Tech Stack:** Rust 2021, `indicatif`, `reqwest` streaming, `tokio`, `which`, PowerShell `Write-Progress`, optional external `aria2c`.

---

## File Structure

- Modify `src/config.rs`: add `DownloadConfig` under `Config` with safe defaults.
- Create `src/terminal.rs`: status, spinner, and progress bar helpers.
- Modify `src/lib.rs`: export `terminal`.
- Modify `src/engine/download.rs`: introduce `DownloadOptions`, built-in streaming download, aria2 command builder, and backend selection.
- Modify `src/engine/mod.rs`: pass download config, show install/activation phases, remove ambiguous duplicate output responsibility from CLI.
- Modify `src/cli.rs`: remove duplicate install/upgrade success lines that conflict with resolved version output.
- Modify `scripts/install.ps1`: add aria2-aware download and stream fallback with `Write-Progress`.
- Modify tests in `tests/config_tests.rs`, `tests/download_tests.rs`, `tests/cli_tests.rs`, `tests/install_script_tests.rs`, and likely `tests/engine_lifecycle_tests.rs`.

## Task 1: Download Config

**Files:**
- Modify: `src/config.rs`
- Test: `tests/config_tests.rs`

- [ ] **Step 1: Write failing config tests**

Add tests that expect default download config and TOML round-trip:

```rust
#[test]
fn default_download_config_disables_aria2() {
    let config = Config::default();

    assert!(!config.download.aria2_enabled);
    assert_eq!(config.download.aria2_split, 5);
    assert_eq!(config.download.aria2_max_connection_per_server, 5);
    assert_eq!(config.download.aria2_min_split_size, "5M");
}

#[test]
fn loads_download_config_from_toml() {
    let home = tempdir().expect("tempdir");
    let paths = Paths::from_home(home.path());
    std::fs::write(
        &paths.config_file,
        r#"
[download]
aria2_enabled = true
aria2_split = 8
aria2_max_connection_per_server = 4
aria2_min_split_size = "10M"
"#,
    )
    .expect("write config");

    let config = Config::load_or_default(&paths).expect("load config");

    assert!(config.download.aria2_enabled);
    assert_eq!(config.download.aria2_split, 8);
    assert_eq!(config.download.aria2_max_connection_per_server, 4);
    assert_eq!(config.download.aria2_min_split_size, "10M");
}
```

- [ ] **Step 2: Run red test**

Run:

```powershell
cargo test --test config_tests
```

Expected: fails because `Config` has no `download` field.

- [ ] **Step 3: Implement minimal config**

Add:

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct DownloadConfig {
    pub aria2_enabled: bool,
    pub aria2_split: u32,
    pub aria2_max_connection_per_server: u32,
    pub aria2_min_split_size: String,
}

impl Default for DownloadConfig {
    fn default() -> Self {
        Self {
            aria2_enabled: false,
            aria2_split: 5,
            aria2_max_connection_per_server: 5,
            aria2_min_split_size: "5M".to_string(),
        }
    }
}
```

Then add `pub download: DownloadConfig` to `Config`.

- [ ] **Step 4: Run green test**

Run:

```powershell
cargo test --test config_tests
```

Expected: all config tests pass.

## Task 2: Terminal Helpers

**Files:**
- Create: `src/terminal.rs`
- Modify: `src/lib.rs`
- Test: no direct unit tests required; helpers are exercised through download and CLI tests.

- [ ] **Step 1: Create minimal terminal module**

Implement helpers:

```rust
use indicatif::{ProgressBar, ProgressStyle};

pub fn status(message: impl AsRef<str>) {
    println!("{}", message.as_ref());
}

pub fn install_start(package: &str, version: &str) {
    status(format!("==> Installing {package}@{version}"));
}

pub fn install_success(package: &str, version: &str) {
    status(format!("Installed {package}@{version}"));
}

pub fn using_version(package: &str, version: &str) {
    status(format!("Using {package}@{version}"));
}

pub fn spinner(message: impl Into<String>) -> ProgressBar {
    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::with_template("{spinner:.green} {msg}")
            .unwrap_or_else(|_| ProgressStyle::default_spinner()),
    );
    spinner.enable_steady_tick(std::time::Duration::from_millis(100));
    spinner.set_message(message.into());
    spinner
}

pub fn download_bar(total: u64, message: impl Into<String>) -> ProgressBar {
    let bar = ProgressBar::new(total);
    bar.set_style(
        ProgressStyle::with_template(
            "{msg} [{bar:40.cyan/blue}] {bytes}/{total_bytes} {bytes_per_sec} ETA {eta}",
        )
        .unwrap_or_else(|_| ProgressStyle::default_bar())
        .progress_chars("=>-"),
    );
    bar.set_message(message.into());
    bar
}
```

- [ ] **Step 2: Export module**

Add to `src/lib.rs`:

```rust
pub mod terminal;
```

- [ ] **Step 3: Compile check**

Run:

```powershell
cargo test --lib
```

Expected: library tests compile and pass.

## Task 3: Built-In Streaming Download Progress

**Files:**
- Modify: `src/engine/download.rs`
- Test: `tests/download_tests.rs`

- [ ] **Step 1: Write failing download API tests**

Update tests to use `DownloadOptions` and add local file copy coverage:

```rust
use pv::config::DownloadConfig;
use pv::engine::download::{download_to_cache, verify_sha256, DownloadOptions};

#[tokio::test]
async fn copies_local_file_url_to_cache_and_checks_hash() {
    let source_dir = tempdir().expect("source tempdir");
    let cache_dir = tempdir().expect("cache tempdir");
    let source = source_dir.path().join("payload.txt");
    fs::write(&source, b"hello").expect("write payload");

    let file = download_to_cache(
        &format!("file://{}", source.display()),
        cache_dir.path(),
        "payload.txt",
        "sha256:2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824",
        &DownloadOptions::from_config(&DownloadConfig::default()),
    )
    .await
    .expect("download");

    assert_eq!(fs::read(file).expect("read file"), b"hello");
}
```

Also update the existing HTTP test to pass `&DownloadOptions::from_config(&DownloadConfig::default())`.

- [ ] **Step 2: Run red test**

Run:

```powershell
cargo test --test download_tests
```

Expected: fails because `DownloadOptions` does not exist and `download_to_cache` signature has not changed.

- [ ] **Step 3: Implement minimal streaming download**

Add:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DownloadOptions {
    pub aria2_enabled: bool,
    pub aria2_split: u32,
    pub aria2_max_connection_per_server: u32,
    pub aria2_min_split_size: String,
}

impl DownloadOptions {
    pub fn from_config(config: &crate::config::DownloadConfig) -> Self {
        Self {
            aria2_enabled: config.aria2_enabled,
            aria2_split: config.aria2_split,
            aria2_max_connection_per_server: config.aria2_max_connection_per_server,
            aria2_min_split_size: config.aria2_min_split_size.clone(),
        }
    }
}
```

Change `download_to_cache` to accept `options: &DownloadOptions`. Use `response.bytes_stream()` with `futures_util::StreamExt`, create a file, write chunks through `tokio::io::AsyncWriteExt`, and update progress through `crate::terminal::download_bar` when `content_length()` is available.

- [ ] **Step 4: Run green test**

Run:

```powershell
cargo test --test download_tests
```

Expected: all download tests pass.

## Task 4: Aria2 Command Builder And Backend

**Files:**
- Modify: `src/engine/download.rs`
- Test: `tests/download_tests.rs`

- [ ] **Step 1: Write failing aria2 command test**

Add:

```rust
use std::path::Path;
use pv::engine::download::{build_aria2_command, DownloadOptions};

#[test]
fn builds_aria2_command_with_scoop_style_options() {
    let options = DownloadOptions {
        aria2_enabled: true,
        aria2_split: 8,
        aria2_max_connection_per_server: 4,
        aria2_min_split_size: "10M".to_string(),
    };

    let command = build_aria2_command(
        "https://example.invalid/tool.zip",
        Path::new("C:/cache"),
        "tool.zip",
        &options,
    );

    assert_eq!(command.program, "aria2c");
    assert!(command.args.contains(&"--allow-overwrite=true".to_string()));
    assert!(command.args.contains(&"--auto-file-renaming=false".to_string()));
    assert!(command.args.contains(&"--continue=true".to_string()));
    assert!(command.args.contains(&"--split=8".to_string()));
    assert!(command.args.contains(&"--max-connection-per-server=4".to_string()));
    assert!(command.args.contains(&"--min-split-size=10M".to_string()));
    assert!(command.args.contains(&"https://example.invalid/tool.zip".to_string()));
}
```

- [ ] **Step 2: Run red test**

Run:

```powershell
cargo test --test download_tests builds_aria2_command_with_scoop_style_options
```

Expected: fails because `build_aria2_command` does not exist.

- [ ] **Step 3: Implement command builder and backend selection**

Add:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExternalCommand {
    pub program: String,
    pub args: Vec<String>,
}

pub fn build_aria2_command(
    url: &str,
    cache_dir: &Path,
    file_name: &str,
    options: &DownloadOptions,
) -> ExternalCommand {
    ExternalCommand {
        program: "aria2c".to_string(),
        args: vec![
            "--allow-overwrite=true".to_string(),
            "--auto-file-renaming=false".to_string(),
            "--continue=true".to_string(),
            format!("--dir={}", cache_dir.display()),
            format!("--out={file_name}"),
            format!("--split={}", options.aria2_split),
            format!(
                "--max-connection-per-server={}",
                options.aria2_max_connection_per_server
            ),
            format!("--min-split-size={}", options.aria2_min_split_size),
            url.to_string(),
        ],
    }
}
```

In `download_to_cache`, before built-in HTTP streaming, if `options.aria2_enabled && which::which("aria2c").is_ok()`, run aria2 via `std::process::Command`; if aria2 is enabled but unavailable, print a fallback status and continue with built-in download.

- [ ] **Step 4: Run green test**

Run:

```powershell
cargo test --test download_tests
```

Expected: all download tests pass.

## Task 5: Install Flow Output

**Files:**
- Modify: `src/engine/mod.rs`
- Modify: `src/cli.rs`
- Test: `tests/engine_lifecycle_tests.rs` or `tests/cli_tests.rs`

- [ ] **Step 1: Write failing output test**

Add a CLI-level test using an isolated `PV_HOME` fixture if available in existing lifecycle helpers. The assertion should require the resolved version message:

```rust
#[test]
fn install_output_mentions_resolved_package_version() {
    let mut cmd = Command::cargo_bin("pv").expect("pv binary is built");
    let home = tempfile::tempdir().expect("tempdir");

    cmd.env("PV_HOME", home.path())
        .arg("install")
        .arg("ripgrep")
        .assert()
        .success()
        .stdout(predicate::str::contains("Installing ripgrep@"))
        .stdout(predicate::str::contains("Installed ripgrep@"));
}
```

If this is too expensive because bucket fixtures are not wired into CLI tests, put the assertion in the existing engine lifecycle test and capture only engine side effects. Keep the test focused on resolved package/version output.

- [ ] **Step 2: Run red test**

Run:

```powershell
cargo test --test cli_tests install_output_mentions_resolved_package_version
```

Expected: fails because output is currently `installed ripgrep`, not resolved version progress.

- [ ] **Step 3: Implement install phase output**

In `Engine::install` after manifest resolution:

```rust
crate::terminal::install_start(&manifest.name, &manifest.version);
```

Before install strategies:

```rust
let spinner = crate::terminal::spinner("Installing package");
install_manifest(&manifest, &source, &version_dir, &self.paths.home)?;
spinner.finish_and_clear();
```

Before activation:

```rust
let spinner = crate::terminal::spinner(format!("Activating {}@{}", manifest.name, manifest.version));
VersionManager::new(...).activate(...)?;
spinner.finish_and_clear();
crate::terminal::install_success(&manifest.name, &manifest.version);
```

Change the download call to pass `&DownloadOptions::from_config(&self.config.download)`.

Remove or adjust `println!("installed {}", arg.package);` from `src/cli.rs` so output is not duplicated or less precise than engine output.

- [ ] **Step 4: Run green relevant tests**

Run:

```powershell
cargo test --test cli_tests
cargo test --test engine_lifecycle_tests
```

Expected: relevant CLI and engine lifecycle tests pass.

## Task 6: NPM And Upgrade Progress

**Files:**
- Modify: `src/engine/mod.rs`
- Modify: `src/engine/install/npm.rs` if needed
- Test: existing npm-related tests if present; otherwise compile plus engine tests.

- [ ] **Step 1: Add/adjust tests for upgrade output if practical**

If existing tests can create outdated fixtures, assert upgrade no longer only prints `upgrade complete` and uses `install` output per package. If not practical, document manual verification and keep compile/test coverage.

- [ ] **Step 2: Implement minimal NPM status**

In `install_npm`, print:

```rust
crate::terminal::install_start(package, &resolved_version);
```

Wrap `install_manifest`, scan, and activation in status spinners. Use `install_success(package, &resolved_version)` after config save.

- [ ] **Step 3: Remove vague CLI upgrade success**

In `src/cli.rs`, remove `println!("upgrade complete")` if each package install reports success. If no packages are outdated, engine can print a clear `Already up to date`.

- [ ] **Step 4: Run tests**

Run:

```powershell
cargo test --test update_tests
cargo test --test engine_lifecycle_tests
```

Expected: tests pass.

## Task 7: Bootstrap Installer Progress And Aria2

**Files:**
- Modify: `scripts/install.ps1`
- Test: `tests/install_script_tests.rs`

- [ ] **Step 1: Write failing script tests**

Add assertions:

```rust
#[test]
fn install_script_supports_aria2_and_write_progress() {
    let script = std::fs::read_to_string("scripts/install.ps1").expect("read install script");

    assert!(script.contains("aria2c"));
    assert!(script.contains("Write-Progress"));
    assert!(script.contains("Invoke-WebRequestStream"));
}
```

- [ ] **Step 2: Run red test**

Run:

```powershell
cargo test --test install_script_tests install_script_supports_aria2_and_write_progress
```

Expected: fails because script does not contain aria2 stream progress logic.

- [ ] **Step 3: Implement script download helpers**

Add:

```powershell
function Invoke-WebRequestStream {
    param(
        [string] $Url,
        [string] $OutFile
    )

    $response = Invoke-WebRequest -Uri $Url -UseBasicParsing
    $bytes = $response.Content
    if ($bytes -is [string]) {
        $bytes = [Text.Encoding]::UTF8.GetBytes($bytes)
    }

    $total = $bytes.Length
    Write-Progress -Activity "Downloading pv asset" -Status $Url -PercentComplete 0
    [IO.File]::WriteAllBytes($OutFile, $bytes)
    Write-Progress -Activity "Downloading pv asset" -Completed
}
```

For a more accurate implementation, use `[System.Net.Http.HttpClient]` and stream chunks to file while updating `Write-Progress`.

Update `Invoke-DownloadFile`:

```powershell
if (Get-Command aria2c -ErrorAction SilentlyContinue) {
    Write-Status "downloading with aria2 $Url"
    aria2c --allow-overwrite=true --auto-file-renaming=false --continue=true --dir ([IO.Path]::GetDirectoryName($OutFile)) --out ([IO.Path]::GetFileName($OutFile)) $Url
    if ($LASTEXITCODE -ne 0) {
        throw "aria2 download failed with exit code $LASTEXITCODE"
    }
    return
}

Write-Status "downloading $Url"
Invoke-WebRequestStream -Url $Url -OutFile $OutFile
```

- [ ] **Step 4: Run green script tests**

Run:

```powershell
cargo test --test install_script_tests
```

Expected: tests pass, including the Windows local release zip install test when run on Windows.

## Task 8: Final Verification

**Files:**
- All modified files.

- [ ] **Step 1: Format**

Run:

```powershell
cargo fmt
```

Expected: exit code 0.

- [ ] **Step 2: Run full tests**

Run:

```powershell
cargo test
```

Expected: all tests pass.

- [ ] **Step 3: Inspect diff**

Run:

```powershell
git diff -- src tests scripts docs
```

Expected: diff only covers terminal progress, download config/backend, bootstrap installer progress, tests, and the design/plan docs.

## Self-Review

- Spec coverage: tasks cover Rust package install progress, Rust aria2 backend, PowerShell bootstrap progress, PowerShell aria2 support, and tests.
- Placeholder scan: no TODO/TBD/fill-in-later text remains.
- Type consistency: `DownloadConfig`, `DownloadOptions`, `build_aria2_command`, and `ExternalCommand` names are consistent across tasks.
- Repo constraint: no git commit steps are included because the user did not request commits.
