// src/systemd.rs

use crate::privileges::require_root;
use std::io;
use std::process::Command;

const PATH_UNITS: &[&str] = &["register-appimages.path", "move-appimages.path"];

const STATUS_UNITS: &[&str] = &[
    "register-appimages.path",
    "move-appimages.path",
    "register-appimages.service",
    "move-appimages.service",
];

fn systemctl_bin() -> String {
    std::env::var("APPIMAN_SYSTEMCTL").unwrap_or_else(|_| "systemctl".to_string())
}

fn enable_units(systemctl: &str, units: &[&str]) -> io::Result<()> {
    let mut failures = Vec::new();

    for unit in units {
        println!("Enabling and starting: {}", unit);
        let status = Command::new(systemctl)
            .args(["enable", "--now", unit])
            .status()?;

        if !status.success() {
            failures.push((*unit).to_string());
            eprintln!("⚠️ systemctl enable --now {} exited with {}", unit, status);
        }
    }

    if !failures.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            format!("Failed to enable/start: {}", failures.join(", ")),
        ));
    }

    Ok(())
}

fn disable_units(systemctl: &str, units: &[&str]) -> io::Result<()> {
    let mut failures = Vec::new();

    for unit in units {
        println!("Disabling and stopping: {}", unit);
        let status = Command::new(systemctl)
            .args(["disable", "--now", unit])
            .status()?;

        if !status.success() {
            failures.push((*unit).to_string());
            eprintln!("⚠️ systemctl disable --now {} exited with {}", unit, status);
        }
    }

    if !failures.is_empty() {
        return Err(io::Error::other(format!(
            "Failed to disable/stop: {}",
            failures.join(", ")
        )));
    }

    Ok(())
}

pub fn enable_all() -> io::Result<()> {
    require_root()?;

    let systemctl = systemctl_bin();
    enable_units(&systemctl, PATH_UNITS)?;
    println!("✅ All .path units enabled and started.");
    Ok(())
}

pub fn disable_all() -> io::Result<()> {
    require_root()?;

    let systemctl = systemctl_bin();
    disable_units(&systemctl, PATH_UNITS)?;
    println!("✅ All .path units disabled and stopped.");
    Ok(())
}

pub fn print_status() -> io::Result<()> {
    let systemctl = systemctl_bin();

    for unit in STATUS_UNITS {
        println!("\n🔍 Status for: {}", unit);
        let _status = Command::new(&systemctl)
            .args(["status", "--no-pager", unit])
            .status()?;
    }

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
    fn enable_units_succeeds_when_systemctl_returns_zero() {
        let root = TempDir::new().unwrap();
        let systemctl = root.path().join("systemctl");
        write_executable(
            &systemctl,
            r#"#!/usr/bin/env bash
set -euo pipefail
log="$(dirname "$0")/calls.log"
echo "$*" >> "$log"
exit 0
"#,
        );

        enable_units(systemctl.to_str().unwrap(), &["a.path", "b.path"]).unwrap();

        let calls = fs::read_to_string(root.path().join("calls.log")).unwrap();
        assert!(calls.contains("enable --now a.path"));
        assert!(calls.contains("enable --now b.path"));
    }

    #[cfg(unix)]
    #[test]
    fn enable_units_reports_failure_but_runs_all_units() {
        let root = TempDir::new().unwrap();
        let systemctl = root.path().join("systemctl");
        write_executable(
            &systemctl,
            r#"#!/usr/bin/env bash
set -euo pipefail
log="$(dirname "$0")/calls.log"
echo "$*" >> "$log"
if [[ "${3:-}" == "b.path" ]]; then
  exit 1
fi
exit 0
"#,
        );

        let err = enable_units(systemctl.to_str().unwrap(), &["a.path", "b.path"]).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::Other);

        let calls = fs::read_to_string(root.path().join("calls.log")).unwrap();
        assert!(calls.contains("enable --now a.path"));
        assert!(calls.contains("enable --now b.path"));
    }

    #[cfg(unix)]
    #[test]
    fn disable_units_succeeds_when_systemctl_returns_zero() {
        let root = TempDir::new().unwrap();
        let systemctl = root.path().join("systemctl");
        write_executable(
            &systemctl,
            r#"#!/usr/bin/env bash
set -euo pipefail
log="$(dirname "$0")/calls.log"
echo "$*" >> "$log"
exit 0
"#,
        );

        disable_units(systemctl.to_str().unwrap(), &["a.path", "b.path"]).unwrap();

        let calls = fs::read_to_string(root.path().join("calls.log")).unwrap();
        assert!(calls.contains("disable --now a.path"));
        assert!(calls.contains("disable --now b.path"));
    }

    #[cfg(unix)]
    #[test]
    fn disable_units_reports_failure_but_runs_all_units() {
        let root = TempDir::new().unwrap();
        let systemctl = root.path().join("systemctl");
        write_executable(
            &systemctl,
            r#"#!/usr/bin/env bash
set -euo pipefail
log="$(dirname "$0")/calls.log"
echo "$*" >> "$log"
if [[ "${3:-}" == "b.path" ]]; then
  exit 1
fi
exit 0
"#,
        );

        let err = disable_units(systemctl.to_str().unwrap(), &["a.path", "b.path"]).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::Other);

        let calls = fs::read_to_string(root.path().join("calls.log")).unwrap();
        assert!(calls.contains("disable --now a.path"));
        assert!(calls.contains("disable --now b.path"));
    }
}
