use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("Failed to read config file: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Failed to parse config: {0}")]
    ParseError(#[from] toml::de::Error),

    #[error("Environment variable error: {0}")]
    EnvVarError(#[from] std::env::VarError),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Directories {
    pub raw: String,

    pub bin: String,

    pub icons: String,

    pub desktop: String,

    pub symlink: String,

    pub home_root: String,
}

impl Default for Directories {
    fn default() -> Self {
        Directories {
            raw: default_raw_dir(),
            bin: default_bin_dir(),
            icons: default_icon_dir(),
            desktop: default_desktop_dir(),
            symlink: default_symlink_dir(),
            home_root: default_home_root(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Logging {
    pub level: String,

    #[serde(default)]
    pub json_output: bool,
}

impl Default for Logging {
    fn default() -> Self {
        Logging {
            level: default_log_level(),
            json_output: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub directories: Directories,

    #[serde(default)]
    pub logging: Logging,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            directories: Directories::default(),
            logging: Logging::default(),
        }
    }
}

impl Config {
    pub fn load() -> Result<Self, ConfigError> {
        let config_path = Self::config_path();

        if !config_path.exists() {
            return Ok(Config::default());
        }

        let content = std::fs::read_to_string(&config_path)?;
        let mut config: Config = toml::from_str(&content)?;

        config.apply_env_overrides();

        Ok(config)
    }

    pub fn config_path() -> PathBuf {
        std::env::var_os("APPIMAN_CONFIG")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("/etc/appiman/config.toml"))
    }

    pub fn raw_dir(&self) -> PathBuf {
        PathBuf::from(&self.directories.raw)
    }

    pub fn bin_dir(&self) -> PathBuf {
        PathBuf::from(&self.directories.bin)
    }

    pub fn icon_dir(&self) -> PathBuf {
        PathBuf::from(&self.directories.icons)
    }

    pub fn desktop_dir(&self) -> PathBuf {
        PathBuf::from(&self.directories.desktop)
    }

    pub fn symlink_dir(&self) -> PathBuf {
        PathBuf::from(&self.directories.symlink)
    }

    pub fn home_root(&self) -> PathBuf {
        PathBuf::from(&self.directories.home_root)
    }

    pub fn log_level(&self) -> &str {
        &self.logging.level
    }

    pub fn json_output(&self) -> bool {
        self.logging.json_output
    }

    fn apply_env_overrides(&mut self) {
        if let Ok(val) = std::env::var("APPIMAN_RAW_DIR") {
            self.directories.raw = val;
        }
        if let Ok(val) = std::env::var("APPIMAN_BIN_DIR") {
            self.directories.bin = val;
        }
        if let Ok(val) = std::env::var("APPIMAN_ICON_DIR") {
            self.directories.icons = val;
        }
        if let Ok(val) = std::env::var("APPIMAN_DESKTOP_DIR") {
            self.directories.desktop = val;
        }
        if let Ok(val) = std::env::var("APPIMAN_SYMLINK_DIR") {
            self.directories.symlink = val;
        }
        if let Ok(val) = std::env::var("APPIMAN_HOME_ROOT") {
            self.directories.home_root = val;
        }
        if let Ok(val) = std::env::var("RUST_LOG") {
            self.logging.level = val;
        }
    }
}

fn default_raw_dir() -> String {
    "/opt/applications/raw".to_string()
}

fn default_bin_dir() -> String {
    "/opt/applications/bin".to_string()
}

fn default_icon_dir() -> String {
    "/opt/applications/icons".to_string()
}

fn default_desktop_dir() -> String {
    "/usr/share/applications".to_string()
}

fn default_symlink_dir() -> String {
    "/usr/local/bin".to_string()
}

fn default_home_root() -> String {
    "/home".to_string()
}

fn default_log_level() -> String {
    "info".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn config_loads_default_when_missing() {
        let temp = TempDir::new().unwrap();
        let config_path = temp.path().join("config.toml");

        unsafe {
            std::env::set_var("APPIMAN_CONFIG", config_path);
        }

        let config = Config::load().unwrap();

        assert_eq!(config.directories.raw, "/opt/applications/raw");
        assert_eq!(config.directories.bin, "/opt/applications/bin");
        assert_eq!(config.directories.icons, "/opt/applications/icons");
        assert_eq!(config.directories.desktop, "/usr/share/applications");
        assert_eq!(config.directories.symlink, "/usr/local/bin");
        assert_eq!(config.directories.home_root, "/home");
        assert_eq!(config.logging.level, "info");
        assert_eq!(config.logging.json_output, false);

        unsafe {
            std::env::remove_var("APPIMAN_CONFIG");
        }
    }

    #[test]
    fn config_loads_from_file() {
        let temp = TempDir::new().unwrap();
        let config_path = temp.path().join("config.toml");

        let config_content = r#"
[directories]
raw = "/custom/raw"
bin = "/custom/bin"
icons = "/custom/icons"
desktop = "/custom/desktop"
symlink = "/custom/symlink"
home_root = "/custom/home"

[logging]
level = "debug"
json_output = true
"#;

        fs::write(&config_path, config_content).unwrap();
        unsafe {
            std::env::remove_var("APPIMAN_CONFIG");
            std::env::remove_var("APPIMAN_RAW_DIR");
            std::env::remove_var("APPIMAN_BIN_DIR");
            std::env::remove_var("APPIMAN_ICON_DIR");
            std::env::remove_var("APPIMAN_DESKTOP_DIR");
            std::env::remove_var("APPIMAN_SYMLINK_DIR");
            std::env::remove_var("APPIMAN_HOME_ROOT");
            std::env::set_var("APPIMAN_CONFIG", config_path);
        }

        let config = Config::load().unwrap();

        assert_eq!(config.directories.raw, "/custom/raw");
        assert_eq!(config.directories.bin, "/custom/bin");
        assert_eq!(config.logging.level, "debug");
        assert_eq!(config.logging.json_output, true);

        unsafe {
            std::env::remove_var("APPIMAN_CONFIG");
        }
    }

    #[test]
    fn config_env_overrides_work() {
        let temp = TempDir::new().unwrap();
        let config_path = temp.path().join("config.toml");

        let config_content = r#"
[directories]
raw = "/config/raw"
bin = "/config/bin"
icons = "/config/icons"
desktop = "/config/desktop"
symlink = "/config/symlink"
home_root = "/config/home"
"#;

        fs::write(&config_path, config_content).unwrap();
        unsafe {
            std::env::remove_var("APPIMAN_CONFIG");
            std::env::remove_var("APPIMAN_RAW_DIR");
            std::env::remove_var("APPIMAN_BIN_DIR");
            std::env::remove_var("APPIMAN_ICON_DIR");
            std::env::remove_var("APPIMAN_DESKTOP_DIR");
            std::env::remove_var("APPIMAN_SYMLINK_DIR");
            std::env::remove_var("APPIMAN_HOME_ROOT");
            std::env::set_var("APPIMAN_CONFIG", config_path);
            std::env::set_var("APPIMAN_RAW_DIR", "/env/raw");
        }

        let config = Config::load().unwrap();

        assert_eq!(config.directories.raw, "/env/raw");

        unsafe {
            std::env::remove_var("APPIMAN_CONFIG");
            std::env::remove_var("APPIMAN_RAW_DIR");
        }
    }

    #[test]
    fn config_path_methods_work() {
        let config = Config::default();
        let raw_dir = config.raw_dir();
        let bin_dir = config.bin_dir();

        assert_eq!(raw_dir, PathBuf::from("/opt/applications/raw"));
        assert_eq!(bin_dir, PathBuf::from("/opt/applications/bin"));
    }
}
