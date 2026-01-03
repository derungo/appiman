// src/ingest.rs

use crate::privileges::require_root;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

const DEFAULT_MOVE_SCRIPT: &str = "/usr/local/sbin/move-appimages.sh";

fn move_script_path() -> PathBuf {
    std::env::var_os("APPIMAN_MOVE_SCRIPT")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_MOVE_SCRIPT))
}

fn run_move_script(script_path: &Path) -> io::Result<()> {
    if !script_path.exists() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("Mover script not found: {}", script_path.display()),
        ));
    }

    let status = Command::new(script_path).status()?;

    if status.success() {
        Ok(())
    } else {
        Err(io::Error::new(
            io::ErrorKind::Other,
            format!("Mover script exited with {}", status),
        ))
    }
}

pub fn run_ingest() -> io::Result<()> {
    require_root()?;

    println!("📥 Ingesting user-downloaded AppImages...");

    let script_path = move_script_path();
    run_move_script(&script_path)?;

    println!("✅ Ingest complete.");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

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

    #[test]
    fn run_move_script_errors_when_missing() {
        let root = TempDir::new().unwrap();
        let script = root.path().join("missing.sh");

        let err = run_move_script(&script).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::NotFound);
    }

    #[cfg(unix)]
    #[test]
    fn run_move_script_succeeds_on_zero_exit() {
        let root = TempDir::new().unwrap();
        let script = root.path().join("move.sh");
        write_executable(&script, "#!/usr/bin/env bash\nexit 0\n");

        run_move_script(&script).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn run_move_script_errors_on_nonzero_exit() {
        let root = TempDir::new().unwrap();
        let script = root.path().join("move.sh");
        write_executable(&script, "#!/usr/bin/env bash\nexit 7\n");

        let err = run_move_script(&script).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::Other);
    }
}
