mod clean;
mod privileges;
mod scan;
mod setup;
mod systemd;

use std::env;
use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    let command = args.get(1).map(|s| s.as_str());

    match command {
        None | Some("help") | Some("-h") | Some("--help") => {
            print_help();
            ExitCode::SUCCESS
        }
        Some("init") => run_and_report(setup::initialize),
        Some("enable") => run_and_report(systemd::enable_all),
        Some("status") => run_and_report(systemd::print_status),
        Some("clean") => run_and_report(clean::run_cleanup),
        Some("scan") => run_and_report(scan::run_scan),
        Some(other) => {
            eprintln!("❌ Unknown command: {}", other);
            print_help();
            ExitCode::from(2)
        }
    }
}

fn run_and_report(f: fn() -> std::io::Result<()>) -> ExitCode {
    match f() {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("❌ {}", err);
            ExitCode::FAILURE
        }
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
