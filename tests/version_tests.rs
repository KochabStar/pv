use std::cell::RefCell;
use std::fs;
use std::path::{Path, PathBuf};

use pv::config::{Config, Paths};
use pv::manifest::Manifest;
use pv::platform::Platform;
use pv::version::VersionManager;
use tempfile::tempdir;

#[derive(Default)]
struct FakePlatform {
    links: RefCell<Vec<(PathBuf, PathBuf)>>,
    shims: RefCell<Vec<String>>,
    path_registrations: RefCell<Vec<PathBuf>>,
}

impl Platform for FakePlatform {
    fn make_active_link(&self, target: &Path, link: &Path) -> pv::error::Result<()> {
        self.links
            .borrow_mut()
            .push((target.to_path_buf(), link.to_path_buf()));
        Ok(())
    }

    fn remove_active_link(&self, _link: &Path) -> pv::error::Result<()> {
        Ok(())
    }

    fn create_shim(
        &self,
        exe_name: &str,
        _shim_exe: &Path,
        _shim_config: &Path,
        _target: &Path,
    ) -> pv::error::Result<()> {
        self.shims.borrow_mut().push(exe_name.to_string());
        Ok(())
    }

    fn remove_shim(&self, _shim_exe: &Path, _shim_config: &Path) -> pv::error::Result<()> {
        Ok(())
    }

    fn register_path(&self, _dir: &Path) -> pv::error::Result<()> {
        self.path_registrations
            .borrow_mut()
            .push(_dir.to_path_buf());
        Ok(())
    }

    fn exe_ext(&self) -> &'static str {
        ".exe"
    }

    fn spawn_shell_with_path(&self, _path_prefix: &Path) -> pv::error::Result<()> {
        Ok(())
    }
}

fn manifest(name: &str, version: &str) -> Manifest {
    toml::from_str(&format!(
        r#"
name = "{name}"
version = "{version}"
type = "single"
bin = ["{name}.exe"]

[architecture.x64]
url = "https://example.invalid/{name}.exe"
hash = "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
"#
    ))
    .expect("manifest")
}

#[test]
fn activates_installed_version_and_creates_shims() {
    let home = tempdir().expect("tempdir");
    let paths = Paths::from_home(home.path());
    let version_dir = paths.apps.join("node").join("20.11.0");
    fs::create_dir_all(&version_dir).expect("version dir");
    let platform = FakePlatform::default();
    let mut config = Config::default();
    let manager = VersionManager::new(paths.clone(), &platform);

    manager
        .activate(&mut config, &manifest("node", "20.11.0"))
        .expect("activate");

    assert_eq!(config.active_version("node"), Some("20.11.0"));
    assert_eq!(
        platform.links.borrow().as_slice(),
        &[(version_dir, paths.apps.join("node").join("current"))]
    );
    assert_eq!(
        platform.shims.borrow().as_slice(),
        &["node.exe".to_string()]
    );
    assert_eq!(
        platform.path_registrations.borrow().as_slice(),
        &[paths.shims]
    );
    assert!(config.path_registered);
}

#[test]
fn skips_path_registration_when_config_already_registered() {
    let home = tempdir().expect("tempdir");
    let paths = Paths::from_home(home.path());
    fs::create_dir_all(paths.apps.join("node").join("20.11.0")).expect("version dir");
    let platform = FakePlatform::default();
    let mut config = Config {
        path_registered: true,
        ..Config::default()
    };
    let manager = VersionManager::new(paths, &platform);

    manager
        .activate(&mut config, &manifest("node", "20.11.0"))
        .expect("activate");

    assert_eq!(config.active_version("node"), Some("20.11.0"));
    assert!(platform.path_registrations.borrow().is_empty());
}

#[test]
fn lists_installed_versions_with_active_marker() {
    let home = tempdir().expect("tempdir");
    let paths = Paths::from_home(home.path());
    fs::create_dir_all(paths.apps.join("node").join("18.20.0")).expect("version dir");
    fs::create_dir_all(paths.apps.join("node").join("20.11.0")).expect("version dir");
    let platform = FakePlatform::default();
    let mut config = Config::default();
    config.set_active_version("node", "20.11.0");
    let manager = VersionManager::new(paths, &platform);

    let listing = manager.list(&config, Some("node")).expect("list");

    assert_eq!(listing.len(), 1);
    assert_eq!(listing[0].versions.len(), 2);
    assert!(listing[0]
        .versions
        .iter()
        .any(|version| version.version == "20.11.0" && version.active));
}
