use semver::Version;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutdatedPackage {
    pub package: String,
    pub installed: String,
    pub available: String,
}

pub fn compare_installed_to_available(
    package: &str,
    installed: &str,
    available: &str,
) -> Option<OutdatedPackage> {
    if version_gt(available, installed) {
        Some(OutdatedPackage {
            package: package.to_string(),
            installed: installed.to_string(),
            available: available.to_string(),
        })
    } else {
        None
    }
}

pub fn version_gt(left: &str, right: &str) -> bool {
    match (Version::parse(left), Version::parse(right)) {
        (Ok(left), Ok(right)) => left > right,
        _ => left > right,
    }
}
