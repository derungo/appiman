use hex;
use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::Read;
use std::path::PathBuf;
use thiserror::Error;

use super::normalize_appimage_name;
use super::Metadata;

#[derive(Debug, Error)]
pub enum AppImageError {
    #[error("AppImage file not found: {0}")]
    NotFound(PathBuf),

    #[error("Invalid AppImage format: {0}")]
    InvalidFormat(String),

    #[error("Failed to extract AppImage: {0}")]
    ExtractFailed(#[from] ExtractError),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Debug, Error)]
#[allow(dead_code)]
pub enum ExtractError {
    #[error("Extraction failed: {0}")]
    ExtractionFailed(String),

    #[error("Temp directory creation failed: {0}")]
    TempDirFailed(#[from] std::io::Error),

    #[error("AppImage execution failed: {status}")]
    ExecutionFailed { status: std::process::ExitStatus },
}

#[derive(Debug, PartialEq)]
pub struct AppImage {
    pub path: PathBuf,
    pub metadata: Option<Metadata>,
}

impl AppImage {
    pub fn new(path: PathBuf) -> Result<Self, AppImageError> {
        if !path.exists() {
            return Err(AppImageError::NotFound(path));
        }

        if !path
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("AppImage"))
        {
            return Err(AppImageError::InvalidFormat(format!(
                "Invalid extension: {:?}",
                path.extension()
            )));
        }

        Ok(AppImage {
            path,
            metadata: None,
        })
    }

    pub fn validate(&self) -> Result<(), AppImageError> {
        if !self.path.exists() {
            return Err(AppImageError::NotFound(self.path.clone()));
        }

        let metadata = std::fs::metadata(&self.path)?;
        if !metadata.is_file() {
            return Err(AppImageError::InvalidFormat(
                "Not a regular file".to_string(),
            ));
        }

        Ok(())
    }

    pub fn normalize_name(&self) -> String {
        let stem = self.path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
        normalize_appimage_name(stem)
    }

    #[cfg(unix)]
    pub fn is_executable(&self) -> Result<bool, AppImageError> {
        use std::os::unix::fs::PermissionsExt;
        let metadata = std::fs::metadata(&self.path)?;
        let mode = metadata.permissions().mode();
        Ok(mode & 0o111 != 0)
    }

    #[cfg(not(unix))]
    pub fn is_executable(&self) -> Result<bool, AppImageError> {
        Ok(true)
    }

    pub fn get_checksum(&self) -> Result<String, AppImageError> {
        let mut file = File::open(&self.path)?;
        let mut hasher = Sha256::new();
        let mut buffer = [0u8; 8192];

        loop {
            let n = file.read(&mut buffer)?;
            if n == 0 {
                break;
            }
            hasher.update(&buffer[..n]);
        }

        Ok(hex::encode(hasher.finalize()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn appimage_new_rejects_nonexistent_path() {
        let path = PathBuf::from("/nonexistent/path.AppImage");
        let result = AppImage::new(path);
        assert!(matches!(result, Err(AppImageError::NotFound(_))));
    }

    #[test]
    fn appimage_new_rejects_invalid_extension() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("testfile.txt");
        fs::write(&path, b"test content").unwrap();

        let result = AppImage::new(path);
        assert!(matches!(result, Err(AppImageError::InvalidFormat(_))));
    }

    #[test]
    fn appimage_new_accepts_valid_extension() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("TestApp.AppImage");
        fs::write(&path, b"test content").unwrap();

        let result = AppImage::new(path);
        assert!(result.is_ok());
    }

    #[test]
    fn normalize_name_handles_appimage_files() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("TestApp-v1.2.3-x86_64.AppImage");
        fs::write(&path, b"test content").unwrap();

        let app = AppImage::new(path).unwrap();
        assert_eq!(app.normalize_name(), "testapp");
    }

    #[test]
    fn validate_works_on_valid_appimage() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("TestApp.AppImage");
        fs::write(&path, b"test content").unwrap();

        let app = AppImage::new(path).unwrap();
        assert!(app.validate().is_ok());
    }

    #[test]
    fn get_checksum_returns_sha256_hash() {
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test.AppImage");
        let content = b"test content for checksum";

        fs::write(&test_file, content).unwrap();

        let app = AppImage::new(test_file).unwrap();
        let checksum = app.get_checksum().unwrap();

        assert_eq!(checksum.len(), 64);
        assert!(checksum
            .chars()
            .all(|c| c.is_ascii_hexdigit() || c.is_ascii_lowercase()));

        let expected_hash = sha2::Sha256::digest(content);
        let expected_hex = hex::encode(expected_hash);
        assert_eq!(checksum, expected_hex);
    }
}
