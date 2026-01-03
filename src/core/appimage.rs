use std::path::PathBuf;
use thiserror::Error;

use super::Metadata;
use super::normalize_appimage_name;

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

    #[error("Metadata not found")]
    MetadataNotFound,
}

#[derive(Debug, Error)]
pub enum ExtractError {
    #[error("Extraction failed: {0}")]
    ExtractionFailed(String),

    #[error("Temp directory creation failed: {0}")]
    TempDirFailed(#[from] std::io::Error),

    #[error("AppImage execution failed: {status}")]
    ExecutionFailed { status: std::process::ExitStatus },
}

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
            .map_or(false, |ext| ext.eq_ignore_ascii_case("AppImage"))
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::{NamedTempFile, TempDir};

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
}
