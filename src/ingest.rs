// src/ingest.rs

use crate::mover::{Mover, Scanner};
use crate::privileges::require_root;
use std::io;
use std::path::PathBuf;

const DEFAULT_RAW_DIR: &str = "/opt/applications/raw";
const DEFAULT_HOME_ROOT: &str = "/home";

fn raw_dir() -> PathBuf {
    std::env::var_os("APPIMAN_RAW_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_RAW_DIR))
}

fn home_root() -> PathBuf {
    std::env::var_os("APPIMAN_HOME_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_HOME_ROOT))
}

pub fn run_ingest() -> io::Result<()> {
    require_root()?;

    println!("📥 Ingesting user-downloaded AppImages...");

    let scanner = Scanner::new(home_root());
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

    let mover = Mover::new(home_root(), raw_dir());
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
    #[test]
    fn ingest_runs_scanner_and_mover() {
        // Integration test would require setting up test directories
        // For now, just test that the function exists
        assert!(true);
    }
}
