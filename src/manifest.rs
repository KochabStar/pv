use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::{PvError, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum InstallType {
    Archive,
    Single,
    Installer,
    /// npm registry 包，通过 npm CLI 安装，无需 url/hash/architecture
    #[serde(rename = "npm")]
    Npm,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArchitectureManifest {
    pub url: String,
    pub hash: String,
    #[serde(default)]
    pub extract_dir: Option<String>,
    #[serde(default)]
    pub bin: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Architecture {
    pub x64: ArchitectureManifest,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
    pub install_type: InstallType,
    #[serde(default)]
    pub installer: Option<String>,
    #[serde(default)]
    pub bin: Vec<String>,
    pub architecture: Architecture,
    #[serde(default)]
    pub env: BTreeMap<String, String>,
}

impl Manifest {
    pub fn from_path(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let text = fs::read_to_string(path).map_err(|source| PvError::Io {
            path: path.to_path_buf(),
            source,
        })?;
        let manifest: Self = toml::from_str(&text).map_err(|source| PvError::ManifestParse {
            path: path.to_path_buf(),
            source: Box::new(source),
        })?;
        manifest.validate(path)?;
        Ok(manifest)
    }

    pub fn validate(&self, path: impl Into<PathBuf>) -> Result<()> {
        let path = path.into();
        if self.name.trim().is_empty() {
            return Err(PvError::ManifestValidation {
                path,
                message: "name is required".to_string(),
            });
        }
        if self.version.trim().is_empty() {
            return Err(PvError::ManifestValidation {
                path,
                message: "version is required".to_string(),
            });
        }
        // Npm 类型不需要 url/hash/architecture 字段
        if self.install_type != InstallType::Npm {
            if self.architecture.x64.url.trim().is_empty() {
                return Err(PvError::ManifestValidation {
                    path,
                    message: "architecture.x64.url is required".to_string(),
                });
            }
            if self.architecture.x64.hash.trim().is_empty() {
                return Err(PvError::ManifestValidation {
                    path,
                    message: "architecture.x64.hash is required".to_string(),
                });
            }
        }
        if self.effective_bins().is_empty() {
            return Err(PvError::ManifestValidation {
                path,
                message: "bin is required".to_string(),
            });
        }
        if self.install_type == InstallType::Installer && self.installer.is_none() {
            return Err(PvError::ManifestValidation {
                path,
                message: "installer type is required for installer packages".to_string(),
            });
        }
        Ok(())
    }

    pub fn x64(&self) -> Option<&ArchitectureManifest> {
        Some(&self.architecture.x64)
    }

    pub fn effective_bins(&self) -> &Vec<String> {
        if self.architecture.x64.bin.is_empty() {
            &self.bin
        } else {
            &self.architecture.x64.bin
        }
    }

    pub fn to_toml(&self) -> Result<String> {
        toml::to_string_pretty(self)
            .map_err(|error| PvError::Platform(format!("manifest serialization failed: {error}")))
    }

    /// 为 npm registry 包构造虚拟 Manifest（无需 TOML 清单文件）
    pub fn npm_virtual(name: &str, version: &str, bins: Vec<String>) -> Self {
        Self {
            name: name.to_string(),
            version: version.to_string(),
            description: None,
            homepage: None,
            license: None,
            install_type: InstallType::Npm,
            installer: None,
            bin: bins,
            architecture: Architecture {
                x64: ArchitectureManifest {
                    url: String::new(),
                    hash: String::new(),
                    extract_dir: None,
                    bin: Vec::new(),
                },
            },
            env: BTreeMap::new(),
        }
    }
}
