use assert_cmd::Command;
use predicates::prelude::*;

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
