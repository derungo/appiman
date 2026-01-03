// src/scan.rs

use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

const DEFAULT_REGISTER_SCRIPT: &str = "/usr/local/sbin/register-appimages.sh";

fn register_script_path() -> PathBuf {
    std::env::var_os("APPIMAN_REGISTER_SCRIPT")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_REGISTER_SCRIPT))
}

pub(crate) fn run_register_script(script_path: &Path) -> io::Result<()> {
    if !script_path.exists() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("Registration script not found: {}", script_path.display()),
        ));
    }

    let status = Command::new(script_path).status()?;

    if status.success() {
        Ok(())
    } else {
        Err(io::Error::new(
            io::ErrorKind::Other,
            format!("Registration script exited with {}", status),
        ))
    }
}

pub fn run_scan() -> io::Result<()> {
    println!("🔄 Triggering full AppImage re-registration...");

    let script_path = register_script_path();
    run_register_script(&script_path)?;

    println!("✅ Re-registration complete.");
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
    fn run_register_script_errors_when_missing() {
        let root = TempDir::new().unwrap();
        let script = root.path().join("missing.sh");

        let err = run_register_script(&script).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::NotFound);
    }

    #[cfg(unix)]
    #[test]
    fn run_register_script_succeeds_on_zero_exit() {
        let root = TempDir::new().unwrap();
        let script = root.path().join("register.sh");
        write_executable(&script, "#!/usr/bin/env bash\nexit 0\n");

        run_register_script(&script).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn run_register_script_errors_on_nonzero_exit() {
        let root = TempDir::new().unwrap();
        let script = root.path().join("register.sh");
        write_executable(&script, "#!/usr/bin/env bash\nexit 42\n");

        let err = run_register_script(&script).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::Other);
    }
}
