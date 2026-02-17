// src/setup.rs

use crate::privileges::require_root;
use std::fs;
use std::io;
use std::path::Path;
use std::path::PathBuf;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

const APP_SUBDIRS: &[&str] = &["raw", "bin", "icons"];

const REGISTER_APPIMAGES_SERVICE: &str = include_str!("../assets/register-appimages.service");
const REGISTER_APPIMAGES_PATH: &str = include_str!("../assets/register-appimages.path");
const MOVE_APPIMAGES_SERVICE: &str = include_str!("../assets/move-appimages.service");
const MOVE_APPIMAGES_PATH: &str = include_str!("../assets/move-appimages.path");
const MOVE_APPIMAGES_TIMER: &str = include_str!("../assets/move-appimages.timer");

const UNIT_ASSETS: &[(&str, &str)] = &[
    ("register-appimages.service", REGISTER_APPIMAGES_SERVICE),
    ("register-appimages.path", REGISTER_APPIMAGES_PATH),
    ("move-appimages.service", MOVE_APPIMAGES_SERVICE),
    ("move-appimages.path", MOVE_APPIMAGES_PATH),
    ("move-appimages.timer", MOVE_APPIMAGES_TIMER),
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

fn install_source_executable() -> io::Result<PathBuf> {
    if let Some(path) = std::env::var_os("APPIMAGE") {
        return Ok(PathBuf::from(path));
    }

    std::env::current_exe()
}

fn install_appiman_binary(bin_dir: &Path) -> io::Result<()> {
    let source = install_source_executable()?;
    let dest = bin_dir.join("appiman");

    println!(
        "Installing appiman binary: {} -> {}",
        source.display(),
        dest.display()
    );

    if source != dest {
        fs::copy(&source, &dest)?;
    }

    set_mode(&dest, 0o755)?;
    Ok(())
}

fn initialize_impl(base_dir: &Path, bin_dir: &Path, unit_dir: &Path) -> io::Result<()> {
    for subdir in APP_SUBDIRS {
        let dir = base_dir.join(subdir);
        println!("Creating directory: {}", dir.display());
        fs::create_dir_all(dir)?;
    }

    fs::create_dir_all(bin_dir)?;
    fs::create_dir_all(unit_dir)?;

    install_appiman_binary(bin_dir)?;

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
    let bin_dir = Path::new("/usr/local/bin");
    let unit_dir = Path::new("/etc/systemd/system");

    initialize_impl(base_dir, bin_dir, unit_dir)?;

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
        let bin_dir = root.path().join("usr/local/bin");
        let unit_dir = root.path().join("etc/systemd/system");

        initialize_impl(&base_dir, &bin_dir, &unit_dir).unwrap();

        for subdir in APP_SUBDIRS {
            assert!(base_dir.join(subdir).is_dir(), "missing {subdir} dir");
        }

        assert!(
            bin_dir.join("appiman").is_file(),
            "missing installed appiman binary"
        );

        for (name, contents) in UNIT_ASSETS {
            let path = unit_dir.join(name);
            assert!(path.is_file(), "missing unit {}", path.display());
            assert_eq!(fs::read_to_string(&path).unwrap(), *contents);
        }
    }

    #[cfg(unix)]
    #[test]
    fn installed_units_have_correct_permissions() {
        let root = TempDir::new().unwrap();

        let base_dir = root.path().join("opt/applications");
        let bin_dir = root.path().join("usr/local/bin");
        let unit_dir = root.path().join("etc/systemd/system");

        initialize_impl(&base_dir, &bin_dir, &unit_dir).unwrap();

        for (name, _) in UNIT_ASSETS {
            let mode = fs::metadata(unit_dir.join(name))
                .unwrap()
                .permissions()
                .mode()
                & 0o777;
            assert_eq!(mode, 0o644, "unit {name} mode was {mode:o}");
        }

        let appiman_mode = fs::metadata(bin_dir.join("appiman"))
            .unwrap()
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(appiman_mode, 0o755, "appiman mode was {appiman_mode:o}");
    }
}
