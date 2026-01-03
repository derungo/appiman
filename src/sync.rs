// src/sync.rs

use crate::{ingest, scan};
use std::io;

#[cfg(test)]
fn run_sync_impl(
    move_script: &std::path::Path,
    register_script: &std::path::Path,
) -> io::Result<()> {
    ingest::run_move_script(move_script)?;
    scan::run_register_script(register_script)?;
    Ok(())
}

pub fn run_sync() -> io::Result<()> {
    println!("🔁 Syncing AppImages (ingest + register)...");

    ingest::run_ingest()?;
    scan::run_scan()?;

    println!("✅ Sync complete.");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::Path;

    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;

    use tempfile::TempDir;

    #[cfg(unix)]
    fn write_executable(path: &Path, contents: &str) {
        fs::write(path, contents).unwrap();
        let mut perms = fs::metadata(path).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(path, perms).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn run_sync_impl_runs_ingest_before_register() {
        let root = TempDir::new().unwrap();

        let move_script = root.path().join("move.sh");
        let register_script = root.path().join("register.sh");

        write_executable(
            &move_script,
            r#"#!/usr/bin/env bash
set -euo pipefail
touch "$(dirname "$0")/moved"
"#,
        );

        write_executable(
            &register_script,
            r#"#!/usr/bin/env bash
set -euo pipefail
if [[ ! -f "$(dirname "$0")/moved" ]]; then
  echo "moved marker missing" >&2
  exit 1
fi
touch "$(dirname "$0")/registered"
"#,
        );

        run_sync_impl(&move_script, &register_script).unwrap();

        assert!(root.path().join("registered").exists());
    }

    #[cfg(unix)]
    #[test]
    fn run_sync_impl_stops_on_ingest_failure() {
        let root = TempDir::new().unwrap();

        let move_script = root.path().join("move.sh");
        let register_script = root.path().join("register.sh");

        write_executable(
            &move_script,
            r#"#!/usr/bin/env bash
set -euo pipefail
exit 3
"#,
        );

        write_executable(
            &register_script,
            r#"#!/usr/bin/env bash
set -euo pipefail
touch "$(dirname "$0")/registered"
"#,
        );

        let err = run_sync_impl(&move_script, &register_script).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::Other);
        assert!(!root.path().join("registered").exists());
    }
}
