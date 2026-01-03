// src/clean.rs

use crate::privileges::require_root;
use regex::Regex;
use std::fs;
use std::io;

const BIN_DIR: &str = "/opt/applications/bin";
const SYMLINK_DIR: &str = "/usr/local/bin";
const DESKTOP_DIR: &str = "/usr/share/applications";
const ICON_DIR: &str = "/opt/applications/icons";

pub fn run_cleanup() -> io::Result<()> {
    require_root()?;

    println!("🧹 Cleaning up legacy AppImage files and artifacts...");

    let re = Regex::new(r"(?i)(-v[\d\.]+|[-_.]?(x86_64|amd64|linux|i386|setup))").unwrap();

    let mut had_errors = false;

    // Clean bin directory
    if let Ok(entries) = fs::read_dir(BIN_DIR) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            if re.is_match(&name) {
                if let Err(err) = fs::remove_file(entry.path()) {
                    had_errors = true;
                    eprintln!("⚠️ Failed to remove bin entry {}: {}", name, err);
                } else {
                    println!("Removed bin entry: {}", name);
                }
            }
        }
    }

    // Clean broken or legacy symlinks
    if let Ok(entries) = fs::read_dir(SYMLINK_DIR) {
        for entry in entries.flatten() {
            let path = entry.path();
            if let Ok(target) = fs::read_link(&path) {
                if !target.exists() || re.is_match(&target.to_string_lossy()) {
                    if let Err(err) = fs::remove_file(&path) {
                        had_errors = true;
                        eprintln!("⚠️ Failed to remove symlink {}: {}", path.display(), err);
                    } else {
                        println!("Removed symlink: {}", path.display());
                    }
                }
            }
        }
    }

    // Clean legacy .desktop entries
    if let Ok(entries) = fs::read_dir(DESKTOP_DIR) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().map(|e| e == "desktop").unwrap_or(false) {
                if let Ok(content) = fs::read_to_string(&path) {
                    if content.contains(BIN_DIR) && re.is_match(&content) {
                        if let Err(err) = fs::remove_file(&path) {
                            had_errors = true;
                            eprintln!(
                                "⚠️ Failed to remove desktop entry {}: {}",
                                path.display(),
                                err
                            );
                        } else {
                            println!("Removed desktop entry: {}", path.display());
                        }
                    }
                }
            }
        }
    }

    // Clean stale icons
    if let Ok(entries) = fs::read_dir(ICON_DIR) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            if re.is_match(&name) {
                if let Err(err) = fs::remove_file(entry.path()) {
                    had_errors = true;
                    eprintln!("⚠️ Failed to remove icon {}: {}", name, err);
                } else {
                    println!("Removed icon: {}", name);
                }
            }
        }
    }

    if had_errors {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            "Cleanup completed with errors.",
        ));
    }

    println!("✅ Cleanup complete.");
    Ok(())
}
