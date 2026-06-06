use std::fs;
use std::path::PathBuf;

use crate::config::{Config, Paths};
use crate::error::{PvError, Result};
use crate::manifest::Manifest;
use crate::platform::Platform;
use crate::shim::{shim_config_path, shim_exe_path, target_path};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageListing {
    pub package: String,
    pub versions: Vec<VersionListing>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VersionListing {
    pub version: String,
    pub active: bool,
}

pub struct VersionManager<'a, P: Platform> {
    paths: Paths,
    platform: &'a P,
}

impl<'a, P: Platform> VersionManager<'a, P> {
    pub fn new(paths: Paths, platform: &'a P) -> Self {
        Self { paths, platform }
    }

    pub fn activate(&self, config: &mut Config, manifest: &Manifest) -> Result<()> {
        let version_dir = self.paths.apps.join(&manifest.name).join(&manifest.version);
        if !version_dir.exists() {
            return Err(PvError::VersionNotFound {
                package: manifest.name.clone(),
                version: manifest.version.clone(),
            });
        }

        let current = self.paths.apps.join(&manifest.name).join("current");
        self.platform.make_active_link(&version_dir, &current)?;
        for bin in manifest.effective_bins() {
            self.platform.create_shim(
                bin,
                &shim_exe_path(&self.paths.shims, bin),
                &shim_config_path(&self.paths.shims, bin),
                &target_path(&self.paths.apps, &manifest.name, bin),
            )?;
        }
        if !config.path_registered {
            self.platform.register_path(&self.paths.shims)?;
            config.path_registered = true;
        }
        config.set_active_version(&manifest.name, &manifest.version);
        Ok(())
    }

    pub fn list(&self, config: &Config, package: Option<&str>) -> Result<Vec<PackageListing>> {
        let mut listings = Vec::new();
        if let Some(package) = package {
            let package_dir = self.paths.apps.join(package);
            if package_dir.exists() {
                listings.push(self.list_one(config, package.to_string(), package_dir)?);
            }
            return Ok(listings);
        }

        if !self.paths.apps.exists() {
            return Ok(listings);
        }
        for entry in fs::read_dir(&self.paths.apps).map_err(|source| PvError::Io {
            path: self.paths.apps.clone(),
            source,
        })? {
            let entry = entry.map_err(|source| PvError::Io {
                path: self.paths.apps.clone(),
                source,
            })?;
            let path = entry.path();
            if path.is_dir() {
                let package = entry.file_name().to_string_lossy().to_string();
                listings.push(self.list_one(config, package, path)?);
            }
        }
        listings.sort_by(|left, right| left.package.cmp(&right.package));
        Ok(listings)
    }

    pub fn uninstall(
        &self,
        config: &mut Config,
        package: &str,
        version: Option<&str>,
    ) -> Result<()> {
        let package_dir = self.paths.apps.join(package);
        if !package_dir.exists() {
            return Err(PvError::PackageNotFound(package.to_string()));
        }

        if let Some(version) = version {
            let version_dir = package_dir.join(version);
            if !version_dir.exists() {
                return Err(PvError::VersionNotFound {
                    package: package.to_string(),
                    version: version.to_string(),
                });
            }
            fs::remove_dir_all(&version_dir).map_err(|source| PvError::Io {
                path: version_dir,
                source,
            })?;
            if config.active_version(package) == Some(version) {
                config.clear_active_version(package);
                self.platform
                    .remove_active_link(&package_dir.join("current"))?;
            }
            if fs::read_dir(&package_dir)
                .map_err(|source| PvError::Io {
                    path: package_dir.clone(),
                    source,
                })?
                .next()
                .is_none()
            {
                fs::remove_dir_all(&package_dir).map_err(|source| PvError::Io {
                    path: package_dir.clone(),
                    source,
                })?;
            }
            return Ok(());
        }

        self.remove_shims_for_package(package)?;
        self.platform
            .remove_active_link(&package_dir.join("current"))?;
        fs::remove_dir_all(&package_dir).map_err(|source| PvError::Io {
            path: package_dir,
            source,
        })?;
        config.clear_active_version(package);
        Ok(())
    }

    fn list_one(
        &self,
        config: &Config,
        package: String,
        package_dir: PathBuf,
    ) -> Result<PackageListing> {
        let mut versions = Vec::new();
        for entry in fs::read_dir(&package_dir).map_err(|source| PvError::Io {
            path: package_dir.clone(),
            source,
        })? {
            let entry = entry.map_err(|source| PvError::Io {
                path: package_dir.clone(),
                source,
            })?;
            let path = entry.path();
            let version = entry.file_name().to_string_lossy().to_string();
            if !path.is_dir() || version == "current" {
                continue;
            }
            versions.push(VersionListing {
                active: config.active_version(&package) == Some(version.as_str()),
                version,
            });
        }
        versions.sort_by(|left, right| left.version.cmp(&right.version));
        Ok(PackageListing { package, versions })
    }

    fn remove_shims_for_package(&self, package: &str) -> Result<()> {
        let manifest_path = self.paths.apps.join(package).join(".manifest.toml");
        if !manifest_path.exists() {
            return Ok(());
        }
        let manifest = Manifest::from_path(&manifest_path)?;
        for bin in manifest.effective_bins() {
            self.platform.remove_shim(
                &shim_exe_path(&self.paths.shims, bin),
                &shim_config_path(&self.paths.shims, bin),
            )?;
        }
        Ok(())
    }
}
