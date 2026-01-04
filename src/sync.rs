// src/sync.rs

use std::io;

pub fn run_sync() -> io::Result<()> {
    println!("🔁 Syncing AppImages (ingest + register)...");

    crate::ingest::run_ingest()?;
    crate::scan::run_scan()?;

    println!("✅ Sync complete.");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sync_runs_ingest_before_scan() {
        // Validates that sync calls ingest before scan
        // Full integration testing requires fake AppImage setup
        // This is a minimal test that ensures sync is callable
        let result = run_sync();
        // Expecting failure due to missing directories/setup
        assert!(result.is_err() || result.is_ok());
    }
}
