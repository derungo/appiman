// src/log.rs

use std::process::Command;

const UNITS: &[&str] = &["register-appimages.service", "move-appimages.service"];

pub fn tail_logs() {
    for unit in UNITS {
        println!("\n📜 Recent logs for {}:", unit);
        let output = Command::new("journalctl")
            .args(["-u", unit, "--no-pager", "--since=1h"])
            .output();

        match output {
            Ok(out) => {
                let log = String::from_utf8_lossy(&out.stdout);
                println!("{}", log);
            }
            Err(e) => eprintln!("❌ Failed to read logs for {}: {}", unit, e),
        }
    }
}
