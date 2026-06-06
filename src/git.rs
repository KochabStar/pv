use std::ffi::OsString;
use std::path::PathBuf;

use crate::config::Paths;
use crate::error::{PvError, Result};

#[derive(Debug, Clone)]
pub struct GitSearchConfig {
    pub pv_git: Option<PathBuf>,
    pub path: OsString,
}

impl GitSearchConfig {
    pub fn from_env() -> Self {
        Self {
            pv_git: std::env::var_os("PV_GIT").map(PathBuf::from),
            path: std::env::var_os("PATH").unwrap_or_default(),
        }
    }
}

pub fn resolve_git_from_env(paths: &Paths) -> Result<PathBuf> {
    resolve_git(paths, &GitSearchConfig::from_env())
}

pub fn resolve_git(paths: &Paths, config: &GitSearchConfig) -> Result<PathBuf> {
    if let Some(path) = &config.pv_git {
        if is_git_candidate(path) {
            return Ok(path.clone());
        }
    }

    for path in packaged_git_candidates(paths) {
        if is_git_candidate(&path) {
            return Ok(path);
        }
    }

    for dir in std::env::split_paths(&config.path) {
        for name in git_executable_names() {
            let path = dir.join(name);
            if is_git_candidate(&path) {
                return Ok(path);
            }
        }
    }

    Err(PvError::Platform(
        "Git is required for bucket operations. Run 'pv install git' and try again.".to_string(),
    ))
}

fn packaged_git_candidates(paths: &Paths) -> Vec<PathBuf> {
    vec![
        paths
            .apps
            .join("git")
            .join("current")
            .join("cmd")
            .join("git.exe"),
        paths
            .apps
            .join("git")
            .join("current")
            .join("bin")
            .join("git.exe"),
        paths.tools.join("mingit").join("cmd").join("git.exe"),
        paths.tools.join("mingit").join("bin").join("git.exe"),
    ]
}

fn git_executable_names() -> &'static [&'static str] {
    if cfg!(windows) {
        &["git.exe", "git.cmd", "git.bat", "git"]
    } else {
        &["git"]
    }
}

fn is_git_candidate(path: &std::path::Path) -> bool {
    path.is_file()
}
