use std::fs;

use pv::engine::install::installer::{
    build_inno_extract_command, build_msi_extract_command, ensure_innounp, INNOUNP_SHA256,
    INNOUNP_URL,
};
use tempfile::tempdir;

#[test]
fn builds_msi_administrative_extract_command() {
    let command = build_msi_extract_command("setup.msi", "C:/pv/apps/tool/1.0.0");

    assert_eq!(command.program, "msiexec");
    assert_eq!(
        command.args,
        vec!["/a", "setup.msi", "/qn", "TARGETDIR=C:/pv/apps/tool/1.0.0"]
    );
}

#[test]
fn builds_inno_extract_command() {
    let command = build_inno_extract_command(
        "C:/pv/tools/innounp/innounp.exe",
        "setup.exe",
        "C:/pv/apps/tool/1.0.0",
    );

    assert_eq!(command.program, "C:/pv/tools/innounp/innounp.exe");
    assert_eq!(
        command.args,
        vec!["-x", "-dC:/pv/apps/tool/1.0.0", "setup.exe"]
    );
}

#[test]
fn reuses_local_innounp_when_present() {
    let home = tempdir().expect("tempdir");
    let tool_dir = home.path().join("tools").join("innounp");
    fs::create_dir_all(&tool_dir).expect("tool dir");
    let exe = tool_dir.join("innounp.exe");
    fs::write(&exe, b"local").expect("write tool");

    assert_eq!(ensure_innounp(home.path()).expect("innounp"), exe);
}

#[test]
fn pins_upstream_innounp_archive() {
    assert!(INNOUNP_URL.contains("innounp050.rar"));
    assert_eq!(
        INNOUNP_SHA256,
        "sha256:1d8837540ccc15d98245a1c73fd08f404b2a7bdfe7dc9bed2fdece818ff6df67"
    );
}
