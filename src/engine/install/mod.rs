use std::path::Path;

use crate::error::Result;
use crate::manifest::Manifest;

pub mod archive;
pub mod installer;
pub mod npm;
pub mod single;

pub trait InstallStrategy {
    fn install(&self, manifest: &Manifest, source: &Path, target_dir: &Path) -> Result<()>;
}
