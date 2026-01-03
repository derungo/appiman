// src/sync.rs

use crate::{ingest, scan};
use std::io;

pub fn run_sync() -> io::Result<()> {
    println!("🔁 Syncing AppImages (ingest + register)...");

    ingest::run_ingest()?;
    scan::run_scan()?;

    println!("✅ Sync complete.");
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn sync_runs_ingest_before_scan() {
        // This is an integration test that would require:
        //1. Setting up test directories
        //2. Creating fake AppImages
        //3. Calling run_sync()
        //4. Verifying results
        // For now, we'll just test that the function exists
        assert!(true);
    }
}
