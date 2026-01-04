use crate::config::Config;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::Path;
use std::process::Command;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum StatusError {
    #[error("IO error: {0}")]
    Io(#[from] io::Error),

    #[error("Systemd error: {0}")]
    SystemdError(String),

    #[error("Config error: {0}")]
    ConfigError(#[from] crate::config::ConfigError),

    #[error("JSON error: {0}")]
    JsonError(String),
}

impl From<StatusError> for std::io::Error {
    fn from(err: StatusError) -> Self {
        std::io::Error::new(std::io::ErrorKind::Other, err.to_string())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppImageStatus {
    pub name: String,
    pub version: String,
    pub path: String,
    pub size_bytes: u64,
    pub registered_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemStatus {
    pub systemd_units: Vec<UnitStatus>,
    pub registered_appimages: Vec<AppImageStatus>,
    pub storage_usage: StorageUsage,
    pub last_scan: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnitStatus {
    pub name: String,
    pub loaded: bool,
    pub enabled: bool,
    pub active: bool,
    pub state: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageUsage {
    pub bin_dir: DirectoryUsage,
    pub raw_dir: DirectoryUsage,
    pub icon_dir: DirectoryUsage,
    pub total_size_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirectoryUsage {
    pub path: String,
    pub file_count: u64,
    pub size_bytes: u64,
}

pub struct StatusReporter {
    config: Config,
}

impl StatusReporter {
    pub fn new() -> Result<Self, StatusError> {
        let config = Config::load()?;
        Ok(StatusReporter { config })
    }

    fn get_status(&self) -> Result<SystemStatus, StatusError> {
        let systemd_units = self.get_systemd_status()?;
        let registered_appimages = self.get_registered_appimages()?;
        let storage_usage = self.get_storage_usage()?;
        let last_scan = self.get_last_scan_timestamp();

        Ok(SystemStatus {
            systemd_units,
            registered_appimages,
            storage_usage,
            last_scan,
        })
    }

    fn get_systemd_status(&self) -> Result<Vec<UnitStatus>, StatusError> {
        let mut units = Vec::new();
        let unit_names = vec![
            "register-appimages.path",
            "move-appimages.path",
            "register-appimages.service",
            "move-appimages.service",
        ];

        for unit_name in unit_names {
            let status = Command::new("systemctl")
                .args(["is-enabled", unit_name])
                .output()
                .map_err(|e| StatusError::SystemdError(e.to_string()))?;

            let enabled = status.status.success();
            let stdout = String::from_utf8_lossy(&status.stdout);
            let loaded = stdout.trim().starts_with("enabled");

            let active_status = Command::new("systemctl")
                .args(["is-active", unit_name])
                .output()
                .map_err(|e| StatusError::SystemdError(e.to_string()))?;

            let active = active_status.status.success();
            let active_stdout = String::from_utf8_lossy(&active_status.stdout);
            let _state = active_stdout.trim().to_string();

            units.push(UnitStatus {
                name: unit_name.to_string(),
                loaded,
                enabled,
                active,
                state: if active { "active" } else { "inactive" }.to_string(),
            });
        }

        Ok(units)
    }

    fn get_registered_appimages(&self) -> Result<Vec<AppImageStatus>, StatusError> {
        let mut appimages = Vec::new();
        let bin_dir = self.config.bin_dir();

        if !bin_dir.exists() {
            return Ok(appimages);
        }

        for entry in fs::read_dir(&bin_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.extension().map(|e| e == "AppImage").unwrap_or(false) {
                let metadata = fs::metadata(&path)?;
                let size_bytes = metadata.len();

                let name = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("unknown")
                    .to_string();

                let version = self.extract_version_from_name(&name);

                appimages.push(AppImageStatus {
                    name: name.clone(),
                    version,
                    path: path.display().to_string(),
                    size_bytes,
                    registered_at: self.get_file_modification_time(&path),
                });
            }
        }

        appimages.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(appimages)
    }

    fn get_storage_usage(&self) -> Result<StorageUsage, StatusError> {
        let bin_usage = self.get_directory_usage(&self.config.bin_dir())?;
        let raw_usage = self.get_directory_usage(&self.config.raw_dir())?;
        let icon_usage = self.get_directory_usage(&self.config.icon_dir())?;

        let total_size_bytes = bin_usage.size_bytes + raw_usage.size_bytes + icon_usage.size_bytes;

        Ok(StorageUsage {
            bin_dir: bin_usage,
            raw_dir: raw_usage,
            icon_dir: icon_usage,
            total_size_bytes,
        })
    }

    fn get_directory_usage(&self, path: &Path) -> Result<DirectoryUsage, StatusError> {
        let mut file_count = 0;
        let mut size_bytes = 0;

        if path.exists() {
            for entry in fs::read_dir(path)? {
                let entry = entry?;
                let metadata = fs::metadata(entry.path())?;

                if metadata.is_file() {
                    file_count += 1;
                    size_bytes += metadata.len();
                }
            }
        }

        Ok(DirectoryUsage {
            path: path.display().to_string(),
            file_count,
            size_bytes,
        })
    }

    fn extract_version_from_name(&self, name: &str) -> String {
        if let Some(pos) = name.rfind('-') {
            let potential_version = &name[pos + 1..];
            let version = potential_version
                .strip_prefix('v')
                .unwrap_or(potential_version);
            if version.chars().all(|c| c.is_numeric() || c == '.') {
                return version.to_string();
            }
        }
        "current".to_string()
    }

    fn get_file_modification_time(&self, path: &Path) -> Option<String> {
        fs::metadata(path)
            .ok()
            .and_then(|m| m.modified().ok())
            .map(|time| {
                let datetime: chrono::DateTime<chrono::Utc> = time.into();
                datetime.format("%Y-%m-%d %H:%M:%S UTC").to_string()
            })
    }

    fn get_last_scan_timestamp(&self) -> Option<String> {
        let bin_dir = self.config.bin_dir();

        if !bin_dir.exists() {
            return None;
        }

        fs::read_dir(bin_dir)
            .ok()?
            .filter_map(Result::ok)
            .filter_map(|entry| {
                fs::metadata(entry.path())
                    .ok()
                    .and_then(|m| m.modified().ok())
            })
            .max()
            .map(|time| {
                let datetime: chrono::DateTime<chrono::Utc> = time.into();
                datetime.format("%Y-%m-%d %H:%M:%S UTC").to_string()
            })
    }

    pub fn print_status(&self, json_output: bool) -> Result<(), StatusError> {
        let status = self.get_status()?;

        if json_output {
            let json_str = serde_json::to_string(&status)
                .map_err(|e| StatusError::JsonError(e.to_string()))?;
            println!("{}", json_str);
        } else {
            self.print_pretty_status(&status);
        }

        Ok(())
    }

    fn print_pretty_status(&self, status: &SystemStatus) {
        println!("\n═════════════════════════════════════════════════════════════");
        println!("                     Appiman Status Report");
        println!("═════════════════════════════════════════════════════════════\n");

        println!("🔷 Systemd Units:");
        println!(
            "  {:<30} {:<10} {:<10} {:<10}",
            "Unit", "Enabled", "Active", "State"
        );
        println!(
            "  {:<30} {:<10} {:<10} {:<10}",
            "─".repeat(30),
            "─".repeat(10),
            "─".repeat(10),
            "─".repeat(10)
        );

        for unit in &status.systemd_units {
            println!(
                "  {:<30} {:<10} {:<10} {:<10}",
                unit.name,
                if unit.enabled { "✅" } else { "❌" },
                if unit.active { "✅" } else { "❌" },
                unit.state
            );
        }

        println!(
            "\n📦 Registered AppImages: {}",
            status.registered_appimages.len()
        );
        if status.registered_appimages.is_empty() {
            println!("  No AppImages registered yet.");
        } else {
            println!(
                "  {:<20} {:<12} {:<10} {:>10}",
                "Name", "Version", "Size", "Registered"
            );
            println!(
                "  {:<20} {:<12} {:<10} {:>10}",
                "─".repeat(20),
                "─".repeat(12),
                "─".repeat(10),
                "─".repeat(10)
            );

            for app in &status.registered_appimages {
                println!(
                    "  {:<20} {:<12} {:>10} {}",
                    app.name,
                    app.version,
                    Self::format_size(app.size_bytes),
                    app.registered_at
                        .as_deref()
                        .unwrap_or(&"unknown".to_string())
                );
            }
        }

        println!("\n💾 Storage Usage:");
        println!(
            "  Directory: {} files, {}",
            self.config.bin_dir().display(),
            Self::format_size(status.storage_usage.bin_dir.size_bytes)
        );
        println!(
            "  Raw:      {} files, {}",
            self.config.raw_dir().display(),
            Self::format_size(status.storage_usage.raw_dir.size_bytes)
        );
        println!(
            "  Icons:    {} files, {}",
            self.config.icon_dir().display(),
            Self::format_size(status.storage_usage.icon_dir.size_bytes)
        );
        println!("  ──────────────────────────────────");
        println!(
            "  Total:     {}",
            Self::format_size(status.storage_usage.total_size_bytes)
        );

        if let Some(timestamp) = &status.last_scan {
            println!("\n⏰ Last Scan: {}", timestamp);
        }

        println!("\n═══════════════════════════════════════════════════════════════\n");
    }

    fn format_size(bytes: u64) -> String {
        if bytes < 1024 {
            format!("{} B", bytes)
        } else if bytes < 1024 * 1024 {
            format!("{:.2} KB", bytes as f64 / 1024.0)
        } else if bytes < 1024 * 1024 * 1024 {
            format!("{:.2} MB", bytes as f64 / (1024.0 * 1024.0))
        } else {
            format!("{:.2} GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_reporter_creates_successfully() {
        let reporter = StatusReporter::new().unwrap();
        assert!(reporter.config.raw_dir().ends_with("raw"));
    }

    #[test]
    fn extract_version_from_name_works() {
        let reporter = StatusReporter::new().unwrap();
        assert_eq!(reporter.extract_version_from_name("test-app"), "current");
        assert_eq!(reporter.extract_version_from_name("test-1.2.3"), "1.2.3");
        assert_eq!(reporter.extract_version_from_name("app-v2"), "2");
    }

    #[test]
    fn format_size_works() {
        assert_eq!(StatusReporter::format_size(512), "512 B");
        assert!(StatusReporter::format_size(1536).starts_with("1.50 KB"));
        assert!(StatusReporter::format_size(2 * 1024 * 1024).starts_with("2.00 MB"));
    }
}
