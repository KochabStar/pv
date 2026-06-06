use std::fs;
use std::path::Path;

use crate::engine::install::InstallStrategy;
use crate::error::{PvError, Result};
use crate::manifest::Manifest;

pub struct SingleInstallStrategy;

impl InstallStrategy for SingleInstallStrategy {
    fn install(&self, manifest: &Manifest, source: &Path, target_dir: &Path) -> Result<()> {
        fs::create_dir_all(target_dir).map_err(|source| PvError::Io {
            path: target_dir.to_path_buf(),
            source,
        })?;
        let file_name = manifest
            .effective_bins()
            .first()
            .map(String::as_str)
            .or_else(|| source.file_name().and_then(|name| name.to_str()))
            .ok_or_else(|| PvError::Extract {
                path: source.to_path_buf(),
                message: "unable to determine single-file target name".to_string(),
            })?;
        fs::copy(source, target_dir.join(file_name)).map_err(|source| PvError::Io {
            path: target_dir.join(file_name),
            source,
        })?;
        Ok(())
    }
}
