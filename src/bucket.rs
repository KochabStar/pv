use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::config::{Config, Paths};
use crate::error::{PvError, Result};
use crate::git::resolve_git_from_env;
use crate::manifest::Manifest;
use crate::update::version_gt;

#[derive(Debug, Clone)]
pub struct SearchResult {
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub path: PathBuf,
}

pub struct BucketManager {
    paths: Paths,
    config: Config,
}

impl BucketManager {
    pub fn new(paths: Paths, config: Config) -> Self {
        Self { paths, config }
    }

    pub fn config(&self) -> &Config {
        &self.config
    }

    pub fn into_config(self) -> Config {
        self.config
    }

    pub fn add_config_record(&mut self, name: &str, url: &str) -> Result<()> {
        self.config.add_bucket(name, url)
    }

    pub fn remove_config_record(&mut self, name: &str) -> Result<()> {
        self.config.remove_bucket(name)
    }

    pub fn add(&mut self, name: &str, url: &str) -> Result<()> {
        self.paths.ensure_dirs()?;
        let bucket_dir = self.paths.buckets.join(name);
        if url == "local" {
            fs::create_dir_all(&bucket_dir).map_err(|source| PvError::Io {
                path: bucket_dir.clone(),
                source,
            })?;
        } else if !bucket_dir.exists() {
            let git = resolve_git_from_env(&self.paths)?;
            let status = Command::new(&git)
                .args(["clone", url])
                .arg(&bucket_dir)
                .status()
                .map_err(|source| PvError::Io {
                    path: bucket_dir.clone(),
                    source,
                })?;
            if !status.success() {
                return Err(PvError::CommandFailed {
                    program: git.display().to_string(),
                    args: vec![
                        "clone".to_string(),
                        url.to_string(),
                        bucket_dir.display().to_string(),
                    ],
                    status: status.to_string(),
                });
            }
        }
        self.add_config_record(name, url)
    }

    pub fn remove(&mut self, name: &str) -> Result<()> {
        let bucket_dir = self.paths.buckets.join(name);
        if bucket_dir.exists() {
            fs::remove_dir_all(&bucket_dir).map_err(|source| PvError::Io {
                path: bucket_dir,
                source,
            })?;
        }
        self.remove_config_record(name)
    }

    pub fn sync(&self) -> Result<()> {
        for bucket in &self.config.buckets {
            let bucket_dir = self.paths.buckets.join(&bucket.name);
            if !bucket_dir.join(".git").exists() {
                continue;
            }
            let git = resolve_git_from_env(&self.paths)?;
            let status = Command::new(&git)
                .arg("-C")
                .arg(&bucket_dir)
                .arg("pull")
                .status()
                .map_err(|source| PvError::Io {
                    path: bucket_dir.clone(),
                    source,
                })?;
            if !status.success() {
                return Err(PvError::CommandFailed {
                    program: git.display().to_string(),
                    args: vec![
                        "-C".to_string(),
                        bucket_dir.display().to_string(),
                        "pull".to_string(),
                    ],
                    status: status.to_string(),
                });
            }
        }
        Ok(())
    }

    pub fn search(&self, keyword: &str) -> Result<Vec<SearchResult>> {
        let needle = keyword.to_lowercase();
        let mut results = Vec::new();
        for path in self.manifest_paths()? {
            let manifest = Manifest::from_path(&path)?;
            let description = manifest.description.clone().unwrap_or_default();
            if manifest.name.to_lowercase().contains(&needle)
                || description.to_lowercase().contains(&needle)
            {
                results.push(SearchResult {
                    name: manifest.name,
                    version: manifest.version,
                    description: if description.is_empty() {
                        None
                    } else {
                        Some(description)
                    },
                    path,
                });
            }
        }
        results.sort_by(|left, right| {
            left.name
                .cmp(&right.name)
                .then(left.version.cmp(&right.version))
        });
        Ok(results)
    }

    pub fn find_manifest(&self, package: &str, version: Option<&str>) -> Result<Manifest> {
        let mut matches = Vec::new();
        for path in self.manifest_paths()? {
            let manifest = Manifest::from_path(&path)?;
            if manifest.name != package {
                continue;
            }
            if let Some(version) = version {
                if manifest.version == version {
                    return Ok(manifest);
                }
            } else {
                matches.push(manifest);
            }
        }

        if version.is_some() {
            return Err(PvError::VersionNotFound {
                package: package.to_string(),
                version: version.unwrap_or_default().to_string(),
            });
        }

        matches
            .into_iter()
            .max_by(|left, right| {
                if version_gt(&left.version, &right.version) {
                    std::cmp::Ordering::Greater
                } else if version_gt(&right.version, &left.version) {
                    std::cmp::Ordering::Less
                } else {
                    left.version.cmp(&right.version)
                }
            })
            .ok_or_else(|| PvError::PackageNotFound(package.to_string()))
    }

    fn manifest_paths(&self) -> Result<Vec<PathBuf>> {
        let mut paths = Vec::new();
        for bucket in &self.config.buckets {
            let root = self.paths.buckets.join(&bucket.name);
            collect_toml_files(&root, &mut paths)?;
        }
        Ok(paths)
    }

    /// 公开版本：供 `ls-remote` 等命令使用
    pub fn manifest_paths_public(&self) -> Result<Vec<PathBuf>> {
        self.manifest_paths()
    }
}

fn collect_toml_files(root: &Path, paths: &mut Vec<PathBuf>) -> Result<()> {
    if !root.exists() {
        return Ok(());
    }

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
            collect_toml_files(&path, paths)?;
        } else if path.extension().and_then(|extension| extension.to_str()) == Some("toml") {
            paths.push(path);
        }
    }
    Ok(())
}
