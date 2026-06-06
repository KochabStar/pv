use std::fs;

use pv::manifest::{InstallType, Manifest};
use tempfile::tempdir;

#[test]
fn parses_top_level_bin_manifest() {
    let dir = tempdir().expect("tempdir");
    let path = dir.path().join("ripgrep.toml");
    fs::write(
        &path,
        r#"
name = "ripgrep"
version = "14.1.0"
description = "fast search"
type = "single"
bin = ["rg.exe"]

[architecture.x64]
url = "https://example.invalid/rg.exe"
hash = "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
"#,
    )
    .expect("write manifest");

    let manifest = Manifest::from_path(&path).expect("parse manifest");
    let arch = manifest.x64().expect("x64 arch");

    assert_eq!(manifest.name, "ripgrep");
    assert_eq!(manifest.install_type, InstallType::Single);
    assert_eq!(manifest.effective_bins(), &["rg.exe".to_string()]);
    assert_eq!(arch.url, "https://example.invalid/rg.exe");
}

#[test]
fn architecture_bin_overrides_top_level_bin() {
    let dir = tempdir().expect("tempdir");
    let path = dir.path().join("node.toml");
    fs::write(
        &path,
        r#"
name = "node"
version = "20.11.0"
type = "archive"
bin = ["node.cmd"]

[architecture.x64]
url = "https://example.invalid/node.zip"
hash = "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
bin = ["node.exe", "npm.cmd"]
"#,
    )
    .expect("write manifest");

    let manifest = Manifest::from_path(&path).expect("parse manifest");

    assert_eq!(
        manifest.effective_bins(),
        &["node.exe".to_string(), "npm.cmd".to_string()]
    );
}

#[test]
fn validates_required_fields() {
    let dir = tempdir().expect("tempdir");
    let path = dir.path().join("bad.toml");
    fs::write(
        &path,
        r#"
name = "bad"
version = "1.0.0"
type = "single"

[architecture.x64]
url = "https://example.invalid/bad.exe"
"#,
    )
    .expect("write manifest");

    assert!(Manifest::from_path(&path).is_err());
}
