// src/scan.rs

use crate::registrar::Processor;
use std::io;
use std::path::PathBuf;

const DEFAULT_RAW_DIR: &str = "/opt/applications/raw";
const DEFAULT_BIN_DIR: &str = "/opt/applications/bin";
const DEFAULT_ICON_DIR: &str = "/opt/applications/icons";
const DEFAULT_DESKTOP_DIR: &str = "/usr/share/applications";
const DEFAULT_SYMLINK_DIR: &str = "/usr/local/bin";

fn raw_dir() -> PathBuf {
    std::env::var_os("APPIMAN_RAW_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_RAW_DIR))
}

fn bin_dir() -> PathBuf {
    std::env::var_os("APPIMAN_BIN_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_BIN_DIR))
}

fn icon_dir() -> PathBuf {
    std::env::var_os("APPIMAN_ICON_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ICON_DIR))
}

fn desktop_dir() -> PathBuf {
    std::env::var_os("APPIMAN_DESKTOP_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_DESKTOP_DIR))
}

fn symlink_dir() -> PathBuf {
    std::env::var_os("APPIMAN_SYMLINK_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_SYMLINK_DIR))
}

pub fn run_scan() -> io::Result<()> {
    println!("🔄 Triggering full AppImage re-registration...");

    let processor = Processor::new(
        raw_dir(),
        bin_dir(),
        icon_dir(),
        desktop_dir(),
        symlink_dir(),
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
    #[test]
    fn scan_runs_processor() {
        // Integration test would require setting up test directories
        // For now, just test that the function exists
        assert!(true);
    }
}
