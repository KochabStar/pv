use std::ffi::OsString;

use pv::config::Paths;
use pv::git::{resolve_git, GitSearchConfig};
use tempfile::tempdir;

#[test]
fn resolves_git_from_pv_git_before_packaged_locations() {
    let home = tempdir().expect("tempdir");
    let paths = Paths::from_home(home.path());
    let configured_git = home.path().join("custom").join("git.exe");
    let packaged_git = paths
        .apps
        .join("git")
        .join("current")
        .join("cmd")
        .join("git.exe");
    std::fs::create_dir_all(configured_git.parent().unwrap()).expect("configured git parent");
    std::fs::create_dir_all(packaged_git.parent().unwrap()).expect("packaged git parent");
    std::fs::write(&configured_git, "").expect("configured git");
    std::fs::write(&packaged_git, "").expect("packaged git");

    let found = resolve_git(
        &paths,
        &GitSearchConfig {
            pv_git: Some(configured_git.clone()),
            path: OsString::new(),
        },
    )
    .expect("resolve git");

    assert_eq!(found, configured_git);
}

#[test]
fn resolves_git_from_pv_managed_git_when_pv_git_is_not_set() {
    let home = tempdir().expect("tempdir");
    let paths = Paths::from_home(home.path());
    let packaged_git = paths
        .apps
        .join("git")
        .join("current")
        .join("cmd")
        .join("git.exe");
    std::fs::create_dir_all(packaged_git.parent().unwrap()).expect("packaged git parent");
    std::fs::write(&packaged_git, "").expect("packaged git");

    let found = resolve_git(
        &paths,
        &GitSearchConfig {
            pv_git: None,
            path: OsString::new(),
        },
    )
    .expect("resolve git");

    assert_eq!(found, packaged_git);
}

#[test]
fn missing_git_error_tells_user_how_to_bootstrap() {
    let home = tempdir().expect("tempdir");
    let paths = Paths::from_home(home.path());

    let error = resolve_git(
        &paths,
        &GitSearchConfig {
            pv_git: None,
            path: OsString::new(),
        },
    )
    .expect_err("git should be missing");

    let message = error.to_string();
    assert!(message.contains("Git is required"));
    assert!(message.contains("pv install git"));
}
