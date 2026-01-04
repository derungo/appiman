 mod clean;
 mod config;
 mod core;
 mod ingest;
 mod mover;
 mod privileges;
 mod registrar;
 mod scan;
 mod security;
 mod setup;
 mod status;
 mod sync;
 mod systemd;
 mod update;

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
        Some("update") => run_update(),
        Some("versions") => run_versions(),
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

fn run_update() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    let apply = args.iter().any(|a| a == "--apply");
    let dry_run = args.iter().any(|a| a == "--dry-run");
    let rollback = args.iter().find(|a| a.starts_with("--rollback="));
    let switch = args.iter().find(|a| a.starts_with("--switch="));

    if let Some(rollback_arg) = rollback {
        if let Some(app_name) = rollback_arg.strip_prefix("--rollback=") {
            match update::run_rollback(app_name) {
                Ok(()) => ExitCode::SUCCESS,
                Err(e) => {
                    eprintln!("❌ Rollback failed: {}", e);
                    ExitCode::FAILURE
                }
            }
        } else {
            eprintln!("❌ Invalid rollback syntax. Use --rollback=<app_name>");
            ExitCode::from(2)
        }
    } else if let Some(switch_arg) = switch {
        if let Some(app_version) = switch_arg.strip_prefix("--switch=") {
            if let Some((app_name, version)) = app_version.split_once(':') {
                match run_switch_version(app_name, version) {
                    Ok(()) => ExitCode::SUCCESS,
                    Err(e) => {
                        eprintln!("❌ Version switch failed: {}", e);
                        ExitCode::FAILURE
                    }
                }
            } else {
                eprintln!("❌ Invalid switch syntax. Use --switch=<app_name>:<version>");
                ExitCode::from(2)
            }
        } else {
            eprintln!("❌ Invalid switch syntax. Use --switch=<app_name>:<version>");
            ExitCode::from(2)
        }
    } else if apply {
        match update::run_update_apply(dry_run) {
            Ok(()) => ExitCode::SUCCESS,
            Err(e) => {
                eprintln!("❌ Update failed: {}", e);
                ExitCode::FAILURE
            }
        }
    } else {
        match update::run_update_check() {
            Ok(()) => ExitCode::SUCCESS,
            Err(e) => {
                eprintln!("❌ Update check failed: {}", e);
                ExitCode::FAILURE
            }
        }
    }
}

fn run_versions() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    let subcommand = args.get(2).map(|s| s.as_str());

    match subcommand {
        Some("list") => {
            let app_name = args.get(3).map(|s| s.as_str());
            match run_list_versions(app_name) {
                Ok(()) => ExitCode::SUCCESS,
                Err(e) => {
                    eprintln!("❌ Failed to list versions: {}", e);
                    ExitCode::FAILURE
                }
            }
        }
        Some("switch") => {
            if let (Some(app_name), Some(version)) = (args.get(3), args.get(4)) {
                match run_switch_version(app_name, version) {
                    Ok(()) => ExitCode::SUCCESS,
                    Err(e) => {
                        eprintln!("❌ Version switch failed: {}", e);
                        ExitCode::FAILURE
                    }
                }
            } else {
                eprintln!("❌ Usage: appiman versions switch <app_name> <version>");
                ExitCode::from(2)
            }
        }
        Some("remove") => {
            if let (Some(app_name), Some(version)) = (args.get(3), args.get(4)) {
                match run_remove_version(app_name, version) {
                    Ok(()) => ExitCode::SUCCESS,
                    Err(e) => {
                        eprintln!("❌ Version removal failed: {}", e);
                        ExitCode::FAILURE
                    }
                }
            } else {
                eprintln!("❌ Usage: appiman versions remove <app_name> <version>");
                ExitCode::from(2)
            }
        }
        _ => {
            println!("Usage: appiman versions <subcommand>");
            println!();
            println!("Subcommands:");
            println!("  list [app_name]    - List versions for an app (or all apps)");
            println!("  switch <app> <ver> - Switch app to specified version");
            println!("  remove <app> <ver> - Remove a specific version");
            ExitCode::from(2)
        }
    }
}

fn run_list_versions(app_name: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
    use crate::core::VersionManager;

    let config = crate::config::Config::load()?;
    let version_manager = VersionManager::new(config);

    if let Some(app) = app_name {
        let versions = version_manager.list_versions(app)?;
        println!("Versions for {}:", app);
        for version in versions {
            let active = if version.is_active { " (active)" } else { "" };
            println!("  {}{} - installed {}", version.version, active, version.installed_at.format("%Y-%m-%d %H:%M:%S"));
        }
    } else {
        let apps = version_manager.list_apps()?;
        println!("Registered applications:");
        for app in apps {
            let current = version_manager.get_current_version(&app)?.unwrap_or_else(|| "none".to_string());
            println!("  {} -> {}", app, current);
        }
    }

    Ok(())
}

fn run_switch_version(app_name: &str, version: &str) -> Result<(), Box<dyn std::error::Error>> {
    use crate::core::VersionManager;

    let config = crate::config::Config::load()?;
    let version_manager = VersionManager::new(config);
    version_manager.switch_version(app_name, version)?;
    println!("✅ Switched {} to version {}", app_name, version);
    Ok(())
}

fn run_remove_version(app_name: &str, version: &str) -> Result<(), Box<dyn std::error::Error>> {
    use crate::core::VersionManager;

    let config = crate::config::Config::load()?;
    let version_manager = VersionManager::new(config);
    version_manager.remove_version(app_name, version)?;
    println!("✅ Removed {} version {}", app_name, version);
    Ok(())
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
    println!("  update   - Check for and apply AppImage updates");
    println!("  versions - Manage AppImage versions");
    println!("  clean    - Remove legacy AppImages and artifacts");
    println!("  help     - Show this help message");
    println!();
    println!("Options:");
    println!("  --json        - Output status in JSON format");
    println!("  --apply       - Apply available updates (with update command)");
    println!("  --dry-run     - Show what would be done without making changes");
    println!("  --rollback=<name> - Rollback specified AppImage to previous version");
    println!("  --switch=<app>:<version> - Switch specified AppImage to a different version");
    println!();
    println!("For more information, visit: https://github.com/derungo/appiman");
}
