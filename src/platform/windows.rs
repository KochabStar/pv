use std::ffi::OsStr;
use std::fs;
use std::os::windows::ffi::OsStrExt;
use std::path::Path;
use std::process::Command;

use crate::error::{PvError, Result};
use crate::platform::Platform;
use crate::shim::ShimConfig;

pub struct WindowsPlatform;

const USER_ENVIRONMENT_KEY: &str = r"HKCU\Environment";
const HWND_BROADCAST: isize = 0xffff;
const WM_SETTINGCHANGE: u32 = 0x001A;
const SMTO_ABORTIFHUNG: u32 = 0x0002;

#[link(name = "user32")]
unsafe extern "system" {
    fn SendMessageTimeoutW(
        hwnd: isize,
        msg: u32,
        wparam: usize,
        lparam: isize,
        flags: u32,
        timeout: u32,
        result: *mut usize,
    ) -> isize;
}

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
        let current = read_user_path(dir)?;
        let updated = merge_path_entry(&current, &dir_text);
        if updated == current {
            return Ok(());
        }
        write_user_path(&updated, dir)
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

fn read_user_path(context: &Path) -> Result<String> {
    let output = Command::new("reg")
        .args(["query", USER_ENVIRONMENT_KEY, "/v", "Path"])
        .output()
        .map_err(|source| PvError::Io {
            path: context.to_path_buf(),
            source,
        })?;
    if !output.status.success() {
        return Ok(String::new());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(parse_user_path_query(&stdout).unwrap_or_default())
}

fn write_user_path(user_path: &str, context: &Path) -> Result<()> {
    let args = vec![
        "add".to_string(),
        USER_ENVIRONMENT_KEY.to_string(),
        "/v".to_string(),
        "Path".to_string(),
        "/t".to_string(),
        "REG_EXPAND_SZ".to_string(),
        "/d".to_string(),
        user_path.to_string(),
        "/f".to_string(),
    ];
    let status = Command::new("reg")
        .args(&args)
        .status()
        .map_err(|source| PvError::Io {
            path: context.to_path_buf(),
            source,
        })?;
    if status.success() {
        broadcast_environment_change();
        Ok(())
    } else {
        Err(PvError::CommandFailed {
            program: "reg".to_string(),
            args,
            status: status.to_string(),
        })
    }
}

fn broadcast_environment_change() {
    let message = OsStr::new("Environment")
        .encode_wide()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();
    let mut result = 0;
    unsafe {
        let _ = SendMessageTimeoutW(
            HWND_BROADCAST,
            WM_SETTINGCHANGE,
            0,
            message.as_ptr() as isize,
            SMTO_ABORTIFHUNG,
            5000,
            &mut result,
        );
    }
}

fn merge_path_entry(user_path: &str, dir: &str) -> String {
    let dir = dir.trim();
    if dir.is_empty() {
        return user_path.to_string();
    }
    let normalized_dir = normalize_path_entry(dir);
    if user_path
        .split(';')
        .any(|entry| normalize_path_entry(entry).eq_ignore_ascii_case(normalized_dir))
    {
        return user_path.to_string();
    }
    if user_path.trim().is_empty() {
        dir.to_string()
    } else {
        format!("{dir};{user_path}")
    }
}

fn normalize_path_entry(entry: &str) -> &str {
    entry.trim().trim_matches('"')
}

fn parse_user_path_query(output: &str) -> Option<String> {
    for line in output.lines() {
        let trimmed = line.trim_start();
        let mut fields = trimmed.split_whitespace();
        let Some(name) = fields.next() else {
            continue;
        };
        let Some(kind) = fields.next() else {
            continue;
        };
        if name != "Path" || !kind.starts_with("REG_") {
            continue;
        }
        let kind_index = trimmed.find(kind)?;
        return Some(trimmed[kind_index + kind.len()..].trim_start().to_string());
    }
    None
}

#[cfg(test)]
mod tests {
    use super::{merge_path_entry, parse_user_path_query};

    #[test]
    fn merge_path_entry_prepends_missing_dir_without_dropping_existing_entries() {
        let user_path = r"%USERPROFILE%\scoop\shims;C:\Users\BlackHole\.dotnet\tools";
        let updated = merge_path_entry(user_path, r"D:\code\rust\pv\.tmp\pv-real-git\shims");

        assert_eq!(
            updated,
            r"D:\code\rust\pv\.tmp\pv-real-git\shims;%USERPROFILE%\scoop\shims;C:\Users\BlackHole\.dotnet\tools"
        );
    }

    #[test]
    fn merge_path_entry_keeps_existing_dir_case_insensitively() {
        let user_path = r"d:\code\rust\pv\shims;%USERPROFILE%\scoop\shims";
        let updated = merge_path_entry(user_path, r"D:\code\rust\pv\shims");

        assert_eq!(updated, user_path);
    }

    #[test]
    fn parse_user_path_query_preserves_spaces_in_path_entries() {
        let output = r#"
HKEY_CURRENT_USER\Environment
    Path    REG_EXPAND_SZ    %USERPROFILE%\scoop\shims;C:\Program Files\PowerShell\7
"#;

        assert_eq!(
            parse_user_path_query(output).as_deref(),
            Some(r"%USERPROFILE%\scoop\shims;C:\Program Files\PowerShell\7")
        );
    }
}
