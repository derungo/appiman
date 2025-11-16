// src/clean.rs

use std::fs;
use regex::Regex;
use nix::unistd::Uid;

const BIN_DIR: &str = "/opt/applications/bin";
const SYMLINK_DIR: &str = "/usr/local/bin";
const DESKTOP_DIR: &str = "/usr/share/applications";
const ICON_DIR: &str = "/opt/applications/icons";

pub fn run_cleanup() {
    if !Uid::effective().is_root() {
        eprintln!("❌ This command must be run as root.");
        std::process::exit(1);
    }

    println!("🧹 Cleaning up legacy AppImage files and artifacts...");

    let re = Regex::new(r"(?i)(-v[\d\.]+|[-_.]?(x86_64|amd64|linux|i386|setup))").unwrap();

    // Clean bin directory
    if let Ok(entries) = fs::read_dir(BIN_DIR) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            if re.is_match(&name) {
                let _ = fs::remove_file(entry.path());
                println!("Removed bin entry: {}", name);
            }
        }
    }

    // Clean broken or legacy symlinks
    if let Ok(entries) = fs::read_dir(SYMLINK_DIR) {
        for entry in entries.flatten() {
            let path = entry.path();
            if let Ok(target) = fs::read_link(&path) {
                if !target.exists() || re.is_match(&target.to_string_lossy()) {
                    let _ = fs::remove_file(&path);
                    println!("Removed symlink: {}", path.display());
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
                    if content.contains("/opt/applications/bin/") && re.is_match(&content) {
                        let _ = fs::remove_file(&path);
                        println!("Removed desktop entry: {}", path.display());
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
                let _ = fs::remove_file(entry.path());
                println!("Removed icon: {}", name);
            }
        }
    }

    println!("✅ Cleanup complete.");
}
