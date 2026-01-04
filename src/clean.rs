// src/clean.rs

use crate::config::Config;
use crate::privileges::require_root;
use regex::Regex;
use std::fs;
use std::io;

lazy_static::lazy_static! {
    static ref CLEAN_REGEX: Regex = Regex::new(
        r"(?i)(-v[\d\.]+|[-_.]?(x86_64|amd64|linux|i386|setup))"
    ).unwrap();
}

pub fn run_cleanup() -> io::Result<()> {
    require_root()?;

    let config = Config::load().map_err(|e| {
        io::Error::other(
            format!("Failed to load config: {}", e),
        )
    })?;

    let bin_dir = config.bin_dir();
    let symlink_dir = config.symlink_dir();
    let desktop_dir = config.desktop_dir();
    let icon_dir = config.icon_dir();

    println!("🧹 Cleaning up legacy AppImage files and artifacts...");

    let re = &CLEAN_REGEX;

    let mut had_errors = false;

    // Clean bin directory
    if let Ok(entries) = fs::read_dir(&bin_dir) {
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
    if let Ok(entries) = fs::read_dir(&symlink_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if let Ok(target) = fs::read_link(&path)
                && (!target.exists() || re.is_match(&target.to_string_lossy())) {
                    if let Err(err) = fs::remove_file(&path) {
                        had_errors = true;
                        eprintln!("⚠️ Failed to remove symlink {}: {}", path.display(), err);
                    } else {
                        println!("Removed symlink: {}", path.display());
                    }
                }
        }
    }

    // Clean legacy .desktop entries
    if let Ok(entries) = fs::read_dir(&desktop_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().is_some_and(|e| e == "desktop")
                && let Ok(content) = fs::read_to_string(&path)
                    && content.contains(bin_dir.to_string_lossy().as_ref()) && re.is_match(&content)
                    {
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

    // Clean stale icons
    if let Ok(entries) = fs::read_dir(&icon_dir) {
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
        return Err(io::Error::other(
            "Cleanup completed with errors.",
        ));
    }

    println!("✅ Cleanup complete.");
    Ok(())
}
