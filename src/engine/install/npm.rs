//! Npm registry 安装策略：通过 npm CLI 安装全局包到目标目录。
//!
//! 不需要下载 URL / hash 校验 —— npm 自身处理包获取和完整性。
//! 需要系统中已安装 node + npm（可先通过 pv 安装 node）。

use std::fs;
use std::path::Path;
use std::process::Command;

use crate::engine::install::InstallStrategy;
use crate::error::{PvError, Result};
use crate::manifest::Manifest;

pub struct NpmInstallStrategy;

impl InstallStrategy for NpmInstallStrategy {
    fn install(&self, manifest: &Manifest, _source: &Path, target_dir: &Path) -> Result<()> {
        if target_dir.exists() {
            fs::remove_dir_all(target_dir).map_err(|source| PvError::Io {
                path: target_dir.to_path_buf(),
                source,
            })?;
        }
        fs::create_dir_all(target_dir).map_err(|source| PvError::Io {
            path: target_dir.to_path_buf(),
            source,
        })?;

        let npm = find_npm()?;
        let package_spec = format!("{}@{}", manifest.name, manifest.version);

        let status = Command::new(&npm)
            .arg("install")
            .arg("-g")
            .arg(&package_spec)
            .arg("--prefix")
            .arg(target_dir)
            .arg("--ignore-scripts") // 安全：不跑 install 脚本
            .env("NPM_CONFIG_UPDATE_NOTIFIER", "false")
            .status()
            .map_err(|source| PvError::Io {
                path: target_dir.to_path_buf(),
                source,
            })?;

        if !status.success() {
            return Err(PvError::CommandFailed {
                program: npm,
                args: vec![
                    "install".to_string(),
                    "-g".to_string(),
                    package_spec,
                    "--prefix".to_string(),
                    target_dir.display().to_string(),
                    "--ignore-scripts".to_string(),
                ],
                status: status.to_string(),
            });
        }

        Ok(())
    }
}

/// 在 PATH 中查找 npm 可执行文件（Windows 上找 npm.cmd）
fn find_npm() -> Result<String> {
    #[cfg(windows)]
    const NPM: &str = "npm.cmd";
    #[cfg(not(windows))]
    const NPM: &str = "npm";

    which::which(NPM)
        .map(|p| p.display().to_string())
        .map_err(|_| PvError::Platform(
            "npm 未找到。请先安装 Node.js：\n  pv install node\n\nnpm 后端需要 node + npm 才能安装 npm 包。".to_string()
        ))
}
