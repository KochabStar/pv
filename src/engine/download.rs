use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use crate::error::{PvError, Result};

pub async fn download_to_cache(
    url: &str,
    cache_dir: &Path,
    file_name: &str,
    expected_hash: &str,
) -> Result<PathBuf> {
    fs::create_dir_all(cache_dir).map_err(|source| PvError::Io {
        path: cache_dir.to_path_buf(),
        source,
    })?;
    let target = cache_dir.join(file_name);

    if let Some(path) = url.strip_prefix("file://") {
        fs::copy(path, &target).map_err(|source| PvError::Io {
            path: PathBuf::from(path),
            source,
        })?;
    } else {
        let response = reqwest::get(url)
            .await
            .map_err(|error| PvError::Download(error.to_string()))?;
        if !response.status().is_success() {
            return Err(PvError::Download(format!(
                "{url} returned {}",
                response.status()
            )));
        }
        let bytes = response
            .bytes()
            .await
            .map_err(|error| PvError::Download(error.to_string()))?;
        fs::write(&target, bytes).map_err(|source| PvError::Io {
            path: target.clone(),
            source,
        })?;
    }

    if let Err(error) = verify_sha256(&target, expected_hash) {
        let _ = fs::remove_file(&target);
        return Err(error);
    }
    Ok(target)
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
