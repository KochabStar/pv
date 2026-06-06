use std::fs;

use pv::bucket::BucketManager;
use pv::config::{BucketConfig, Config, Paths};
use tempfile::tempdir;

fn write_manifest(dir: &std::path::Path, file: &str, name: &str, version: &str, description: &str) {
    fs::write(
        dir.join(file),
        format!(
            r#"
name = "{name}"
version = "{version}"
description = "{description}"
type = "single"
bin = ["{name}.exe"]

[architecture.x64]
url = "https://example.invalid/{name}.exe"
hash = "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
"#
        ),
    )
    .expect("write manifest");
}

#[test]
fn scans_and_searches_bucket_manifests() {
    let home = tempdir().expect("tempdir");
    let paths = Paths::from_home(home.path());
    let bucket = paths.buckets.join("main");
    fs::create_dir_all(&bucket).expect("bucket dir");
    write_manifest(
        &bucket,
        "ripgrep.toml",
        "ripgrep",
        "14.1.0",
        "fast recursive search",
    );
    write_manifest(
        &bucket,
        "node.toml",
        "node",
        "20.11.0",
        "JavaScript runtime",
    );
    let config = Config {
        buckets: vec![BucketConfig {
            name: "main".to_string(),
            url: "local".to_string(),
        }],
        active_versions: Default::default(),
        path_registered: false,
    };
    let manager = BucketManager::new(paths, config);

    let results = manager.search("search").expect("search");

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].name, "ripgrep");
}

#[test]
fn finds_latest_or_requested_manifest() {
    let home = tempdir().expect("tempdir");
    let paths = Paths::from_home(home.path());
    let bucket = paths.buckets.join("main");
    fs::create_dir_all(&bucket).expect("bucket dir");
    write_manifest(&bucket, "node-18.toml", "node", "18.20.0", "runtime");
    write_manifest(&bucket, "node-20.toml", "node", "20.11.0", "runtime");
    let config = Config {
        buckets: vec![BucketConfig {
            name: "main".to_string(),
            url: "local".to_string(),
        }],
        active_versions: Default::default(),
        path_registered: false,
    };
    let manager = BucketManager::new(paths, config);

    assert_eq!(
        manager.find_manifest("node", None).unwrap().version,
        "20.11.0"
    );
    assert_eq!(
        manager
            .find_manifest("node", Some("18.20.0"))
            .unwrap()
            .version,
        "18.20.0"
    );
}

#[test]
fn bucket_manager_adds_config_record_without_duplicate_external_add() {
    let home = tempdir().expect("tempdir");
    let paths = Paths::from_home(home.path());
    let config = Config::default();
    let mut manager = BucketManager::new(paths, config);

    manager
        .add_config_record("main", "https://example.invalid/main.git")
        .expect("add config");

    assert_eq!(manager.config().buckets.len(), 1);
    assert!(manager
        .add_config_record("main", "https://example.invalid/main.git")
        .is_err());
}
