use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

use unrar_ng::Archive;

use crate::engine::download::verify_sha256;
use crate::engine::install::InstallStrategy;
use crate::error::{PvError, Result};
use crate::manifest::Manifest;

pub const INNOUNP_URL: &str =
    "https://downloads.sourceforge.net/project/innounp/innounp/innounp%200.50/innounp050.rar";
pub const INNOUNP_SHA256: &str =
    "sha256:1d8837540ccc15d98245a1c73fd08f404b2a7bdfe7dc9bed2fdece818ff6df67";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExternalCommand {
    pub program: String,
    pub args: Vec<String>,
}

pub struct InstallerInstallStrategy {
    pub pv_home: PathBuf,
}

impl InstallStrategy for InstallerInstallStrategy {
    fn install(&self, manifest: &Manifest, source: &Path, target_dir: &Path) -> Result<()> {
        fs::create_dir_all(target_dir).map_err(|source| PvError::Io {
            path: target_dir.to_path_buf(),
            source,
        })?;
        let installer = manifest.installer.as_deref().unwrap_or_default();
        let command = match installer {
            "msi" => {
                build_msi_extract_command(&source.to_string_lossy(), &target_dir.to_string_lossy())
            }
            "inno" => {
                let innounp = ensure_innounp(&self.pv_home)?;
                build_inno_extract_command(
                    &innounp.to_string_lossy(),
                    &source.to_string_lossy(),
                    &target_dir.to_string_lossy(),
                )
            }
            other => {
                return Err(PvError::Extract {
                    path: source.to_path_buf(),
                    message: format!("unsupported installer type: {other}"),
                })
            }
        };
        run_external(command)
    }
}

pub fn build_msi_extract_command(source: &str, target_dir: &str) -> ExternalCommand {
    ExternalCommand {
        program: "msiexec".to_string(),
        args: vec![
            "/a".to_string(),
            source.to_string(),
            "/qn".to_string(),
            format!("TARGETDIR={target_dir}"),
        ],
    }
}

pub fn build_inno_extract_command(
    innounp: &str,
    source: &str,
    target_dir: &str,
) -> ExternalCommand {
    ExternalCommand {
        program: innounp.to_string(),
        args: vec![
            "-x".to_string(),
            format!("-d{target_dir}"),
            source.to_string(),
        ],
    }
}

pub fn ensure_innounp(pv_home: &Path) -> Result<PathBuf> {
    let exe = pv_home.join("tools").join("innounp").join("innounp.exe");
    if exe.exists() {
        return Ok(exe);
    }

    let cache_dir = pv_home.join("cache");
    fs::create_dir_all(&cache_dir).map_err(|source| PvError::Io {
        path: cache_dir.clone(),
        source,
    })?;
    let archive_path = cache_dir.join("innounp050.rar");
    download_blocking(INNOUNP_URL, &archive_path)?;
    verify_sha256(&archive_path, INNOUNP_SHA256)?;
    extract_innounp_archive(&archive_path, pv_home, &exe)?;
    Ok(exe)
}

fn run_external(command: ExternalCommand) -> Result<()> {
    let status = Command::new(&command.program)
        .args(&command.args)
        .status()
        .map_err(|source| PvError::Io {
            path: PathBuf::from(&command.program),
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

fn download_blocking(url: &str, target: &Path) -> Result<()> {
    let mut response =
        reqwest::blocking::get(url).map_err(|error| PvError::Download(error.to_string()))?;
    if !response.status().is_success() {
        return Err(PvError::Download(format!(
            "{url} returned {}",
            response.status()
        )));
    }

    let mut file = fs::File::create(target).map_err(|source| PvError::Io {
        path: target.to_path_buf(),
        source,
    })?;
    response.copy_to(&mut file).map_err(|error| {
        PvError::Download(format!("failed to write {}: {error}", target.display()))
    })?;
    file.flush().map_err(|source| PvError::Io {
        path: target.to_path_buf(),
        source,
    })
}

fn extract_innounp_archive(archive_path: &Path, pv_home: &Path, exe: &Path) -> Result<()> {
    let tools_dir = pv_home.join("tools");
    let tool_dir = tools_dir.join("innounp");
    let extract_dir = tools_dir.join("innounp-extract");

    if extract_dir.exists() {
        fs::remove_dir_all(&extract_dir).map_err(|source| PvError::Io {
            path: extract_dir.clone(),
            source,
        })?;
    }
    fs::create_dir_all(&tools_dir).map_err(|source| PvError::Io {
        path: tools_dir.clone(),
        source,
    })?;

    Archive::new(archive_path)
        .open_for_processing()
        .map_err(|error| PvError::Extract {
            path: archive_path.to_path_buf(),
            message: error.to_string(),
        })?
        .extract_all(&extract_dir)
        .map_err(|error| PvError::Extract {
            path: archive_path.to_path_buf(),
            message: error.to_string(),
        })?;

    let extracted_exe =
        find_file_named(&extract_dir, "innounp.exe")?.ok_or_else(|| PvError::Extract {
            path: archive_path.to_path_buf(),
            message: "innounp.exe not found in archive".to_string(),
        })?;

    fs::create_dir_all(&tool_dir).map_err(|source| PvError::Io {
        path: tool_dir.clone(),
        source,
    })?;
    fs::copy(&extracted_exe, exe).map_err(|source| PvError::Io {
        path: exe.to_path_buf(),
        source,
    })?;
    fs::remove_dir_all(&extract_dir).map_err(|source| PvError::Io {
        path: extract_dir,
        source,
    })
}

fn find_file_named(root: &Path, name: &str) -> Result<Option<PathBuf>> {
    for entry in fs::read_dir(root).map_err(|source| PvError::Io {
        path: root.to_path_buf(),
        source,
    })? {
        let entry = entry.map_err(|source| PvError::Io {
            path: root.to_path_buf(),
            source,
        })?;
        let path = entry.path();
        if path.is_dir() {
            if let Some(found) = find_file_named(&path, name)? {
                return Ok(Some(found));
            }
        } else if path
            .file_name()
            .and_then(|file_name| file_name.to_str())
            .is_some_and(|file_name| file_name.eq_ignore_ascii_case(name))
        {
            return Ok(Some(path));
        }
    }
    Ok(None)
}
