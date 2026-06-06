use std::path::Path;

use crate::error::Result;

#[cfg(windows)]
pub mod windows;

pub trait Platform {
    fn make_active_link(&self, target: &Path, link: &Path) -> Result<()>;
    fn remove_active_link(&self, link: &Path) -> Result<()>;
    fn create_shim(
        &self,
        exe_name: &str,
        shim_exe: &Path,
        shim_config: &Path,
        target: &Path,
    ) -> Result<()>;
    fn remove_shim(&self, shim_exe: &Path, shim_config: &Path) -> Result<()>;
    fn register_path(&self, dir: &Path) -> Result<()>;
    fn exe_ext(&self) -> &'static str;
    fn spawn_shell_with_path(&self, path_prefix: &Path) -> Result<()>;
}
