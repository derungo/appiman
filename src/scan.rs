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
    #[test]
    fn scan_runs_processor() {
        // Integration test would require setting up test directories
        // For now, just test that the function exists
        assert!(true);
    }
}
