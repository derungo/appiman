// src/systemd.rs

use std::process::Command;
use nix::unistd::Uid;

pub fn enable_all() {
    if !Uid::effective().is_root() {
        eprintln!("❌ This command must be run with sudo/root.");
        std::process::exit(1);
    }

    let units = [
        "register-appimages.path",
        "move-appimages.path",
    ];

    for unit in &units {
        println!("Enabling and starting: {}", unit);
        let _ = Command::new("systemctl").args(["enable", "--now", unit]).status();
    }

    println!("✅ All .path units enabled and started.");
}

pub fn print_status() {
    let units = [
        "register-appimages.path",
        "move-appimages.path",
        "register-appimages.service",
        "move-appimages.service",
    ];

    for unit in &units {
        println!("\n🔍 Status for: {}", unit);
        let _ = Command::new("systemctl").arg("status").arg(unit).status();
    }
}