use pv::package::{ExactPackageSpec, PackageSpec};

#[test]
fn parses_name_without_version() {
    let spec = PackageSpec::parse("node").expect("valid package spec");

    assert_eq!(spec.name, "node");
    assert_eq!(spec.version.as_deref(), None);
}

#[test]
fn parses_name_with_version() {
    let spec = PackageSpec::parse("node@20.11.0").expect("valid package spec");

    assert_eq!(spec.name, "node");
    assert_eq!(spec.version.as_deref(), Some("20.11.0"));
}

#[test]
fn exact_spec_requires_version() {
    let spec = ExactPackageSpec::parse("node@20.11.0").expect("valid exact spec");

    assert_eq!(spec.name, "node");
    assert_eq!(spec.version, "20.11.0");
    assert!(ExactPackageSpec::parse("node").is_err());
}

#[test]
fn rejects_empty_names_and_versions() {
    assert!(PackageSpec::parse("").is_err());
    assert!(PackageSpec::parse("@20.11.0").is_err());
    assert!(PackageSpec::parse("node@").is_err());
    assert!(PackageSpec::parse("node@20@extra").is_err());
}
