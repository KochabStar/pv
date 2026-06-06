use pv::config::{BucketConfig, Config, Paths};
use tempfile::tempdir;

#[test]
fn builds_paths_from_home() {
    let home = tempdir().expect("tempdir");
    let paths = Paths::from_home(home.path());

    assert_eq!(paths.home, home.path());
    assert_eq!(paths.config_file, home.path().join("config.toml"));
    assert_eq!(paths.buckets, home.path().join("buckets"));
    assert_eq!(paths.apps, home.path().join("apps"));
    assert_eq!(paths.shims, home.path().join("shims"));
    assert_eq!(paths.cache, home.path().join("cache"));
    assert_eq!(paths.tools, home.path().join("tools"));
}

#[test]
fn creates_directories_and_saves_config() {
    let home = tempdir().expect("tempdir");
    let paths = Paths::from_home(home.path());

    let mut config = Config::load_or_default(&paths).expect("load config");
    config
        .add_bucket("main", "https://example.invalid/main.git")
        .expect("add bucket");
    config.set_active_version("node", "20.11.0");
    config.save(&paths).expect("save config");

    let loaded = Config::load_or_default(&paths).expect("reload config");

    assert_eq!(loaded.buckets[0].name, "main");
    assert_eq!(loaded.active_version("node"), Some("20.11.0"));
    assert!(paths.buckets.exists());
    assert!(paths.apps.exists());
    assert!(paths.shims.exists());
    assert!(paths.cache.exists());
    assert!(paths.tools.exists());
}

#[test]
fn rejects_duplicate_bucket_names() {
    let mut config = Config::default();

    config
        .add_bucket("main", "https://example.invalid/main.git")
        .expect("first add");

    assert!(config
        .add_bucket("main", "https://example.invalid/other.git")
        .is_err());
}

#[test]
fn removes_bucket_by_name() {
    let mut config = Config {
        buckets: vec![BucketConfig {
            name: "main".to_string(),
            url: "https://example.invalid/main.git".to_string(),
        }],
        active_versions: Default::default(),
        path_registered: false,
    };

    config.remove_bucket("main").expect("remove bucket");

    assert!(config.buckets.is_empty());
}
