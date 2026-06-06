use std::fs;
use std::path::Path;

use crate::engine::install::InstallStrategy;
use crate::error::{PvError, Result};
use crate::manifest::Manifest;

pub struct ArchiveInstallStrategy;

impl InstallStrategy for ArchiveInstallStrategy {
    fn install(&self, manifest: &Manifest, source: &Path, target_dir: &Path) -> Result<()> {
        fs::create_dir_all(target_dir).map_err(|source| PvError::Io {
            path: target_dir.to_path_buf(),
            source,
        })?;

        let extension = source
            .extension()
            .and_then(|extension| extension.to_str())
            .unwrap_or_default()
            .to_ascii_lowercase();
        match extension.as_str() {
            "zip" => extract_zip(source, target_dir)?,
            "gz" | "tgz" => extract_tar_gz(source, target_dir)?,
            _ => {
                return Err(PvError::Extract {
                    path: source.to_path_buf(),
                    message: format!("unsupported archive extension: {extension}"),
                })
            }
        }

        if let Some(extract_dir) = &manifest.x64().and_then(|arch| arch.extract_dir.clone()) {
            promote_extract_dir(target_dir, extract_dir)?;
        }
        Ok(())
    }
}

fn extract_zip(source: &Path, target_dir: &Path) -> Result<()> {
    let file = fs::File::open(source).map_err(|source_error| PvError::Io {
        path: source.to_path_buf(),
        source: source_error,
    })?;
    let mut archive = zip::ZipArchive::new(file).map_err(|error| PvError::Extract {
        path: source.to_path_buf(),
        message: error.to_string(),
    })?;
    archive
        .extract(target_dir)
        .map_err(|error| PvError::Extract {
            path: source.to_path_buf(),
            message: error.to_string(),
        })
}

fn extract_tar_gz(source: &Path, target_dir: &Path) -> Result<()> {
    let file = fs::File::open(source).map_err(|source_error| PvError::Io {
        path: source.to_path_buf(),
        source: source_error,
    })?;
    let decoder = flate2::read::GzDecoder::new(file);
    let mut archive = tar::Archive::new(decoder);
    archive
        .unpack(target_dir)
        .map_err(|source_error| PvError::Io {
            path: target_dir.to_path_buf(),
            source: source_error,
        })
}

fn promote_extract_dir(target_dir: &Path, extract_dir: &str) -> Result<()> {
    let inner = target_dir.join(extract_dir);
    if !inner.exists() {
        return Ok(());
    }
    for entry in fs::read_dir(&inner).map_err(|source| PvError::Io {
        path: inner.clone(),
        source,
    })? {
        let entry = entry.map_err(|source| PvError::Io {
            path: inner.clone(),
            source,
        })?;
        let destination = target_dir.join(entry.file_name());
        fs::rename(entry.path(), &destination).map_err(|source| PvError::Io {
            path: destination,
            source,
        })?;
    }
    fs::remove_dir_all(&inner).map_err(|source| PvError::Io {
        path: inner,
        source,
    })
}
