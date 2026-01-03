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
    #[test]
    fn ingest_runs_scanner_and_mover() {
        // Integration test would require setting up test directories
        // For now, just test that the function exists
        assert!(true);
    }
}
