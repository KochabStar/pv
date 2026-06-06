use std::cell::RefCell;
use std::fs;
use std::path::{Path, PathBuf};

use pv::config::{BucketConfig, Config, Paths};
use pv::engine::Engine;
use pv::platform::Platform;
use tempfile::tempdir;

#[derive(Default)]
struct FakePlatform {
    links: RefCell<Vec<(PathBuf, PathBuf)>>,
    removed_shims: RefCell<usize>,
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
        _exe_name: &str,
        _shim_exe: &Path,
        _shim_config: &Path,
        _target: &Path,
    ) -> pv::error::Result<()> {
        Ok(())
    }

    fn remove_shim(&self, _shim_exe: &Path, _shim_config: &Path) -> pv::error::Result<()> {
        *self.removed_shims.borrow_mut() += 1;
        Ok(())
    }

    fn register_path(&self, _dir: &Path) -> pv::error::Result<()> {
        Ok(())
    }

    fn exe_ext(&self) -> &'static str {
        ".exe"
    }

    fn spawn_shell_with_path(&self, _path_prefix: &Path) -> pv::error::Result<()> {
        Ok(())
    }
}

fn write_single_manifest(
    path: &Path,
    name: &str,
    version: &str,
    source: &Path,
    hash: &str,
    bin: &str,
) {
    let source_url = source.to_string_lossy().replace('\\', "/");
    fs::write(
        path,
        format!(
            r#"
name = "{name}"
version = "{version}"
description = "{name} fixture"
type = "single"

[architecture.x64]
url = "file://{source_url}"
hash = "sha256:{hash}"
bin = ["{bin}"]
"#,
        ),
    )
    .expect("write manifest");
}

#[tokio::test]
async fn full_single_package_lifecycle() {
    let home = tempdir().expect("tempdir");
    let paths = Paths::from_home(home.path());
    let bucket = paths.buckets.join("main");
    fs::create_dir_all(&bucket).expect("bucket");
    let rg_source = home.path().join("rg.exe");
    fs::write(&rg_source, b"rg").expect("source");
    write_single_manifest(
        &bucket.join("ripgrep.toml"),
        "ripgrep",
        "14.1.0",
        &rg_source,
        "5a5000a3f3ff868603876ea774d1d272912a4949e738597a944cfc6050b8d097",
        "rg.exe",
    );

    let config = Config {
        buckets: vec![BucketConfig {
            name: "main".to_string(),
            url: "local".to_string(),
        }],
        active_versions: Default::default(),
        path_registered: false,
    };
    let platform = FakePlatform::default();
    let mut engine = Engine::new(paths.clone(), config, &platform);

    engine.install("ripgrep").await.expect("install");
    engine.use_version("ripgrep@14.1.0").expect("use");
    let list = engine.list(Some("ripgrep")).expect("list");
    let outdated = engine.outdated().expect("outdated");
    engine.uninstall("ripgrep").expect("uninstall all");

    assert_eq!(list[0].versions[0].version, "14.1.0");
    assert!(outdated.is_empty());
    assert!(!paths.apps.join("ripgrep").exists());
    assert_eq!(*platform.removed_shims.borrow(), 1);
}
