use std::cell::RefCell;
use std::fs;
use std::path::{Path, PathBuf};

use pv::config::{Config, Paths};
use pv::engine::Engine;
use pv::platform::Platform;
use tempfile::tempdir;

#[derive(Default)]
struct FakePlatform {
    path_prefixes: RefCell<Vec<PathBuf>>,
}

impl Platform for FakePlatform {
    fn make_active_link(&self, _target: &Path, _link: &Path) -> pv::error::Result<()> {
        Ok(())
    }

    fn remove_active_link(&self, _link: &Path) -> pv::error::Result<()> {
        Ok(())
    }

    fn create_shim(
        &self,
        _exe_name: &str,
        _shim_exe: &Path,
        _shim_config: &Path,
        _target: &Path,
    ) -> pv::error::Result<()> {
        Ok(())
    }

    fn remove_shim(&self, _shim_exe: &Path, _shim_config: &Path) -> pv::error::Result<()> {
        Ok(())
    }

    fn register_path(&self, _dir: &Path) -> pv::error::Result<()> {
        Ok(())
    }

    fn exe_ext(&self) -> &'static str {
        ".exe"
    }

    fn spawn_shell_with_path(&self, path_prefix: &Path) -> pv::error::Result<()> {
        self.path_prefixes
            .borrow_mut()
            .push(path_prefix.to_path_buf());
        Ok(())
    }
}

#[test]
fn shell_uses_requested_version_directory_as_path_prefix() {
    let home = tempdir().expect("tempdir");
    let paths = Paths::from_home(home.path());
    fs::create_dir_all(paths.apps.join("node").join("20.11.0")).expect("version dir");
    let platform = FakePlatform::default();
    let engine = Engine::new(paths.clone(), Config::default(), &platform);

    engine.shell("node@20.11.0").expect("shell");

    assert_eq!(
        platform.path_prefixes.borrow().as_slice(),
        &[paths.apps.join("node").join("20.11.0")]
    );
}
