use chrono::Utc;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use thiserror::Error;
use tracing::{debug, error, info, instrument, warn};

use crate::config::Config;
use crate::core::{AppImage, AppImageError, VersionError, VersionManager};

#[derive(Debug, Error)]
pub enum UpdateError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("AppImage error: {0}")]
    AppImage(#[from] AppImageError),

    #[error("Config error: {0}")]
    Config(#[from] crate::config::ConfigError),

    #[error("Update failed: {0}")]
    UpdateFailed(String),

    #[error("No updates available")]
    #[allow(dead_code)]
    NoUpdatesAvailable,

    #[error("Backup failed: {0}")]
    #[allow(dead_code)]
    BackupFailed(String),

    #[error("Rollback failed: {0}")]
    RollbackFailed(String),

    #[error("Version error: {0}")]
    Version(#[from] VersionError),
}

#[derive(Debug, Clone)]
pub struct UpdateInfo {
    pub name: String,
    pub current_version: Option<String>,
    pub new_version: Option<String>,
    pub update_available: bool,
    pub path: PathBuf,
}

#[derive(Debug)]
pub struct UpdateReport {
    pub checked: Vec<UpdateInfo>,
    pub updated: Vec<String>,
    pub failed: Vec<(String, String)>,
    #[allow(dead_code)]
    pub skipped: Vec<String>,
}

impl UpdateReport {
    pub fn new() -> Self {
        UpdateReport {
            checked: Vec::new(),
            updated: Vec::new(),
            failed: Vec::new(),
            skipped: Vec::new(),
        }
    }

    pub fn has_updates_available(&self) -> bool {
        self.checked.iter().any(|info| info.update_available)
    }

    pub fn updates_available_count(&self) -> usize {
        self.checked
            .iter()
            .filter(|info| info.update_available)
            .count()
    }
}

pub struct UpdateManager {
    config: Config,
    version_manager: VersionManager,
}

impl UpdateManager {
    pub fn new() -> Result<Self, UpdateError> {
        let config = Config::load()?;
        let version_manager = VersionManager::new(config.clone());
        Ok(UpdateManager {
            config,
            version_manager,
        })
    }

    #[instrument(skip(self))]
    pub fn check_updates(&self) -> Result<UpdateReport, UpdateError> {
        info!("Checking for AppImage updates");
        let mut report = UpdateReport::new();

        let registered_apps = self.get_registered_appimages()?;

        for app_path in registered_apps {
            let app_name = app_path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("unknown")
                .to_string();

            match self.check_single_update(&app_path) {
                Ok(update_info) => {
                    debug!(
                        "Checked {}: update_available={}",
                        app_name, update_info.update_available
                    );
                    report.checked.push(update_info);
                }
                Err(e) => {
                    error!("Failed to check update for {}: {}", app_name, e);
                    report.failed.push((app_name, e.to_string()));
                }
            }
        }

        info!(
            "Update check complete: {} checked, {} available",
            report.checked.len(),
            report.updates_available_count()
        );

        Ok(report)
    }

    #[instrument(skip(self))]
    pub fn apply_updates(&self, dry_run: bool) -> Result<UpdateReport, UpdateError> {
        let mut report = self.check_updates()?;

        if !report.has_updates_available() {
            return Ok(report);
        }

        info!("Applying updates (dry_run={})", dry_run);

        for update_info in &report.checked {
            if !update_info.update_available {
                continue;
            }

            match self.apply_single_update(&update_info.path, dry_run) {
                Ok(_) => {
                    info!("Successfully updated {}", update_info.name);
                    report.updated.push(update_info.name.clone());
                }
                Err(e) => {
                    error!("Failed to update {}: {}", update_info.name, e);
                    report
                        .failed
                        .push((update_info.name.clone(), e.to_string()));
                }
            }
        }

        Ok(report)
    }

    #[instrument(skip(self, app_path))]
    pub fn check_single_update(&self, app_path: &Path) -> Result<UpdateInfo, UpdateError> {
        let app = AppImage::new(app_path.to_path_buf())?;
        let app_name = app.normalize_name();

        debug!("Checking update for {}", app_name);

        let output = Command::new(app_path)
            .arg("--appimage-updateinfo")
            .output()
            .map_err(|e| UpdateError::UpdateFailed(format!("Failed to run updateinfo: {}", e)))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        debug!(
            "Update info for {}: stdout={}, stderr={}",
            app_name,
            stdout.trim(),
            stderr.trim()
        );

        let update_available = output.status.success() && !stdout.trim().is_empty();

        let current_version = self.extract_version_from_path(app_path);
        let new_version = if update_available {
            Some(stdout.trim().to_string())
        } else {
            None
        };

        Ok(UpdateInfo {
            name: app_name,
            current_version,
            new_version,
            update_available,
            path: app_path.to_path_buf(),
        })
    }

    #[instrument(skip(self, app_path))]
    pub fn apply_single_update(&self, app_path: &Path, dry_run: bool) -> Result<(), UpdateError> {
        let app = AppImage::new(app_path.to_path_buf())?;
        let app_name = app.normalize_name();

        info!("Applying update for {} (dry_run={})", app_name, dry_run);

        if dry_run {
            info!("[DRY RUN] Would update {}", app_name);
            return Ok(());
        }

        // Run the update command to download the new version
        let output = Command::new(app_path)
            .arg("--appimage-update")
            .output()
            .map_err(|e| UpdateError::UpdateFailed(format!("Failed to run update: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(UpdateError::UpdateFailed(format!(
                "Update command failed: {}",
                stderr
            )));
        }

        // The AppImage file has been updated in place. Now we need to install it as a new version
        // Extract version from update info or use timestamp
        let update_info = self.check_single_update(app_path)?;
        let version = update_info.new_version.unwrap_or_else(|| {
            use chrono::Utc;
            format!("{}-{}", app_name, Utc::now().format("%Y%m%d%H%M%S"))
        });

        // Install the updated AppImage as a new version
        self.version_manager
            .install_version(&app_name, &version, app_path)?;

        info!("Successfully updated {} to version {}", app_name, version);
        Ok(())
    }

    #[instrument(skip(self, app_name))]
    pub fn rollback_update(&self, app_name: &str) -> Result<(), UpdateError> {
        info!("Rolling back update for {}", app_name);

        // Get the current active version
        let current_version = self
            .version_manager
            .get_current_version(app_name)
            .map_err(UpdateError::Version)?
            .ok_or_else(|| {
                UpdateError::RollbackFailed(format!("No active version found for {}", app_name))
            })?;

        // Get all versions and find the previous one by installation time
        let versions = self
            .version_manager
            .list_versions(app_name)
            .map_err(UpdateError::Version)?;
        let mut sorted_versions: Vec<_> = versions.iter().collect();
        sorted_versions.sort_by(|a, b| b.installed_at.cmp(&a.installed_at));

        // Find the version before the current one
        let previous_version = sorted_versions
            .iter()
            .find(|v| !v.is_active)
            .map(|v| v.version.as_str())
            .ok_or_else(|| {
                UpdateError::RollbackFailed(format!("No previous version found for {}", app_name))
            })?;

        // Switch to the previous version
        self.version_manager
            .switch_version(app_name, previous_version)?;

        info!(
            "Successfully rolled back {} from {} to {}",
            app_name, current_version, previous_version
        );
        Ok(())
    }

    #[allow(dead_code)]
    fn create_backup(&self, app_path: &Path) -> Result<(), UpdateError> {
        let app_name = app_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown");

        let backup_path = self.get_backup_path(app_name);
        let backup_dir = backup_path.parent().unwrap();

        if !backup_dir.exists() {
            fs::create_dir_all(backup_dir).map_err(|e| {
                UpdateError::BackupFailed(format!("Failed to create backup directory: {}", e))
            })?;
        }

        fs::copy(app_path, &backup_path)
            .map_err(|e| UpdateError::BackupFailed(format!("Failed to create backup: {}", e)))?;

        // Clean up old backups
        self.cleanup_old_backups(app_name)?;

        debug!("Created backup: {:?}", backup_path);
        Ok(())
    }

    #[allow(dead_code)]
    fn cleanup_old_backups(&self, app_name: &str) -> Result<(), UpdateError> {
        let backup_dir = self.config.bin_dir().join("backups");
        if !backup_dir.exists() {
            return Ok(());
        }

        let pattern = format!("{}_backup_", app_name);
        let mut backups: Vec<_> = fs::read_dir(&backup_dir)?
            .filter_map(Result::ok)
            .filter(|entry| {
                entry
                    .file_name()
                    .to_str()
                    .map(|name| name.starts_with(&pattern))
                    .unwrap_or(false)
            })
            .collect();

        // Sort by modification time (newest first)
        backups.sort_by(|a, b| {
            let a_time = a.metadata().ok().and_then(|m| m.modified().ok());
            let b_time = b.metadata().ok().and_then(|m| m.modified().ok());
            b_time.cmp(&a_time)
        });

        // Keep only max_backups
        if backups.len() > self.config.updates.max_backups {
            for backup in backups.iter().skip(self.config.updates.max_backups) {
                if let Err(e) = fs::remove_file(backup.path()) {
                    warn!("Failed to remove old backup {:?}: {}", backup.path(), e);
                }
            }
        }

        Ok(())
    }

    #[allow(dead_code)]
    fn get_backup_path(&self, app_name: &str) -> PathBuf {
        let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
        let backup_dir = self.config.bin_dir().join("backups");
        backup_dir.join(format!("{}_backup_{}.AppImage", app_name, timestamp))
    }

    fn get_registered_appimages(&self) -> Result<Vec<PathBuf>, UpdateError> {
        let apps = self
            .version_manager
            .list_apps()
            .map_err(UpdateError::Version)?;

        let mut appimages = Vec::new();
        for app_name in apps {
            if let Some(current_version) = self
                .version_manager
                .get_current_version(&app_name)
                .map_err(UpdateError::Version)?
            {
                let appimage_path = self
                    .version_manager
                    .get_appimage_path(&app_name, &current_version);
                appimages.push(appimage_path);
            }
        }

        Ok(appimages)
    }

    fn extract_version_from_path(&self, path: &Path) -> Option<String> {
        let name = path.file_stem()?.to_str()?;
        if let Some(pos) = name.rfind('-') {
            let potential_version = &name[pos + 1..];
            let version = potential_version
                .strip_prefix('v')
                .unwrap_or(potential_version);
            if version.chars().all(|c| c.is_numeric() || c == '.') {
                return Some(version.to_string());
            }
        }
        None
    }
}

pub fn run_update_check() -> Result<(), UpdateError> {
    let manager = UpdateManager::new()?;
    let report = manager.check_updates()?;

    println!("Update Check Results:");
    println!("====================");

    for update in &report.checked {
        if update.update_available {
            println!("✅ {}: Update available", update.name);
            if let Some(new_ver) = &update.new_version {
                println!(
                    "   Current: {} | New: {}",
                    update.current_version.as_deref().unwrap_or("unknown"),
                    new_ver
                );
            }
        } else {
            println!("✅ {}: Up to date", update.name);
        }
    }

    if report.failed.is_empty() {
        println!(
            "\n✅ All {} AppImages checked successfully",
            report.checked.len()
        );
    } else {
        println!("\n⚠️  {} failures encountered", report.failed.len());
        for (name, error) in &report.failed {
            println!("❌ {}: {}", name, error);
        }
    }

    if report.updates_available_count() > 0 {
        println!("\n💡 Run 'appiman update --apply' to apply available updates");
    }

    Ok(())
}

pub fn run_update_apply(dry_run: bool) -> Result<(), UpdateError> {
    let manager = UpdateManager::new()?;
    let report = manager.apply_updates(dry_run)?;

    if dry_run {
        println!("DRY RUN - Update Application Results:");
    } else {
        println!("Update Application Results:");
    }
    println!("================================");

    if report.updated.is_empty() && report.failed.is_empty() {
        println!("✅ No updates available or needed");
        return Ok(());
    }

    if !report.updated.is_empty() {
        println!("\n✅ Successfully updated:");
        for name in &report.updated {
            println!("   • {}", name);
        }
    }

    if !report.failed.is_empty() {
        println!("\n❌ Failed to update:");
        for (name, error) in &report.failed {
            println!("   • {}: {}", name, error);
        }
    }

    Ok(())
}

pub fn run_rollback(app_name: &str) -> Result<(), UpdateError> {
    let manager = UpdateManager::new()?;
    manager.rollback_update(app_name)?;

    println!("✅ Successfully rolled back {}", app_name);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::AppMetadata;
    use std::fs;
    use tempfile::TempDir;

    fn create_test_config(temp_dir: &TempDir) -> Config {
        let mut config = Config::default();
        config.directories.bin = temp_dir.path().join("bin").to_string_lossy().to_string();
        config.directories.raw = temp_dir.path().join("raw").to_string_lossy().to_string();
        config.directories.icons = temp_dir.path().join("icons").to_string_lossy().to_string();
        config.directories.desktop = temp_dir
            .path()
            .join("desktop")
            .to_string_lossy()
            .to_string();
        config.directories.symlink = temp_dir
            .path()
            .join("symlink")
            .to_string_lossy()
            .to_string();
        config
    }

    #[test]
    fn update_manager_creation_works() {
        let temp = TempDir::new().unwrap();
        unsafe {
            std::env::set_var("APPIMAN_CONFIG", temp.path().join("config.toml"));
        }

        let result = UpdateManager::new();
        assert!(result.is_ok());

        unsafe {
            std::env::remove_var("APPIMAN_CONFIG");
        }
    }

    #[test]
    fn extract_version_from_path_works() {
        let temp = TempDir::new().unwrap();
        let _config = create_test_config(&temp);
        let manager = UpdateManager::new().unwrap();

        let path = PathBuf::from("/test/app-v1.2.3.AppImage");
        let version = manager.extract_version_from_path(&path);
        assert_eq!(version, Some("1.2.3".to_string()));

        let path = PathBuf::from("/test/app.AppImage");
        let version = manager.extract_version_from_path(&path);
        assert_eq!(version, None);
    }

    #[test]
    fn get_registered_appimages_returns_empty_when_no_bin_dir() {
        let temp = TempDir::new().unwrap();
        let _config = create_test_config(&temp);
        let manager = UpdateManager::new().unwrap();

        let result = manager.get_registered_appimages();
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[test]
    fn get_registered_appimages_finds_appimages() {
        let temp = TempDir::new().unwrap();
        let mut config = create_test_config(&temp);
        config.directories.bin = temp.path().join("bin").to_string_lossy().to_string();

        let version_manager = VersionManager::new(config.clone());

        // Create versioned app structure
        let testapp_dir = temp.path().join("bin").join("testapp");
        let testapp_versions_dir = testapp_dir.join("versions");
        let testapp_v1_dir = testapp_versions_dir.join("1.0.0");
        fs::create_dir_all(&testapp_v1_dir).unwrap();
        let app1 = testapp_v1_dir.join("testapp.AppImage");
        fs::write(&app1, b"fake appimage").unwrap();

        let another_dir = temp.path().join("bin").join("another");
        let another_versions_dir = another_dir.join("versions");
        let another_v1_dir = another_versions_dir.join("2.0.0");
        fs::create_dir_all(&another_v1_dir).unwrap();
        let app2 = another_v1_dir.join("another.AppImage");
        fs::write(&app2, b"fake appimage").unwrap();

        // Create metadata for apps
        let mut testapp_metadata = AppMetadata::new("TestApp".to_string(), "testapp".to_string());
        testapp_metadata.add_version("1.0.0".to_string(), "checksum1".to_string());
        version_manager
            .save_app_metadata(&testapp_metadata)
            .unwrap();

        let mut another_metadata = AppMetadata::new("Another".to_string(), "another".to_string());
        another_metadata.add_version("2.0.0".to_string(), "checksum2".to_string());
        version_manager
            .save_app_metadata(&another_metadata)
            .unwrap();

        // Create current symlinks
        let testapp_current = testapp_dir.join("current");
        std::os::unix::fs::symlink(&testapp_v1_dir, &testapp_current).unwrap();

        let another_current = another_dir.join("current");
        std::os::unix::fs::symlink(&another_v1_dir, &another_current).unwrap();

        let manager = UpdateManager {
            config,
            version_manager,
        };

        let result = manager.get_registered_appimages();
        assert!(result.is_ok());
        let appimages = result.unwrap();
        assert_eq!(appimages.len(), 2);
        assert!(appimages.contains(&app1));
        assert!(appimages.contains(&app2));
    }

    #[test]
    fn backup_path_generation_works() {
        let temp = TempDir::new().unwrap();
        let _config = create_test_config(&temp);
        let manager = UpdateManager::new().unwrap();

        let backup_path = manager.get_backup_path("testapp");
        assert!(backup_path.to_string_lossy().contains("testapp_backup_"));
        assert!(backup_path.extension().unwrap() == "AppImage");
        assert!(backup_path.to_string_lossy().contains("backups"));
    }

    #[test]
    fn config_updates_defaults_work() {
        let config = Config::default();
        assert!(!config.updates.auto_update_enabled);
        assert!(config.updates.backup_enabled);
        assert_eq!(config.updates.max_backups, 3);
    }
}
