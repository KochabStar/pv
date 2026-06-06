use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::{PvError, Result};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShimConfig {
    pub target: PathBuf,
}

impl ShimConfig {
    pub fn to_toml(&self) -> Result<String> {
        toml::to_string_pretty(self)
            .map_err(|error| PvError::Platform(format!("shim serialization failed: {error}")))
    }

    pub fn from_toml(input: &str) -> Result<Self> {
        toml::from_str(input).map_err(|source| PvError::ManifestParse {
            path: PathBuf::from("<shim>"),
            source,
        })
    }
}

pub fn shim_exe_path(shims: &Path, exe_name: &str) -> PathBuf {
    let stem = Path::new(exe_name)
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or(exe_name);
    shims.join(format!("{stem}.exe"))
}

pub fn shim_config_path(shims: &Path, exe_name: &str) -> PathBuf {
    let stem = Path::new(exe_name)
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or(exe_name);
    shims.join(format!("{stem}.shim"))
}

pub fn target_path(apps: &Path, package: &str, exe_name: &str) -> PathBuf {
    apps.join(package).join("current").join(exe_name)
}
