use std::fs;
use std::path::Path;
use std::process::Command;

use crate::error::{PvError, Result};
use crate::platform::Platform;
use crate::shim::ShimConfig;

pub struct WindowsPlatform;

impl Platform for WindowsPlatform {
    fn make_active_link(&self, target: &Path, link: &Path) -> Result<()> {
        self.remove_active_link(link)?;
        if let Some(parent) = link.parent() {
            fs::create_dir_all(parent).map_err(|source| PvError::Io {
                path: parent.to_path_buf(),
                source,
            })?;
        }
        junction::create(target, link).map_err(|error| PvError::Platform(error.to_string()))
    }

    fn remove_active_link(&self, link: &Path) -> Result<()> {
        if !link.exists() {
            return Ok(());
        }
        fs::remove_dir(link)
            .or_else(|_| fs::remove_dir_all(link))
            .map_err(|source| PvError::Io {
                path: link.to_path_buf(),
                source,
            })
    }

    fn create_shim(
        &self,
        _exe_name: &str,
        shim_exe: &Path,
        shim_config: &Path,
        target: &Path,
    ) -> Result<()> {
        if let Some(parent) = shim_exe.parent() {
            fs::create_dir_all(parent).map_err(|source| PvError::Io {
                path: parent.to_path_buf(),
                source,
            })?;
        }

        let current_exe = std::env::current_exe().map_err(|source| PvError::Io {
            path: shim_exe.to_path_buf(),
            source,
        })?;
        let shim_source = current_exe.with_file_name("pv-shim.exe");
        fs::copy(&shim_source, shim_exe).map_err(|source| PvError::Io {
            path: shim_exe.to_path_buf(),
            source,
        })?;
        fs::write(
            shim_config,
            ShimConfig {
                target: target.to_path_buf(),
            }
            .to_toml()?,
        )
        .map_err(|source| PvError::Io {
            path: shim_config.to_path_buf(),
            source,
        })
    }

    fn remove_shim(&self, shim_exe: &Path, shim_config: &Path) -> Result<()> {
        remove_file_if_exists(shim_exe)?;
        remove_file_if_exists(shim_config)
    }

    fn register_path(&self, dir: &Path) -> Result<()> {
        let dir_text = dir.to_string_lossy();
        let current = std::env::var("PATH").unwrap_or_default();
        if current
            .split(';')
            .any(|entry| entry.eq_ignore_ascii_case(&dir_text))
        {
            return Ok(());
        }
        let updated = if current.is_empty() {
            dir_text.to_string()
        } else {
            format!("{dir_text};{current}")
        };
        let status = Command::new("setx")
            .args(["PATH", &updated])
            .status()
            .map_err(|source| PvError::Io {
                path: dir.to_path_buf(),
                source,
            })?;
        if status.success() {
            Ok(())
        } else {
            Err(PvError::CommandFailed {
                program: "setx".to_string(),
                args: vec!["PATH".to_string(), updated],
                status: status.to_string(),
            })
        }
    }

    fn exe_ext(&self) -> &'static str {
        ".exe"
    }

    fn spawn_shell_with_path(&self, path_prefix: &Path) -> Result<()> {
        let shell = std::env::var("COMSPEC").unwrap_or_else(|_| "cmd.exe".to_string());
        let mut command = Command::new(shell);
        let path = std::env::var("PATH").unwrap_or_default();
        command.env("PATH", format!("{};{path}", path_prefix.display()));
        let status = command.status().map_err(|source| PvError::Io {
            path: path_prefix.to_path_buf(),
            source,
        })?;
        if status.success() {
            Ok(())
        } else {
            Err(PvError::CommandFailed {
                program: "shell".to_string(),
                args: Vec::new(),
                status: status.to_string(),
            })
        }
    }
}

fn remove_file_if_exists(path: &Path) -> Result<()> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(source) => Err(PvError::Io {
            path: path.to_path_buf(),
            source,
        }),
    }
}
