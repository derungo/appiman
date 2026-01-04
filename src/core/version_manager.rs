use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;
use tracing::{info, warn};

use crate::config::Config;
use crate::core::{AppImage, AppImageError, AppMetadata, VersionInfo};

#[derive(Debug, Error)]
pub enum VersionError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("AppImage error: {0}")]
    AppImage(#[from] AppImageError),

    #[error("Metadata error: {0}")]
    Metadata(#[from] crate::core::metadata::MetadataError),

    #[error("Version not found: {0}")]
    VersionNotFound(String),

    #[error("Version already exists: {0}")]
    VersionExists(String),

    #[error("Invalid version format: {0}")]
    InvalidVersion(String),

    #[error("App not found: {0}")]
    AppNotFound(String),
}

pub struct VersionManager {
    config: Config,
}

impl VersionManager {
    pub fn new(config: Config) -> Self {
        VersionManager { config }
    }

    pub fn get_app_dir(&self, app_name: &str) -> PathBuf {
        self.config.bin_dir().join(app_name)
    }

    pub fn get_versions_dir(&self, app_name: &str) -> PathBuf {
        self.get_app_dir(app_name).join("versions")
    }

    pub fn get_version_dir(&self, app_name: &str, version: &str) -> PathBuf {
        self.get_versions_dir(app_name).join(version)
    }

    pub fn get_appimage_path(&self, app_name: &str, version: &str) -> PathBuf {
        self.get_version_dir(app_name, version)
            .join(format!("{}.AppImage", app_name))
    }

    pub fn get_current_link(&self, app_name: &str) -> PathBuf {
        self.get_app_dir(app_name).join("current")
    }

    pub fn get_metadata_path(&self, app_name: &str) -> PathBuf {
        self.get_app_dir(app_name).join("metadata.json")
    }

    pub fn load_app_metadata(&self, app_name: &str) -> Result<AppMetadata, VersionError> {
        let metadata_path = self.get_metadata_path(app_name);
        if !metadata_path.exists() {
            return Err(VersionError::AppNotFound(app_name.to_string()));
        }

        let content = fs::read_to_string(&metadata_path)?;
        Ok(AppMetadata::from_json(&content)?)
    }

    pub fn save_app_metadata(&self, metadata: &AppMetadata) -> Result<(), VersionError> {
        let metadata_path = self.get_metadata_path(&metadata.name);
        let app_dir = metadata_path.parent().unwrap();

        if !app_dir.exists() {
            fs::create_dir_all(app_dir)?;
        }

        let json = metadata.to_json()?;
        fs::write(&metadata_path, json)?;
        Ok(())
    }

    pub fn install_version(
        &self,
        app_name: &str,
        version: &str,
        appimage_path: &Path,
    ) -> Result<(), VersionError> {
        let app = AppImage::new(appimage_path.to_path_buf())?;
        let checksum = app.get_checksum()?;

        // Load or create app metadata
        let mut metadata = match self.load_app_metadata(app_name) {
            Ok(m) => m,
            Err(VersionError::AppNotFound(_)) => {
                // First version for this app
                AppMetadata::new(app_name.to_string(), app_name.to_string())
            }
            Err(e) => return Err(e),
        };

        // Check if version already exists
        if metadata.get_version(version).is_some() {
            return Err(VersionError::VersionExists(version.to_string()));
        }

        // Create version directory
        let version_dir = self.get_version_dir(app_name, version);
        fs::create_dir_all(&version_dir)?;

        // Copy AppImage
        let target_path = self.get_appimage_path(app_name, version);
        fs::copy(appimage_path, &target_path)?;

        // Make executable
        self.make_executable(&target_path)?;

        // Add version to metadata
        metadata.add_version(version.to_string(), checksum);
        self.save_app_metadata(&metadata)?;

        // Update current symlink
        self.update_current_link(app_name)?;

        // Cleanup old versions
        self.cleanup_old_versions(app_name)?;

        info!("Installed {} version {}", app_name, version);
        Ok(())
    }

    pub fn switch_version(&self, app_name: &str, version: &str) -> Result<(), VersionError> {
        let mut metadata = self.load_app_metadata(app_name)?;

        if !metadata.set_active_version(version) {
            return Err(VersionError::VersionNotFound(version.to_string()));
        }

        self.save_app_metadata(&metadata)?;
        self.update_current_link(app_name)?;

        info!("Switched {} to version {}", app_name, version);
        Ok(())
    }

    pub fn remove_version(&self, app_name: &str, version: &str) -> Result<(), VersionError> {
        let mut metadata = self.load_app_metadata(app_name)?;

        if metadata.versions.len() <= 1 {
            return Err(VersionError::InvalidVersion(
                "Cannot remove the last version".to_string(),
            ));
        }

        if metadata.get_active_version().map(|v| v.version.as_str()) == Some(version) {
            return Err(VersionError::InvalidVersion(
                "Cannot remove active version".to_string(),
            ));
        }

        // Remove version directory
        let version_dir = self.get_version_dir(app_name, version);
        if version_dir.exists() {
            fs::remove_dir_all(&version_dir)?;
        }

        // Remove from metadata
        metadata.remove_version(version);
        self.save_app_metadata(&metadata)?;

        info!("Removed {} version {}", app_name, version);
        Ok(())
    }

    pub fn list_versions(&self, app_name: &str) -> Result<Vec<VersionInfo>, VersionError> {
        let metadata = self.load_app_metadata(app_name)?;
        Ok(metadata.versions.clone())
    }

    pub fn get_current_version(&self, app_name: &str) -> Result<Option<String>, VersionError> {
        let metadata = self.load_app_metadata(app_name)?;
        Ok(metadata.get_active_version().map(|v| v.version.clone()))
    }

    pub fn list_apps(&self) -> Result<Vec<String>, VersionError> {
        let bin_dir = self.config.bin_dir();
        if !bin_dir.exists() {
            return Ok(Vec::new());
        }

        let mut apps = Vec::new();
        for entry in fs::read_dir(&bin_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir()
                && self
                    .get_metadata_path(&entry.file_name().to_string_lossy())
                    .exists()
            {
                apps.push(entry.file_name().to_string_lossy().to_string());
            }
        }
        Ok(apps)
    }

    pub fn cleanup_old_versions(&self, app_name: &str) -> Result<(), VersionError> {
        if !self.config.versions.auto_cleanup_enabled {
            return Ok(());
        }

        let mut metadata = self.load_app_metadata(app_name)?;
        let max_versions = self.config.versions.max_versions_per_app;

        if metadata.versions.len() <= max_versions {
            return Ok(());
        }

        // Sort versions by installation date, keep newest
        metadata
            .versions
            .sort_by(|a, b| b.installed_at.cmp(&a.installed_at));

        // Remove old versions
        let to_remove: Vec<String> = metadata
            .versions
            .iter()
            .skip(max_versions)
            .map(|v| v.version.clone())
            .collect();

        for version in to_remove {
            if metadata.get_active_version().map(|v| v.version.as_str()) != Some(&version) {
                warn!("Removing old version {} of {}", version, app_name);
                let version_dir = self.get_version_dir(app_name, &version);
                if version_dir.exists() {
                    fs::remove_dir_all(&version_dir)?;
                }
                metadata.remove_version(&version);
            }
        }

        self.save_app_metadata(&metadata)?;
        Ok(())
    }

    #[allow(dead_code)]
    pub fn migrate_legacy_app(
        &self,
        app_name: &str,
        appimage_path: &Path,
    ) -> Result<(), VersionError> {
        let app = AppImage::new(appimage_path.to_path_buf())?;
        let checksum = app.get_checksum()?;

        // Extract version from filename or use "legacy"
        let version = app.normalize_name();
        let version = if version == app_name {
            "legacy".to_string()
        } else {
            // Try to extract version from normalized name
            if let Some(pos) = version.rfind('-') {
                let potential_version = &version[pos + 1..];
                if potential_version
                    .chars()
                    .all(|c| c.is_numeric() || c == '.')
                {
                    potential_version.to_string()
                } else {
                    "legacy".to_string()
                }
            } else {
                "legacy".to_string()
            }
        };

        let mut metadata = AppMetadata::new(app_name.to_string(), app_name.to_string());
        metadata.add_version(version, checksum);

        // Create version directory structure
        let version_dir = self.get_version_dir(app_name, &metadata.versions[0].version);
        fs::create_dir_all(&version_dir)?;

        // Move existing AppImage
        let target_path = self.get_appimage_path(app_name, &metadata.versions[0].version);
        fs::rename(appimage_path, &target_path)?;

        // Save metadata
        self.save_app_metadata(&metadata)?;

        // Create current symlink
        self.update_current_link(app_name)?;

        info!("Migrated legacy app {} to versioned storage", app_name);
        Ok(())
    }

    fn update_current_link(&self, app_name: &str) -> Result<(), VersionError> {
        let metadata = self.load_app_metadata(app_name)?;
        if let Some(active_version) = metadata.get_active_version() {
            let current_link = self.get_current_link(app_name);
            let version_dir = self.get_version_dir(app_name, &active_version.version);

            // Remove existing link
            if current_link.exists() {
                fs::remove_file(&current_link)?;
            }

            // Create new symlink
            std::os::unix::fs::symlink(&version_dir, &current_link)?;
        }
        Ok(())
    }

    fn make_executable(&self, path: &Path) -> Result<(), VersionError> {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(path)?.permissions();
            perms.set_mode(0o755);
            fs::set_permissions(path, perms)?;
        }
        Ok(())
    }
}
