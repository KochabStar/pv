use assert_cmd::Command;
use predicates::prelude::*;

use std::fs;
use std::path::Path;

#[test]
fn help_mentions_core_commands() {
    let mut cmd = Command::cargo_bin("pv").expect("pv binary is built");

    cmd.arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("install"))
        .stdout(predicate::str::contains("bucket"))
        .stdout(predicate::str::contains("outdated"));
}

#[test]
fn version_command_prints_binary_name() {
    let mut cmd = Command::cargo_bin("pv").expect("pv binary is built");

    cmd.arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("pv"));
}

#[test]
fn search_requires_keyword() {
    let mut cmd = Command::cargo_bin("pv").expect("pv binary is built");

    cmd.arg("search")
        .assert()
        .failure()
        .stderr(predicate::str::contains("required"));
}

#[test]
fn search_prints_message_when_no_results_match() {
    let home = tempfile::tempdir().expect("tempdir");

    let mut cmd = Command::cargo_bin("pv").expect("pv binary is built");
    cmd.env("PV_HOME", home.path())
        .arg("search")
        .arg("definitely-missing")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "No packages found for definitely-missing",
        ));
}

#[test]
fn search_npm_prefix_queries_npm_registry() {
    let home = tempfile::tempdir().expect("tempdir");
    let npm_dir = tempfile::tempdir().expect("npm dir");
    write_fake_npm(npm_dir.path());

    let previous_path = std::env::var_os("PATH").unwrap_or_default();
    let path = std::env::join_paths(
        std::iter::once(npm_dir.path().to_path_buf()).chain(std::env::split_paths(&previous_path)),
    )
    .expect("join PATH");

    let mut cmd = Command::cargo_bin("pv").expect("pv binary is built");
    cmd.env("PV_HOME", home.path())
        .env("PATH", path)
        .arg("search")
        .arg("npm:codex")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "@openai/codex  0.137.0  Codex CLI fixture",
        ));
}

#[test]
fn search_formats_results_as_readable_table() {
    let home = tempfile::tempdir().expect("tempdir");
    let npm_dir = tempfile::tempdir().expect("npm dir");
    write_fake_npm(npm_dir.path());

    let previous_path = std::env::var_os("PATH").unwrap_or_default();
    let path = std::env::join_paths(
        std::iter::once(npm_dir.path().to_path_buf()).chain(std::env::split_paths(&previous_path)),
    )
    .expect("join PATH");

    let mut cmd = Command::cargo_bin("pv").expect("pv binary is built");
    cmd.env("PV_HOME", home.path())
        .env("PATH", path)
        .arg("search")
        .arg("npm:codex")
        .assert()
        .success()
        .stdout(predicate::str::contains("Package"))
        .stdout(predicate::str::contains("Version"))
        .stdout(predicate::str::contains("Description"))
        .stdout(predicate::str::contains(
            "@openai/codex  0.137.0  Codex CLI fixture",
        ))
        .stdout(predicate::str::contains("npm i -g noisy-package").not())
        .stdout(predicate::str::contains("img.shields.io").not());
}

#[test]
#[cfg(windows)]
fn install_output_mentions_resolved_package_version() {
    let home = temp_home_with_ripgrep_manifest();

    let mut cmd = Command::cargo_bin("pv").expect("pv binary is built");
    cmd.env("PV_HOME", home.path())
        .arg("install")
        .arg("ripgrep")
        .assert()
        .success()
        .stdout(predicate::str::contains("Installing ripgrep@14.1.0"))
        .stdout(predicate::str::contains("Installed ripgrep@14.1.0"));
}

#[test]
#[cfg(not(windows))]
fn install_reports_unsupported_platform_on_non_windows() {
    let home = temp_home_with_ripgrep_manifest();

    let mut cmd = Command::cargo_bin("pv").expect("pv binary is built");
    cmd.env("PV_HOME", home.path())
        .arg("install")
        .arg("ripgrep")
        .assert()
        .failure()
        .stdout(predicate::str::contains("Installing ripgrep@14.1.0"))
        .stderr(predicate::str::contains("Windows MVP only"));
}

fn temp_home_with_ripgrep_manifest() -> tempfile::TempDir {
    let home = tempfile::tempdir().expect("tempdir");
    let paths = pv::config::Paths::from_home(home.path());
    let bucket = paths.buckets.join("main");
    fs::create_dir_all(&bucket).expect("bucket");
    let source = home.path().join("ripgrep.exe");
    fs::write(&source, b"rg").expect("source");
    write_single_manifest(
        &bucket.join("ripgrep.toml"),
        "ripgrep",
        "14.1.0",
        &source,
        "5a5000a3f3ff868603876ea774d1d272912a4949e738597a944cfc6050b8d097",
    );
    let mut config = pv::config::Config::default();
    config.add_bucket("main", "local").expect("add bucket");
    config.path_registered = true;
    config.save(&paths).expect("save config");

    home
}

fn write_single_manifest(path: &Path, name: &str, version: &str, source: &Path, hash: &str) {
    let source_url = source.to_string_lossy().replace('\\', "/");
    fs::write(
        path,
        format!(
            r#"
name = "{name}"
version = "{version}"
description = "{name} fixture"
type = "single"
bin = ["{name}.exe"]

[architecture.x64]
url = "file://{source_url}"
hash = "sha256:{hash}"
"#,
        ),
    )
    .expect("write manifest");
}

fn write_fake_npm(dir: &Path) {
    #[cfg(windows)]
    fs::write(
        dir.join("npm.cmd"),
        r#"@echo off
echo [{"name":"@openai/codex","version":"0.137.0","description":"Codex CLI fixture"},{"name":"noisy-package","version":"1.0.0","description":"^<p^>^<code^>npm i -g noisy-package^</code^>^</p^>"},{"name":"badge-package","version":"2.0.0","description":"[![npm version](https://img.shields.io/npm/v/pkg)](https://npmjs.com/package/pkg) Useful package"}]
"#,
    )
    .expect("write fake npm.cmd");

    #[cfg(not(windows))]
    {
        use std::os::unix::fs::PermissionsExt;

        let npm = dir.join("npm");
        fs::write(
            &npm,
            r#"#!/bin/sh
printf '%s\n' '[{"name":"@openai/codex","version":"0.137.0","description":"Codex CLI fixture"},{"name":"noisy-package","version":"1.0.0","description":"<p><code>npm i -g noisy-package</code></p>"},{"name":"badge-package","version":"2.0.0","description":"[![npm version](https://img.shields.io/npm/v/pkg)](https://npmjs.com/package/pkg) Useful package"}]'
"#,
        )
        .expect("write fake npm");

        let mut permissions = fs::metadata(&npm).expect("fake npm metadata").permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&npm, permissions).expect("chmod fake npm");
    }
}
