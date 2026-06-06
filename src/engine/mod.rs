use std::fs;
use std::path::Path;

use crate::bucket::BucketManager;
use crate::config::{Config, Paths};
use crate::engine::download::download_to_cache;
use crate::engine::install::archive::ArchiveInstallStrategy;
use crate::engine::install::installer::InstallerInstallStrategy;
use crate::engine::install::single::SingleInstallStrategy;
use crate::engine::install::InstallStrategy;
use crate::error::{PvError, Result};
use crate::manifest::{InstallType, Manifest};
use crate::package::{ExactPackageSpec, PackageSpec};
use crate::platform::Platform;
use crate::version::{PackageListing, VersionManager};

pub mod download;
pub mod install;

pub struct Engine<'a, P: Platform> {
    paths: Paths,
    config: Config,
    platform: &'a P,
}

impl<'a, P: Platform> Engine<'a, P> {
    pub fn new(paths: Paths, config: Config, platform: &'a P) -> Self {
        Self {
            paths,
            config,
            platform,
        }
    }

    pub fn config(&self) -> &Config {
        &self.config
    }

    pub async fn install(&mut self, input: &str) -> Result<()> {
        self.paths.ensure_dirs()?;
        let spec = PackageSpec::parse(input)?;
        let manifest = BucketManager::new(self.paths.clone(), self.config.clone())
            .find_manifest(&spec.name, spec.version.as_deref())?;
        let version_dir = self.paths.apps.join(&manifest.name).join(&manifest.version);

        if !version_dir.exists() {
            let arch = manifest.x64().expect("x64 architecture exists");
            let source_name = source_file_name(&arch.url);
            let source =
                download_to_cache(&arch.url, &self.paths.cache, &source_name, &arch.hash).await?;
            install_manifest(&manifest, &source, &version_dir, &self.paths.home)?;
            write_installed_manifest(&manifest, &self.paths.apps.join(&manifest.name))?;
        }

        VersionManager::new(self.paths.clone(), self.platform)
            .activate(&mut self.config, &manifest)?;
        self.config.save(&self.paths)
    }

    pub fn use_version(&mut self, input: &str) -> Result<()> {
        let spec = ExactPackageSpec::parse(input)?;
        let manifest = self.installed_manifest_or_bucket(&spec.name, Some(&spec.version))?;
        VersionManager::new(self.paths.clone(), self.platform)
            .activate(&mut self.config, &manifest)?;
        self.config.save(&self.paths)
    }

    pub fn uninstall(&mut self, input: &str) -> Result<()> {
        let spec = PackageSpec::parse(input)?;
        VersionManager::new(self.paths.clone(), self.platform).uninstall(
            &mut self.config,
            &spec.name,
            spec.version.as_deref(),
        )?;
        self.config.save(&self.paths)
    }

    pub fn list(&self, package: Option<&str>) -> Result<Vec<PackageListing>> {
        VersionManager::new(self.paths.clone(), self.platform).list(&self.config, package)
    }

    pub fn search(&self, keyword: &str) -> Result<Vec<crate::bucket::SearchResult>> {
        BucketManager::new(self.paths.clone(), self.config.clone()).search(keyword)
    }

    pub fn info(&self, package: &str) -> Result<Manifest> {
        let spec = PackageSpec::parse(package)?;
        BucketManager::new(self.paths.clone(), self.config.clone())
            .find_manifest(&spec.name, spec.version.as_deref())
    }

    pub fn outdated(&self) -> Result<Vec<crate::update::OutdatedPackage>> {
        let bucket = BucketManager::new(self.paths.clone(), self.config.clone());
        let mut results = Vec::new();

        for (package, installed) in &self.config.active_versions {
            let available = bucket.find_manifest(package, None)?;
            if let Some(outdated) = crate::update::compare_installed_to_available(
                package,
                installed,
                &available.version,
            ) {
                results.push(outdated);
            }
        }

        Ok(results)
    }

    pub async fn upgrade(&mut self, package: Option<&str>) -> Result<()> {
        if let Some(package) = package {
            let manifest = BucketManager::new(self.paths.clone(), self.config.clone())
                .find_manifest(package, None)?;
            self.install(&format!("{}@{}", manifest.name, manifest.version))
                .await
        } else {
            let outdated = self.outdated()?;
            for item in outdated {
                self.install(&format!("{}@{}", item.package, item.available))
                    .await?;
            }
            Ok(())
        }
    }

    pub fn shell(&self, input: &str) -> Result<()> {
        let spec = ExactPackageSpec::parse(input)?;
        let version_dir = self.paths.apps.join(&spec.name).join(&spec.version);
        if !version_dir.exists() {
            return Err(PvError::VersionNotFound {
                package: spec.name,
                version: spec.version,
            });
        }
        self.platform.spawn_shell_with_path(&version_dir)
    }

    fn installed_manifest_or_bucket(
        &self,
        package: &str,
        version: Option<&str>,
    ) -> Result<Manifest> {
        let manifest_path = self.paths.apps.join(package).join(".manifest.toml");
        if manifest_path.exists() {
            let manifest = Manifest::from_path(&manifest_path)?;
            if version.is_none() || version == Some(manifest.version.as_str()) {
                return Ok(manifest);
            }
        }
        BucketManager::new(self.paths.clone(), self.config.clone()).find_manifest(package, version)
    }
}

fn source_file_name(url: &str) -> String {
    Path::new(url.trim_start_matches("file://"))
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("download.bin")
        .to_string()
}

fn install_manifest(
    manifest: &Manifest,
    source: &Path,
    version_dir: &Path,
    pv_home: &Path,
) -> Result<()> {
    if version_dir.exists() {
        fs::remove_dir_all(version_dir).map_err(|source| PvError::Io {
            path: version_dir.to_path_buf(),
            source,
        })?;
    }
    match manifest.install_type {
        InstallType::Single => SingleInstallStrategy.install(manifest, source, version_dir),
        InstallType::Archive => ArchiveInstallStrategy.install(manifest, source, version_dir),
        InstallType::Installer => InstallerInstallStrategy {
            pv_home: pv_home.to_path_buf(),
        }
        .install(manifest, source, version_dir),
    }
}

fn write_installed_manifest(manifest: &Manifest, package_dir: &Path) -> Result<()> {
    fs::create_dir_all(package_dir).map_err(|source| PvError::Io {
        path: package_dir.to_path_buf(),
        source,
    })?;
    fs::write(package_dir.join(".manifest.toml"), manifest.to_toml()?).map_err(|source| {
        PvError::Io {
            path: package_dir.join(".manifest.toml"),
            source,
        }
    })
}
