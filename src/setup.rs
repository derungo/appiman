// src/setup.rs

use std::fs;
use std::path::PathBuf;
use std::process::Command;
use nix::unistd::Uid;

const DIRS: &[&str] = &[
    "/opt/applications/raw",
    "/opt/applications/bin",
    "/opt/applications/icons",
];

const SCRIPT_NAMES: &[&str] = &[
    "register-appimages.sh",
    "move-appimages.sh",
];

const UNIT_NAMES: &[&str] = &[
    "register-appimages.service",
    "register-appimages.path",
    "move-appimages.service",
    "move-appimages.path",
];

pub fn initialize() {
    println!("🔧 Initializing AppImage management system...");

    if !Uid::effective().is_root() {
        eprintln!("❌ This command must be run with sudo/root.");
        std::process::exit(1);
    }

    for dir in DIRS {
        println!("Creating directory: {}", dir);
        if let Err(e) = fs::create_dir_all(dir) {
            eprintln!("  ⚠️ Failed to create {}: {}", dir, e);
        }
    }

    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."));

    for script in SCRIPT_NAMES {
        let src = exe_dir.join("assets").join(script);
        let dst = PathBuf::from("/usr/local/sbin").join(script);
        println!("Installing script: {} → {}", src.display(), dst.display());
        if let Err(e) = fs::copy(&src, &dst) {
            eprintln!("  ⚠️ Failed to copy script {}: {}", src.display(), e);
        } else {
            let _ = Command::new("chmod").arg("+x").arg(&dst).status();
        }
    }

    for unit in UNIT_NAMES {
        let src = exe_dir.join("assets").join(unit);
        let dst = PathBuf::from("/etc/systemd/system").join(unit);
        println!("Installing unit: {} → {}", src.display(), dst.display());
        if let Err(e) = fs::copy(&src, &dst) {
            eprintln!("  ⚠️ Failed to install unit {}: {}", src.display(), e);
        }
    }

    println!("✅ Initialization complete. Run `appiman enable` to activate services.");
}