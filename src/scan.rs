// src/scan.rs

use crate::config::Config;
use crate::core::VersionManager;
use crate::registrar::Processor;
use crate::security::SecurityChecker;
use std::io;

pub fn run_scan() -> io::Result<()> {
    let config =
        Config::load().map_err(|e| io::Error::other(format!("Failed to load config: {}", e)))?;

    println!("🔄 Triggering full AppImage re-registration...");

    let version_manager = VersionManager::new(config.clone());
    let security_checker = SecurityChecker {
        verify_signatures: config.security.verify_signatures,
        require_signatures: config.security.require_signatures,
        warn_unsigned: config.security.warn_unsigned,
        detect_sandboxing: config.security.detect_sandboxing,
    };
    let processor = Processor::new(
        config.raw_dir(),
        config.bin_dir(),
        config.icon_dir(),
        config.desktop_dir(),
        config.symlink_dir(),
        version_manager,
        security_checker,
    )
    .with_performance_config(
        Some(config.raw_dir().join(".cache")),
        config.performance.parallel_processing_enabled,
        config.performance.incremental_scan_enabled,
        None, // TODO: implement last scan time tracking
    );

    let report = processor
        .process_all()
        .map_err(|e| io::Error::other(format!("Failed to process AppImages: {}", e)))?;

    println!(
        "✅ Re-registration complete: {} processed.",
        report.success_count()
    );

    if !report.failed.is_empty() {
        println!("⚠️  {} AppImages failed to process.", report.failed.len());
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::fs;
    use tempfile::TempDir;

    use crate::config::Config;
    use crate::core::VersionManager;
    use crate::registrar::Processor;
    use crate::security::SecurityChecker;

    #[test]
    fn scan_creates_desktop_entry_and_symlink() {
        let temp_dir = TempDir::new().unwrap();
        let raw_dir = temp_dir.path().join("raw");
        let bin_dir = temp_dir.path().join("bin");
        let icon_dir = temp_dir.path().join("icons");
        let desktop_dir = temp_dir.path().join("desktop");
        let symlink_dir = temp_dir.path().join("symlinks");

        fs::create_dir_all(&raw_dir).unwrap();
        fs::create_dir_all(&bin_dir).unwrap();
        fs::create_dir_all(&icon_dir).unwrap();
        fs::create_dir_all(&desktop_dir).unwrap();
        fs::create_dir_all(&symlink_dir).unwrap();

        let mut config = Config::default();
        config.directories.raw = raw_dir.to_string_lossy().to_string();
        config.directories.bin = bin_dir.to_string_lossy().to_string();
        config.directories.icons = icon_dir.to_string_lossy().to_string();
        config.directories.desktop = desktop_dir.to_string_lossy().to_string();
        config.directories.symlink = symlink_dir.to_string_lossy().to_string();

        let version_manager = VersionManager::new(config);
        let security_checker = SecurityChecker::new();
        let processor = Processor::new(
            raw_dir.clone(),
            bin_dir.clone(),
            icon_dir.clone(),
            desktop_dir.clone(),
            symlink_dir.clone(),
            version_manager,
            security_checker,
        )
        .with_performance_config(None, false, false, None);

        let report = processor.process_all().unwrap();

        assert_eq!(report.success_count(), 0);
        assert_eq!(report.failure_count(), 0);
    }

    #[test]
    fn scan_with_fake_appimage_creates_expected_files() {
        let temp_dir = TempDir::new().unwrap();
        let raw_dir = temp_dir.path().join("raw");
        let bin_dir = temp_dir.path().join("bin");
        let icon_dir = temp_dir.path().join("icons");
        let desktop_dir = temp_dir.path().join("desktop");
        let symlink_dir = temp_dir.path().join("symlinks");

        fs::create_dir_all(&raw_dir).unwrap();
        fs::create_dir_all(&bin_dir).unwrap();
        fs::create_dir_all(&icon_dir).unwrap();
        fs::create_dir_all(&desktop_dir).unwrap();
        fs::create_dir_all(&symlink_dir).unwrap();

        let mut config = Config::default();
        config.directories.raw = raw_dir.to_string_lossy().to_string();
        config.directories.bin = bin_dir.to_string_lossy().to_string();
        config.directories.icons = icon_dir.to_string_lossy().to_string();
        config.directories.desktop = desktop_dir.to_string_lossy().to_string();
        config.directories.symlink = symlink_dir.to_string_lossy().to_string();

        let version_manager = VersionManager::new(config);
        let security_checker = SecurityChecker::new();
        let processor = Processor::new(
            raw_dir.clone(),
            bin_dir.clone(),
            icon_dir.clone(),
            desktop_dir.clone(),
            symlink_dir.clone(),
            version_manager,
            security_checker,
        )
        .with_performance_config(None, false, false, None);

        let report = processor.process_all().unwrap();
        assert!(report.processed.is_empty());
    }
}
