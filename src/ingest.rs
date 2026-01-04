// src/ingest.rs

use crate::config::Config;
use crate::mover::{Mover, Scanner};
use crate::privileges::require_root;
use std::io;

pub fn run_ingest() -> io::Result<()> {
    require_root()?;

    let config = Config::load().map_err(|e| {
        io::Error::new(
            io::ErrorKind::Other,
            format!("Failed to load config: {}", e),
        )
    })?;

    println!("📥 Ingesting user-downloaded AppImages...");

    let scanner = Scanner::new(config.home_root());
    let appimages = scanner.find_appimages().map_err(|e| {
        io::Error::new(
            io::ErrorKind::Other,
            format!("Failed to scan for AppImages: {}", e),
        )
    })?;

    if appimages.is_empty() {
        println!("ℹ️  No AppImages found to ingest.");
        return Ok(());
    }

    let mover = Mover::new(config.home_root(), config.raw_dir());
    let report = mover.move_appimages(&appimages).map_err(|e| {
        io::Error::new(
            io::ErrorKind::Other,
            format!("Failed to move AppImages: {}", e),
        )
    })?;

    println!("✅ Ingest complete: {} moved.", report.success_count());

    if !report.errors.is_empty() {
        println!("⚠️  {} errors occurred.", report.error_count());
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::fs;
    use tempfile::TempDir;

    use crate::mover::{Mover, Scanner};

    #[test]
    fn ingest_finds_and_moves_appimages() {
        let temp_dir = TempDir::new().unwrap();
        let home_root = temp_dir.path().join("home");
        let raw_dir = temp_dir.path().join("raw");
        let user_home = home_root.join("testuser");
        let downloads = user_home.join("Downloads");

        fs::create_dir_all(&downloads).unwrap();
        fs::create_dir_all(&raw_dir).unwrap();

        let appimage_path = downloads.join("TestApp.AppImage");
        fs::write(&appimage_path, b"fake appimage").unwrap();

        let scanner = Scanner::new(home_root.clone());
        let found_apps = scanner.find_appimages().unwrap();

        assert_eq!(found_apps.len(), 1);
        assert_eq!(found_apps[0].path, appimage_path);

        let mover = Mover::new(home_root, raw_dir.clone());
        let report = mover.move_appimages(&found_apps).unwrap();

        assert_eq!(report.success_count(), 1);
        assert_eq!(report.errors.len(), 0);
        assert!(raw_dir.join("TestApp.AppImage").exists());
        assert!(!appimage_path.exists());
    }

    #[test]
    fn ingest_handles_multiple_users() {
        let temp_dir = TempDir::new().unwrap();
        let home_root = temp_dir.path().join("home");
        let raw_dir = temp_dir.path().join("raw");

        fs::create_dir_all(&raw_dir).unwrap();

        for user in ["alice", "bob"] {
            let user_home = home_root.join(user);
            let downloads = user_home.join("Downloads");
            fs::create_dir_all(&downloads).unwrap();

            let appimage = downloads.join(format!("{}.AppImage", user));
            fs::write(&appimage, b"fake appimage").unwrap();
        }

        let scanner = Scanner::new(home_root.clone());
        let found_paths = scanner.find_appimages().unwrap();

        assert_eq!(found_paths.len(), 2);

        let mover = Mover::new(home_root, raw_dir.clone());
        let report = mover.move_appimages(&found_paths).unwrap();

        assert_eq!(report.success_count(), 2);
        assert!(raw_dir.join("alice.AppImage").exists());
        assert!(raw_dir.join("bob.AppImage").exists());
    }
}
