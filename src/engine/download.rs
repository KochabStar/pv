use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::Command;

use futures_util::StreamExt;
use sha2::{Digest, Sha256};
use tokio::io::AsyncWriteExt;

use crate::error::{PvError, Result};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DownloadOptions {
    pub aria2_enabled: bool,
    pub aria2_split: u32,
    pub aria2_max_connection_per_server: u32,
    pub aria2_min_split_size: String,
}

impl DownloadOptions {
    pub fn from_config(config: &crate::config::DownloadConfig) -> Self {
        Self {
            aria2_enabled: config.aria2_enabled,
            aria2_split: config.aria2_split,
            aria2_max_connection_per_server: config.aria2_max_connection_per_server,
            aria2_min_split_size: config.aria2_min_split_size.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExternalCommand {
    pub program: String,
    pub args: Vec<String>,
}

pub fn build_aria2_command(
    url: &str,
    cache_dir: &Path,
    file_name: &str,
    options: &DownloadOptions,
) -> ExternalCommand {
    ExternalCommand {
        program: "aria2c".to_string(),
        args: vec![
            "--allow-overwrite=true".to_string(),
            "--auto-file-renaming=false".to_string(),
            "--continue=true".to_string(),
            format!("--dir={}", cache_dir.display()),
            format!("--out={file_name}"),
            format!("--split={}", options.aria2_split),
            format!(
                "--max-connection-per-server={}",
                options.aria2_max_connection_per_server
            ),
            format!("--min-split-size={}", options.aria2_min_split_size),
            url.to_string(),
        ],
    }
}

pub async fn download_to_cache(
    url: &str,
    cache_dir: &Path,
    file_name: &str,
    expected_hash: &str,
    options: &DownloadOptions,
) -> Result<PathBuf> {
    fs::create_dir_all(cache_dir).map_err(|source| PvError::Io {
        path: cache_dir.to_path_buf(),
        source,
    })?;
    let target = cache_dir.join(file_name);

    if let Some(path) = url.strip_prefix("file://") {
        let spinner = crate::terminal::spinner(format!("Copying {file_name}"));
        fs::copy(path, &target).map_err(|source| PvError::Io {
            path: PathBuf::from(path),
            source,
        })?;
        spinner.finish_and_clear();
    } else if options.aria2_enabled && which::which("aria2c").is_ok() {
        download_with_aria2(url, cache_dir, file_name, options)?;
    } else if options.aria2_enabled {
        crate::terminal::status("aria2c not found, falling back to built-in downloader");
        download_http_stream(url, &target, file_name, options).await?;
    } else {
        download_http_stream(url, &target, file_name, options).await?;
    }

    if let Err(error) = verify_sha256(&target, expected_hash) {
        let _ = fs::remove_file(&target);
        return Err(error);
    }
    Ok(target)
}

fn download_with_aria2(
    url: &str,
    cache_dir: &Path,
    file_name: &str,
    options: &DownloadOptions,
) -> Result<()> {
    let command = build_aria2_command(url, cache_dir, file_name, options);
    let status = Command::new(&command.program)
        .args(&command.args)
        .status()
        .map_err(|source| PvError::Io {
            path: PathBuf::from(&command.program),
            source,
        })?;
    if status.success() {
        Ok(())
    } else {
        Err(PvError::CommandFailed {
            program: command.program,
            args: command.args,
            status: status.to_string(),
        })
    }
}

async fn download_http_stream(
    url: &str,
    target: &Path,
    file_name: &str,
    _options: &DownloadOptions,
) -> Result<()> {
    let response = reqwest::get(url)
        .await
        .map_err(|error| PvError::Download(error.to_string()))?;
    if !response.status().is_success() {
        return Err(PvError::Download(format!(
            "{url} returned {}",
            response.status()
        )));
    }

    let progress = if let Some(total) = response.content_length() {
        crate::terminal::download_bar(total, format!("Downloading {file_name}"))
    } else {
        crate::terminal::spinner(format!("Downloading {file_name}"))
    };
    let mut file = tokio::fs::File::create(target)
        .await
        .map_err(|source| PvError::Io {
            path: target.to_path_buf(),
            source,
        })?;
    let mut stream = response.bytes_stream();

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|error| PvError::Download(error.to_string()))?;
        file.write_all(&chunk).await.map_err(|source| PvError::Io {
            path: target.to_path_buf(),
            source,
        })?;
        progress.inc(chunk.len() as u64);
    }
    file.flush().await.map_err(|source| PvError::Io {
        path: target.to_path_buf(),
        source,
    })?;
    progress.finish_and_clear();
    Ok(())
}

pub fn verify_sha256(path: &Path, expected_hash: &str) -> Result<()> {
    let expected = expected_hash
        .strip_prefix("sha256:")
        .unwrap_or(expected_hash)
        .to_lowercase();
    let actual = sha256(path)?;
    if actual == expected {
        Ok(())
    } else {
        Err(PvError::ChecksumMismatch {
            path: path.to_path_buf(),
            expected,
            actual,
        })
    }
}

fn sha256(path: &Path) -> Result<String> {
    let mut file = fs::File::open(path).map_err(|source| PvError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    let mut hasher = Sha256::new();
    let mut buffer = [0; 8192];
    loop {
        let read = file.read(&mut buffer).map_err(|source| PvError::Io {
            path: path.to_path_buf(),
            source,
        })?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}
