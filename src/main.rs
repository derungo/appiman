mod setup;
mod systemd;
mod clean;
mod scan;

use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();
    match args.get(1).map(|s| s.as_str()) {
        Some("init") => setup::initialize(),
        Some("enable") => systemd::enable_all(),
        Some("status") => systemd::print_status(),
        Some("clean") => clean::run_cleanup(),
        Some("scan") => scan::run_scan(),
        Some("help") | _ => print_help(),
    }
}

fn print_help() {
    println!("Usage: appiman <command>");
    println!("Commands:");
    println!("  init     - Create dir structure and install units/scripts");
    println!("  enable   - Enable and start systemd .path units");
    println!("  status   - Show systemd status of watchers");
    println!("  scan     - Run AppImage re-index manually");
    println!("  clean    - Remove legacy AppImages and artifacts");
    println!("  help     - Show this help message");
}
