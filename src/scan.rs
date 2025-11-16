// src/scan.rs

use std::process::Command;

pub fn run_scan() {
    println!("🔄 Triggering full AppImage re-registration...");

    let status = Command::new("/usr/local/sbin/register-appimages.sh")
        .status();

    match status {
        Ok(code) if code.success() => println!("✅ Re-registration complete."),
        Ok(code) => eprintln!("⚠️ Script exited with code: {}", code),
        Err(e) => eprintln!("❌ Failed to run registration script: {}", e),
    }
}