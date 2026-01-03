// src/setup.rs

use crate::privileges::require_root;
use std::fs;
use std::io;
use std::path::Path;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

const APP_SUBDIRS: &[&str] = &["raw", "bin", "icons"];

const REGISTER_APPIMAGES_SH: &str = include_str!("../assets/register-appimages.sh");
const MOVE_APPIMAGES_SH: &str = include_str!("../assets/move-appimages.sh");

const REGISTER_APPIMAGES_SERVICE: &str = include_str!("../assets/register-appimages.service");
const REGISTER_APPIMAGES_PATH: &str = include_str!("../assets/register-appimages.path");
const MOVE_APPIMAGES_SERVICE: &str = include_str!("../assets/move-appimages.service");
const MOVE_APPIMAGES_PATH: &str = include_str!("../assets/move-appimages.path");

const SCRIPT_ASSETS: &[(&str, &str)] = &[
    ("register-appimages.sh", REGISTER_APPIMAGES_SH),
    ("move-appimages.sh", MOVE_APPIMAGES_SH),
];

const UNIT_ASSETS: &[(&str, &str)] = &[
    ("register-appimages.service", REGISTER_APPIMAGES_SERVICE),
    ("register-appimages.path", REGISTER_APPIMAGES_PATH),
    ("move-appimages.service", MOVE_APPIMAGES_SERVICE),
    ("move-appimages.path", MOVE_APPIMAGES_PATH),
];

#[cfg(unix)]
fn set_mode(path: &Path, mode: u32) -> io::Result<()> {
    let mut perms = fs::metadata(path)?.permissions();
    perms.set_mode(mode);
    fs::set_permissions(path, perms)
}

#[cfg(not(unix))]
fn set_mode(_path: &Path, _mode: u32) -> io::Result<()> {
    Ok(())
}

fn initialize_impl(base_dir: &Path, script_dir: &Path, unit_dir: &Path) -> io::Result<()> {
    for subdir in APP_SUBDIRS {
        let dir = base_dir.join(subdir);
        println!("Creating directory: {}", dir.display());
        fs::create_dir_all(dir)?;
    }

    fs::create_dir_all(script_dir)?;
    fs::create_dir_all(unit_dir)?;

    for (name, contents) in SCRIPT_ASSETS {
        let dst = script_dir.join(name);
        println!("Installing script: {} → {}", name, dst.display());
        fs::write(&dst, contents)?;
        set_mode(&dst, 0o755)?;
    }

    for (name, contents) in UNIT_ASSETS {
        let dst = unit_dir.join(name);
        println!("Installing unit: {} → {}", name, dst.display());
        fs::write(&dst, contents)?;
        set_mode(&dst, 0o644)?;
    }

    Ok(())
}

pub fn initialize() -> io::Result<()> {
    println!("🔧 Initializing AppImage management system...");

    require_root()?;

    let base_dir = Path::new("/opt/applications");
    let script_dir = Path::new("/usr/local/sbin");
    let unit_dir = Path::new("/etc/systemd/system");

    initialize_impl(base_dir, script_dir, unit_dir)?;

    println!("✅ Initialization complete. Run `appiman enable` to activate services.");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    use tempfile::TempDir;

    #[test]
    fn initialize_impl_creates_dirs_and_installs_assets() {
        let root = TempDir::new().unwrap();

        let base_dir = root.path().join("opt/applications");
        let script_dir = root.path().join("usr/local/sbin");
        let unit_dir = root.path().join("etc/systemd/system");

        initialize_impl(&base_dir, &script_dir, &unit_dir).unwrap();

        for subdir in APP_SUBDIRS {
            assert!(base_dir.join(subdir).is_dir(), "missing {subdir} dir");
        }

        for (name, contents) in SCRIPT_ASSETS {
            let path = script_dir.join(name);
            assert!(path.is_file(), "missing script {}", path.display());
            assert_eq!(fs::read_to_string(&path).unwrap(), *contents);
        }

        for (name, contents) in UNIT_ASSETS {
            let path = unit_dir.join(name);
            assert!(path.is_file(), "missing unit {}", path.display());
            assert_eq!(fs::read_to_string(&path).unwrap(), *contents);
        }
    }

    #[cfg(unix)]
    #[test]
    fn installed_scripts_are_executable_and_units_are_not() {
        let root = TempDir::new().unwrap();

        let base_dir = root.path().join("opt/applications");
        let script_dir = root.path().join("usr/local/sbin");
        let unit_dir = root.path().join("etc/systemd/system");

        initialize_impl(&base_dir, &script_dir, &unit_dir).unwrap();

        for (name, _) in SCRIPT_ASSETS {
            let mode = fs::metadata(script_dir.join(name))
                .unwrap()
                .permissions()
                .mode()
                & 0o777;
            assert_eq!(mode, 0o755, "script {name} mode was {mode:o}");
        }

        for (name, _) in UNIT_ASSETS {
            let mode = fs::metadata(unit_dir.join(name))
                .unwrap()
                .permissions()
                .mode()
                & 0o777;
            assert_eq!(mode, 0o644, "unit {name} mode was {mode:o}");
        }
    }
}
