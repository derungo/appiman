mod clean;
mod config;
mod core;
mod ingest;
mod log;
mod logger;
mod mover;
mod privileges;
mod registrar;
mod scan;
mod setup;
mod status;
mod sync;
mod systemd;

use std::env;
use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    let command = args.get(1).map(|s| s.as_str());
    let json_output = args.iter().any(|a| a == "--json");

    match command {
        None | Some("help") | Some("-h") | Some("--help") => {
            print_help();
            ExitCode::SUCCESS
        }
        Some("init") => run_and_report(setup::initialize),
        Some("enable") => run_and_report(systemd::enable_all),
        Some("disable") => run_and_report(systemd::disable_all),
        Some("status") => {
            if json_output {
                match status::StatusReporter::new() {
                    Ok(reporter) => match reporter.print_status(true) {
                        Ok(()) => ExitCode::SUCCESS,
                        Err(e) => {
                            eprintln!("❌ {}", e);
                            ExitCode::FAILURE
                        }
                    },
                    Err(e) => {
                        eprintln!("❌ Failed to create status reporter: {}", e);
                        ExitCode::FAILURE
                    }
                }
            } else {
                run_and_report(systemd::print_status)
            }
        }
        Some("clean") => run_and_report(clean::run_cleanup),
        Some("ingest") => run_and_report(ingest::run_ingest),
        Some("scan") => run_and_report(scan::run_scan),
        Some("sync") => run_and_report(sync::run_sync),
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
    println!("Usage: appiman <command> [options]");
    println!();
    println!("Commands:");
    println!("  init     - Create dir structure and install units/scripts");
    println!("  enable   - Enable and start systemd .path units");
    println!("  disable  - Disable and stop systemd .path units");
    println!("  status   - Show systemd status of watchers and AppImage inventory");
    println!("  ingest   - Move user-downloaded AppImages into staging");
    println!("  scan     - Run AppImage re-index manually");
    println!("  sync     - Ingest then register AppImages");
    println!("  clean    - Remove legacy AppImages and artifacts");
    println!("  help     - Show this help message");
    println!();
    println!("Options:");
    println!("  --json   - Output status in JSON format");
    println!();
    println!("For more information, visit: https://github.com/derungo/appiman");
}
