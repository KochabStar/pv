use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde::Deserialize;

use crate::bucket::BucketManager;
use crate::config::{Config, Paths};
use crate::engine::download::{download_to_cache, DownloadOptions};
use crate::engine::install::archive::ArchiveInstallStrategy;
use crate::engine::install::installer::InstallerInstallStrategy;
use crate::engine::install::npm::NpmInstallStrategy;
use crate::engine::install::single::SingleInstallStrategy;
use crate::engine::install::InstallStrategy;
use crate::error::{PvError, Result};
use crate::manifest::{InstallType, Manifest};
use crate::package::{ExactPackageSpec, PackageSpec};
use crate::platform::Platform;
use crate::version::{PackageListing, VersionManager};

pub mod download;
pub mod install;

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

    pub fn config(&self) -> &Config {
        &self.config
    }

    pub async fn install(&mut self, input: &str) -> Result<()> {
        self.paths.ensure_dirs()?;

        // 显式后端前缀：npm:<pkg> → NPM 注册表后端
        if let Some(package) = input.strip_prefix("npm:") {
            let spec = PackageSpec::parse(package)?;
            return self.install_npm(&spec.name, spec.version.as_deref()).await;
        }

        // 默认：bucket TOML 清单路径
        let spec = PackageSpec::parse(input)?;
        let manifest = BucketManager::new(self.paths.clone(), self.config.clone())
            .find_manifest(&spec.name, spec.version.as_deref())?;
        let version_dir = self.paths.apps.join(&manifest.name).join(&manifest.version);

        crate::terminal::install_start(&manifest.name, &manifest.version);
        if !version_dir.exists() {
            let arch = manifest.x64().expect("x64 architecture exists");
            let source_name = source_file_name(&arch.url);
            let download_options = DownloadOptions::from_config(&self.config.download);
            let source = download_to_cache(
                &arch.url,
                &self.paths.cache,
                &source_name,
                &arch.hash,
                &download_options,
            )
            .await?;
            let spinner = crate::terminal::spinner(format!(
                "Installing {}@{}",
                manifest.name, manifest.version
            ));
            install_manifest(&manifest, &source, &version_dir, &self.paths.home)?;
            spinner.finish_and_clear();
            write_installed_manifest(&manifest, &self.paths.apps.join(&manifest.name))?;
        } else {
            crate::terminal::already_installed(&manifest.name, &manifest.version);
        }

        let spinner =
            crate::terminal::spinner(format!("Activating {}@{}", manifest.name, manifest.version));
        VersionManager::new(self.paths.clone(), self.platform)
            .activate(&mut self.config, &manifest)?;
        spinner.finish_and_clear();
        self.config.save(&self.paths)?;
        crate::terminal::install_success(&manifest.name, &manifest.version);
        Ok(())
    }

    /// 安装 npm registry 包
    async fn install_npm(&mut self, package: &str, version: Option<&str>) -> Result<()> {
        let resolved_version = resolve_npm_version(package, version)?;
        let version_dir = self.paths.apps.join(package).join(&resolved_version);

        crate::terminal::install_start(package, &resolved_version);
        if !version_dir.exists() {
            // 先构造临时 manifest（bin 列表待扫描后填充）
            let temp_bins = vec![format!("{}.cmd", package)];
            let manifest = Manifest::npm_virtual(package, &resolved_version, temp_bins);

            let dummy_source = self.paths.cache.join("_npm_placeholder");
            let spinner =
                crate::terminal::spinner(format!("Installing {package}@{resolved_version}"));
            install_manifest(&manifest, &dummy_source, &version_dir, &self.paths.home)?;
            spinner.finish_and_clear();

            // 扫描 npm 安装后实际产生的可执行文件
            let actual_bins = scan_npm_bins(&version_dir)?;
            let manifest = Manifest::npm_virtual(package, &resolved_version, actual_bins);
            write_installed_manifest(&manifest, &self.paths.apps.join(package))?;

            let spinner =
                crate::terminal::spinner(format!("Activating {package}@{resolved_version}"));
            VersionManager::new(self.paths.clone(), self.platform)
                .activate(&mut self.config, &manifest)?;
            spinner.finish_and_clear();
        } else {
            crate::terminal::already_installed(package, &resolved_version);
            // 已安装：从缓存清单读取 bin 列表
            let manifest_path = self.paths.apps.join(package).join(".manifest.toml");
            let manifest = if manifest_path.exists() {
                Manifest::from_path(&manifest_path)?
            } else {
                Manifest::npm_virtual(package, &resolved_version, Vec::new())
            };
            let spinner =
                crate::terminal::spinner(format!("Activating {package}@{resolved_version}"));
            VersionManager::new(self.paths.clone(), self.platform)
                .activate(&mut self.config, &manifest)?;
            spinner.finish_and_clear();
        }

        self.config.save(&self.paths)?;
        crate::terminal::install_success(package, &resolved_version);
        Ok(())
    }

    pub fn use_version(&mut self, input: &str) -> Result<()> {
        let name = input.strip_prefix("npm:").unwrap_or(input);
        let spec = ExactPackageSpec::parse(name)?;
        let manifest = self.installed_manifest_or_bucket(&spec.name, Some(&spec.version))?;
        VersionManager::new(self.paths.clone(), self.platform)
            .activate(&mut self.config, &manifest)?;
        self.config.save(&self.paths)
    }

    pub fn uninstall(&mut self, input: &str) -> Result<()> {
        let name = input.strip_prefix("npm:").unwrap_or(input);
        let spec = PackageSpec::parse(name)?;
        VersionManager::new(self.paths.clone(), self.platform).uninstall(
            &mut self.config,
            &spec.name,
            spec.version.as_deref(),
        )?;
        self.config.save(&self.paths)
    }

    pub fn list(&self, package: Option<&str>) -> Result<Vec<PackageListing>> {
        VersionManager::new(self.paths.clone(), self.platform).list(&self.config, package)
    }

    pub fn search(&self, keyword: &str) -> Result<Vec<crate::bucket::SearchResult>> {
        if let Some(package) = keyword.strip_prefix("npm:") {
            return search_npm(package);
        }

        BucketManager::new(self.paths.clone(), self.config.clone()).search(keyword)
    }

    pub fn info(&self, package: &str) -> Result<Manifest> {
        let name = package.strip_prefix("npm:").unwrap_or(package);
        // npm 包：优先返回已安装清单
        let manifest_path = self.paths.apps.join(name).join(".manifest.toml");
        if manifest_path.exists() {
            return Manifest::from_path(&manifest_path);
        }
        let spec = PackageSpec::parse(name)?;
        BucketManager::new(self.paths.clone(), self.config.clone())
            .find_manifest(&spec.name, spec.version.as_deref())
    }

    pub fn outdated(&self) -> Result<Vec<crate::update::OutdatedPackage>> {
        let bucket = BucketManager::new(self.paths.clone(), self.config.clone());
        let mut results = Vec::new();

        for (package, installed) in &self.config.active_versions {
            let available = bucket.find_manifest(package, None)?;
            if let Some(outdated) = crate::update::compare_installed_to_available(
                package,
                installed,
                &available.version,
            ) {
                results.push(outdated);
            }
        }

        Ok(results)
    }

    pub async fn upgrade(&mut self, package: Option<&str>) -> Result<()> {
        if let Some(package) = package {
            let manifest = BucketManager::new(self.paths.clone(), self.config.clone())
                .find_manifest(package, None)?;
            self.install(&format!("{}@{}", manifest.name, manifest.version))
                .await
        } else {
            let outdated = self.outdated()?;
            if outdated.is_empty() {
                crate::terminal::already_up_to_date();
                return Ok(());
            }
            for item in outdated {
                self.install(&format!("{}@{}", item.package, item.available))
                    .await?;
            }
            Ok(())
        }
    }

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

    fn installed_manifest_or_bucket(
        &self,
        package: &str,
        version: Option<&str>,
    ) -> Result<Manifest> {
        let manifest_path = self.paths.apps.join(package).join(".manifest.toml");
        if manifest_path.exists() {
            let manifest = Manifest::from_path(&manifest_path)?;
            if version.is_none() || version == Some(manifest.version.as_str()) {
                return Ok(manifest);
            }
        }
        BucketManager::new(self.paths.clone(), self.config.clone()).find_manifest(package, version)
    }

    // ── cache ──

    /// 显示缓存目录路径和总大小
    pub fn cache_show(&self) -> Result<()> {
        let cache = &self.paths.cache;
        if !cache.exists() {
            println!("{} (empty)", cache.display());
            return Ok(());
        }
        let size = dir_size(cache)?;
        println!("{}  {}", format_bytes(size), cache.display());
        Ok(())
    }

    /// 清空缓存目录
    pub fn cache_clean(&self) -> Result<()> {
        let cache = &self.paths.cache;
        if cache.exists() {
            fs::remove_dir_all(cache).map_err(|e| PvError::Io {
                path: cache.to_path_buf(),
                source: e,
            })?;
            fs::create_dir_all(cache).map_err(|e| PvError::Io {
                path: cache.to_path_buf(),
                source: e,
            })?;
            println!("cache cleaned");
        } else {
            println!("cache already empty");
        }
        Ok(())
    }

    // ── cleanup ──

    /// 清理非活跃版本；`package` = None 清理全部
    pub fn cleanup(&self, package: Option<&str>, dry_run: bool) -> Result<()> {
        if !self.paths.apps.exists() {
            return Ok(());
        }
        for entry in fs::read_dir(&self.paths.apps).map_err(|e| PvError::Io {
            path: self.paths.apps.clone(),
            source: e,
        })? {
            let entry = entry.map_err(|e| PvError::Io {
                path: self.paths.apps.clone(),
                source: e,
            })?;
            let pkg_path = entry.path();
            if !pkg_path.is_dir() {
                continue;
            }
            let pkg_name = entry.file_name().to_string_lossy().to_string();
            if let Some(filter) = package {
                if pkg_name != filter {
                    continue;
                }
            }
            self.cleanup_package(&pkg_name, &pkg_path, dry_run)?;
        }
        Ok(())
    }

    fn cleanup_package(&self, pkg_name: &str, pkg_dir: &Path, dry_run: bool) -> Result<()> {
        let active = self.config.active_version(pkg_name);
        for entry in fs::read_dir(pkg_dir).map_err(|e| PvError::Io {
            path: pkg_dir.to_path_buf(),
            source: e,
        })? {
            let entry = entry.map_err(|e| PvError::Io {
                path: pkg_dir.to_path_buf(),
                source: e,
            })?;
            let ver_path = entry.path();
            let ver_name = entry.file_name().to_string_lossy().to_string();
            if !ver_path.is_dir() || ver_name == "current" {
                continue;
            }
            if active == Some(ver_name.as_str()) {
                continue;
            }
            if dry_run {
                println!("would remove {}@{}", pkg_name, ver_name);
            } else {
                fs::remove_dir_all(&ver_path).map_err(|e| PvError::Io {
                    path: ver_path.clone(),
                    source: e,
                })?;
                println!("removed {}@{}", pkg_name, ver_name);
            }
        }
        Ok(())
    }

    // ── ls-remote ──

    /// 列出包在 bucket 或 npm registry 的所有可用版本
    pub fn ls_remote(&self, spec: &str) -> Result<Vec<String>> {
        // npm: 前缀 → 调 npm view
        if let Some(package) = spec.strip_prefix("npm:") {
            return ls_remote_npm(package);
        }

        // 默认：bucket TOML 清单
        let bucket = BucketManager::new(self.paths.clone(), self.config.clone());
        let mut versions: Vec<String> = Vec::new();
        for path in bucket.manifest_paths_public()? {
            let manifest = Manifest::from_path(&path)?;
            if manifest.name == spec {
                versions.push(manifest.version);
            }
        }
        versions.sort();
        versions.dedup();
        Ok(versions)
    }

    // ── where ──

    /// 显示包的安装路径
    pub fn where_is(&self, package: &str) -> Result<()> {
        let pkg_dir = self.paths.apps.join(package);
        if !pkg_dir.exists() {
            return Err(PvError::PackageNotFound(package.to_string()));
        }

        // 优先活跃版本
        if let Some(active) = self.config.active_version(package) {
            let active_dir = pkg_dir.join(active);
            if active_dir.exists() {
                println!("{}", active_dir.display());
                return Ok(());
            }
        }

        // 回退：找最新安装的版本
        let mut latest: Option<(String, PathBuf)> = None;
        for entry in fs::read_dir(&pkg_dir).map_err(|e| PvError::Io {
            path: pkg_dir.clone(),
            source: e,
        })? {
            let entry = entry.map_err(|e| PvError::Io {
                path: pkg_dir.clone(),
                source: e,
            })?;
            let ver = entry.file_name().to_string_lossy().to_string();
            if ver == "current" || entry.path().is_file() {
                continue;
            }
            match &latest {
                None => latest = Some((ver, entry.path())),
                Some((prev, _)) if &ver > prev => latest = Some((ver, entry.path())),
                _ => {}
            }
        }

        match latest {
            Some((_ver, path)) => println!("{}", path.display()),
            None => return Err(PvError::PackageNotFound(package.to_string())),
        }
        Ok(())
    }
}

// ── 自由辅助函数 ──

fn source_file_name(url: &str) -> String {
    Path::new(url.trim_start_matches("file://"))
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("download.bin")
        .to_string()
}

fn install_manifest(
    manifest: &Manifest,
    source: &Path,
    version_dir: &Path,
    pv_home: &Path,
) -> Result<()> {
    if version_dir.exists() {
        fs::remove_dir_all(version_dir).map_err(|source| PvError::Io {
            path: version_dir.to_path_buf(),
            source,
        })?;
    }
    match manifest.install_type {
        InstallType::Single => SingleInstallStrategy.install(manifest, source, version_dir),
        InstallType::Archive => ArchiveInstallStrategy.install(manifest, source, version_dir),
        InstallType::Installer => InstallerInstallStrategy {
            pv_home: pv_home.to_path_buf(),
        }
        .install(manifest, source, version_dir),
        InstallType::Npm => NpmInstallStrategy.install(manifest, source, version_dir),
    }
}

fn write_installed_manifest(manifest: &Manifest, package_dir: &Path) -> Result<()> {
    fs::create_dir_all(package_dir).map_err(|source| PvError::Io {
        path: package_dir.to_path_buf(),
        source,
    })?;
    fs::write(package_dir.join(".manifest.toml"), manifest.to_toml()?).map_err(|source| {
        PvError::Io {
            path: package_dir.join(".manifest.toml"),
            source,
        }
    })
}

/// npm 版本解析：无约束返回 latest，有约束返回精确匹配
fn resolve_npm_version(package: &str, constraint: Option<&str>) -> Result<String> {
    if let Some(ver) = constraint {
        return Ok(ver.to_string());
    }

    #[cfg(windows)]
    const NPM: &str = "npm.cmd";
    #[cfg(not(windows))]
    const NPM: &str = "npm";

    let npm = find_npm()?;
    let output = Command::new(&npm)
        .arg("view")
        .arg(package)
        .arg("dist-tags")
        .arg("--json")
        .env("NPM_CONFIG_UPDATE_NOTIFIER", "false")
        .output()
        .map_err(|source| PvError::Io {
            path: Path::new(NPM).to_path_buf(),
            source,
        })?;

    if !output.status.success() {
        return Err(PvError::Platform(format!(
            "无法查询 npm 包 {package} 的版本信息，请确认包名正确且网络可用"
        )));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    // 解析 {"latest": "8.57.0", ...}
    let json: serde_json::Value = serde_json::from_str(&stdout)
        .map_err(|_| PvError::Platform(format!("无法解析 npm 返回的 dist-tags: {stdout}")))?;

    json["latest"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| PvError::Platform(format!("npm 包 {package} 没有 latest 标签")))
}

/// 扫描 npm 安装目录下的可执行文件
fn scan_npm_bins(target_dir: &Path) -> Result<Vec<String>> {
    let mut bins = Vec::new();

    // npm 全局安装到 --prefix 后，bin 在 node_modules/.bin/ 下（Unix）
    // 或直接在根目录（Windows npm.cmd 等）
    let bin_dir = target_dir.join("node_modules").join(".bin");
    let scan_dirs = if bin_dir.exists() {
        vec![bin_dir, target_dir.to_path_buf()]
    } else {
        vec![target_dir.to_path_buf()]
    };

    for dir in &scan_dirs {
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let name = entry.file_name();
                let name_str = name.to_string_lossy();
                // 只看可执行文件扩展名
                #[cfg(windows)]
                let is_exe = name_str.ends_with(".exe") || name_str.ends_with(".cmd");
                #[cfg(not(windows))]
                let is_exe = !name_str.contains('.'); // Unix bin 文件通常无扩展名

                if is_exe && !name_str.starts_with('.') {
                    bins.push(name_str.into_owned());
                }
            }
        }
    }

    if bins.is_empty() {
        // fallback：npm 包可能没有 bin 入口
        return Err(PvError::Platform(format!(
            "npm 包安装后未找到可执行文件，目录: {}",
            target_dir.display()
        )));
    }

    Ok(bins)
}

fn find_npm() -> Result<String> {
    #[cfg(windows)]
    const NPM: &str = "npm.cmd";
    #[cfg(not(windows))]
    const NPM: &str = "npm";

    which::which(NPM)
        .map(|p| p.display().to_string())
        .map_err(|_| PvError::Platform(
            "npm 未找到。请先安装 Node.js：\n  pv install node\n\nnpm 后端需要 node + npm 才能安装 npm 包。"
                .to_string(),
        ))
}

// ── cache / cleanup / ls-remote 辅助函数 ──

fn dir_size(path: &Path) -> Result<u64> {
    let mut total = 0u64;
    for entry in walkdir::WalkDir::new(path)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if entry.file_type().is_file() {
            total += entry.metadata().map(|m| m.len()).unwrap_or(0);
        }
    }
    Ok(total)
}

fn format_bytes(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB"];
    let mut size = bytes as f64;
    let mut unit_idx = 0;
    while size >= 1024.0 && unit_idx < UNITS.len() - 1 {
        size /= 1024.0;
        unit_idx += 1;
    }
    format!("{:.1} {}", size, UNITS[unit_idx])
}

fn ls_remote_npm(package: &str) -> Result<Vec<String>> {
    #[cfg(windows)]
    const NPM: &str = "npm.cmd";
    #[cfg(not(windows))]
    const NPM: &str = "npm";

    let npm = which::which(NPM).map_err(|_| {
        PvError::Platform("npm 未找到。请先安装 Node.js：\n  pv install node".to_string())
    })?;

    let output = Command::new(&npm)
        .arg("view")
        .arg(package)
        .arg("versions")
        .arg("--json")
        .env("NPM_CONFIG_UPDATE_NOTIFIER", "false")
        .output()
        .map_err(|e| PvError::Io {
            path: Path::new(NPM).to_path_buf(),
            source: e,
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(PvError::Platform(format!(
            "npm view failed for {package}: {stderr}"
        )));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let versions: Vec<String> = serde_json::from_str(&stdout)
        .map_err(|_| PvError::Platform(format!("无法解析 npm 返回的版本列表")))?;

    Ok(versions)
}

#[derive(Debug, Deserialize)]
struct NpmSearchPackage {
    name: String,
    version: String,
    #[serde(default)]
    description: Option<String>,
}

fn search_npm(keyword: &str) -> Result<Vec<crate::bucket::SearchResult>> {
    let keyword = keyword.trim();
    if keyword.is_empty() {
        return Err(PvError::InvalidPackageSpec("npm:".to_string()));
    }

    #[cfg(windows)]
    const NPM: &str = "npm.cmd";
    #[cfg(not(windows))]
    const NPM: &str = "npm";

    let npm = find_npm()?;
    let output = Command::new(&npm)
        .arg("search")
        .arg(keyword)
        .arg("--json")
        .arg("--searchlimit=20")
        .env("NPM_CONFIG_UPDATE_NOTIFIER", "false")
        .output()
        .map_err(|source| PvError::Io {
            path: Path::new(NPM).to_path_buf(),
            source,
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(PvError::Platform(format!(
            "npm search failed for {keyword}: {stderr}"
        )));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let packages: Vec<NpmSearchPackage> = serde_json::from_str(&stdout)
        .map_err(|_| PvError::Platform(format!("无法解析 npm 返回的搜索结果: {stdout}")))?;

    Ok(packages
        .into_iter()
        .map(|package| crate::bucket::SearchResult {
            name: package.name,
            version: package.version,
            description: package
                .description
                .filter(|description| !description.is_empty()),
            path: PathBuf::from("npm"),
        })
        .collect())
}
