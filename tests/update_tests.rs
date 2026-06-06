use pv::update::{compare_installed_to_available, version_gt};

#[test]
fn semver_versions_compare_numerically() {
    assert!(version_gt("20.11.0", "18.20.0"));
    assert!(!version_gt("18.20.0", "20.11.0"));
}

#[test]
fn outdated_package_is_reported_only_when_available_is_newer() {
    let outdated = compare_installed_to_available("node", "18.20.0", "20.11.0").expect("outdated");

    assert_eq!(outdated.package, "node");
    assert_eq!(outdated.installed, "18.20.0");
    assert_eq!(outdated.available, "20.11.0");
    assert!(compare_installed_to_available("node", "20.11.0", "20.11.0").is_none());
}
