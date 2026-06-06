use std::path::PathBuf;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum PvError {
    #[error("package not found: {0}")]
    PackageNotFound(String),
    #[error("version not found for {package}: {version}")]
    VersionNotFound { package: String, version: String },
    #[error("invalid package spec: {0}")]
    InvalidPackageSpec(String),
    #[error("manifest parse failed for {path}: {source}")]
    ManifestParse {
        path: PathBuf,
        source: toml::de::Error,
    },
    #[error("manifest validation failed for {path}: {message}")]
    ManifestValidation { path: PathBuf, message: String },
    #[error("download failed: {0}")]
    Download(String),
    #[error("checksum mismatch for {path}: expected {expected}, actual {actual}")]
    ChecksumMismatch {
        path: PathBuf,
        expected: String,
        actual: String,
    },
    #[error("extract failed for {path}: {message}")]
    Extract { path: PathBuf, message: String },
    #[error("platform operation failed: {0}")]
    Platform(String),
    #[error("unsupported platform operation: {0}")]
    UnsupportedPlatform(String),
    #[error("external command failed: {program} {args:?}, status: {status}")]
    CommandFailed {
        program: String,
        args: Vec<String>,
        status: String,
    },
    #[error("io failed for {path}: {source}")]
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
}

pub type Result<T> = std::result::Result<T, PvError>;
