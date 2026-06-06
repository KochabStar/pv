use std::path::PathBuf;

use pv::shim::{shim_config_path, shim_exe_path, target_path, ShimConfig};

#[test]
fn builds_shim_paths() {
    let shims = PathBuf::from("C:/pv/shims");

    assert_eq!(shim_exe_path(&shims, "node.exe"), shims.join("node.exe"));
    assert_eq!(
        shim_config_path(&shims, "node.exe"),
        shims.join("node.shim")
    );
}

#[test]
fn target_path_points_through_current_junction() {
    let apps = PathBuf::from("C:/pv/apps");

    assert_eq!(
        target_path(&apps, "node", "node.exe"),
        apps.join("node").join("current").join("node.exe")
    );
}

#[test]
fn serializes_shim_config() {
    let config = ShimConfig {
        target: PathBuf::from("C:/pv/apps/node/current/node.exe"),
    };

    let text = config.to_toml().expect("serialize");
    let parsed = ShimConfig::from_toml(&text).expect("parse");

    assert_eq!(parsed.target, config.target);
}
