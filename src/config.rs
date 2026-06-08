use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::{PvError, Result};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Paths {
    pub home: PathBuf,
    pub config_file: PathBuf,
    pub buckets: PathBuf,
    pub apps: PathBuf,
    pub shims: PathBuf,
    pub cache: PathBuf,
    pub tools: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BucketConfig {
    pub name: String,
    pub url: String,
}

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

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub buckets: Vec<BucketConfig>,
    pub active_versions: BTreeMap<String, String>,
    #[serde(default)]
    pub path_registered: bool,
    pub download: DownloadConfig,
}

impl Paths {
    pub fn discover() -> Result<Self> {
        if let Ok(home) = std::env::var("PV_HOME") {
            return Ok(Self::from_home(home));
        }

        let user_home = dirs::home_dir().ok_or_else(|| {
            PvError::Platform("unable to determine user home directory".to_string())
        })?;
        Ok(Self::from_home(user_home.join(".pv")))
    }

    pub fn from_home(path: impl AsRef<Path>) -> Self {
        let home = path.as_ref().to_path_buf();
        Self {
            config_file: home.join("config.toml"),
            buckets: home.join("buckets"),
            apps: home.join("apps"),
            shims: home.join("shims"),
            cache: home.join("cache"),
            tools: home.join("tools"),
            home,
        }
    }

    pub fn ensure_dirs(&self) -> Result<()> {
        for path in [
            &self.home,
            &self.buckets,
            &self.apps,
            &self.shims,
            &self.cache,
            &self.tools,
        ] {
            fs::create_dir_all(path).map_err(|source| PvError::Io {
                path: path.to_path_buf(),
                source,
            })?;
        }
        Ok(())
    }
}

impl Config {
    pub fn load_or_default(paths: &Paths) -> Result<Self> {
        paths.ensure_dirs()?;
        if !paths.config_file.exists() {
            return Ok(Self::default());
        }

        let text = fs::read_to_string(&paths.config_file).map_err(|source| PvError::Io {
            path: paths.config_file.clone(),
            source,
        })?;
        toml::from_str(&text).map_err(|source| PvError::ManifestParse {
            path: paths.config_file.clone(),
            source: Box::new(source),
        })
    }

    pub fn save(&self, paths: &Paths) -> Result<()> {
        paths.ensure_dirs()?;
        let text = toml::to_string_pretty(self)
            .map_err(|error| PvError::Platform(format!("config serialization failed: {error}")))?;
        fs::write(&paths.config_file, text).map_err(|source| PvError::Io {
            path: paths.config_file.clone(),
            source,
        })
    }

    pub fn add_bucket(&mut self, name: &str, url: &str) -> Result<()> {
        if self.buckets.iter().any(|bucket| bucket.name == name) {
            return Err(PvError::Platform(format!("bucket already exists: {name}")));
        }

        self.buckets.push(BucketConfig {
            name: name.to_string(),
            url: url.to_string(),
        });
        Ok(())
    }

    pub fn remove_bucket(&mut self, name: &str) -> Result<()> {
        let before = self.buckets.len();
        self.buckets.retain(|bucket| bucket.name != name);
        if self.buckets.len() == before {
            return Err(PvError::Platform(format!("bucket not found: {name}")));
        }
        Ok(())
    }

    pub fn set_active_version(&mut self, package: &str, version: &str) {
        self.active_versions
            .insert(package.to_string(), version.to_string());
    }

    pub fn clear_active_version(&mut self, package: &str) {
        self.active_versions.remove(package);
    }

    pub fn active_version(&self, package: &str) -> Option<&str> {
        self.active_versions.get(package).map(String::as_str)
    }
}
