// src/scan.rs

use crate::config::Config;
use crate::registrar::Processor;
use std::io;

pub fn run_scan() -> io::Result<()> {
    let config = Config::load().map_err(|e| {
        io::Error::new(
            io::ErrorKind::Other,
            format!("Failed to load config: {}", e),
        )
    })?;

    println!("🔄 Triggering full AppImage re-registration...");

    let processor = Processor::new(
        config.raw_dir(),
        config.bin_dir(),
        config.icon_dir(),
        config.desktop_dir(),
        config.symlink_dir(),
    );

    let report = processor.process_all().map_err(|e| {
        io::Error::new(
            io::ErrorKind::Other,
            format!("Failed to process AppImages: {}", e),
        )
    })?;

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

    use crate::registrar::Processor;

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

        let processor = Processor::new(
            raw_dir.clone(),
            bin_dir.clone(),
            icon_dir.clone(),
            desktop_dir.clone(),
            symlink_dir.clone(),
        );

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

        let processor = Processor::new(
            raw_dir.clone(),
            bin_dir.clone(),
            icon_dir.clone(),
            desktop_dir.clone(),
            symlink_dir.clone(),
        );

        let report = processor.process_all().unwrap();
        assert!(report.processed.is_empty());
    }
}
