use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::Path;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum MetadataError {
    #[error("Failed to parse desktop entry: {0}")]
    ParseError(String),

    #[error("Invalid JSON: {0}")]
    JsonError(#[from] serde_json::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VersionInfo {
    pub version: String,
    pub checksum: String,
    pub installed_at: DateTime<Utc>,
    pub is_active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AppMetadata {
    pub name: String,
    pub display_name: String,
    pub categories: Vec<String>,
    pub icon_path: Option<String>,
    pub versions: Vec<VersionInfo>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Metadata {
    pub name: String,
    pub version: Option<String>,
    pub categories: Vec<String>,
    pub icon_path: Option<String>,
    pub extracted_at: DateTime<Utc>,
    pub checksum: String,
}

impl Metadata {
    pub fn new(name: String, checksum: String) -> Self {
        Metadata {
            name,
            version: None,
            categories: vec!["Utility".to_string()],
            icon_path: None,
            extracted_at: Utc::now(),
            checksum,
        }
    }

    pub fn from_desktop_entry(path: &Path) -> Result<Self, MetadataError> {
        let content = std::fs::read_to_string(path)?;
        let mut metadata = Metadata::new("Unknown".to_string(), String::new());

        for line in content.lines() {
            if let Some(stripped) = line.strip_prefix("Name=") {
                metadata.name = stripped.trim().to_string();
            } else if let Some(stripped) = line.strip_prefix("Categories=") {
                metadata.categories = stripped
                    .split(';')
                    .filter(|s| !s.is_empty())
                    .map(|s| s.trim().to_string())
                    .collect();
            } else if let Some(stripped) = line.strip_prefix("Icon=") {
                metadata.icon_path = Some(stripped.trim().to_string());
            }
        }

        Ok(metadata)
    }

    pub fn to_json(&self) -> Result<String, MetadataError> {
        Ok(serde_json::to_string_pretty(self)?)
    }

    pub fn from_json(s: &str) -> Result<Self, MetadataError> {
        Ok(serde_json::from_str(s)?)
    }

    pub fn set_version(&mut self, version: String) {
        self.version = Some(version);
    }

    pub fn set_icon_path(&mut self, path: String) {
        self.icon_path = Some(path);
    }

    pub fn set_name(&mut self, name: String) {
        self.name = name;
    }
}

impl AppMetadata {
    pub fn new(display_name: String, normalized_name: String) -> Self {
        AppMetadata {
            name: normalized_name,
            display_name,
            categories: vec!["Utility".to_string()],
            icon_path: None,
            versions: Vec::new(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    pub fn add_version(&mut self, version: String, checksum: String) -> &mut VersionInfo {
        // Deactivate all other versions
        for v in &mut self.versions {
            v.is_active = false;
        }

        let version_info = VersionInfo {
            version: version.clone(),
            checksum,
            installed_at: Utc::now(),
            is_active: true,
        };

        self.versions.push(version_info);
        self.updated_at = Utc::now();

        self.versions.last_mut().unwrap()
    }

    pub fn get_active_version(&self) -> Option<&VersionInfo> {
        self.versions.iter().find(|v| v.is_active)
    }

    pub fn set_active_version(&mut self, version: &str) -> bool {
        let mut found = false;
        for v in &mut self.versions {
            if v.version == version {
                v.is_active = true;
                found = true;
            } else {
                v.is_active = false;
            }
        }
        if found {
            self.updated_at = Utc::now();
        }
        found
    }

    pub fn get_version(&self, version: &str) -> Option<&VersionInfo> {
        self.versions.iter().find(|v| v.version == version)
    }

    pub fn remove_version(&mut self, version: &str) -> bool {
        if let Some(pos) = self.versions.iter().position(|v| v.version == version) {
            self.versions.remove(pos);
            self.updated_at = Utc::now();
            true
        } else {
            false
        }
    }

    pub fn to_json(&self) -> Result<String, MetadataError> {
        Ok(serde_json::to_string_pretty(self)?)
    }

    pub fn from_json(s: &str) -> Result<Self, MetadataError> {
        Ok(serde_json::from_str(s)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::NamedTempFile;

    #[test]
    fn metadata_from_desktop_entry_parses_fields() {
        let temp_file = NamedTempFile::new().unwrap();
        let content = r#"[Desktop Entry]
Name=Test Application
Categories=Utility;Office;
Icon=testapp
"#;
        fs::write(temp_file.path(), content).unwrap();

        let metadata = Metadata::from_desktop_entry(temp_file.path()).unwrap();

        assert_eq!(metadata.name, "Test Application");
        assert_eq!(metadata.categories, vec!["Utility", "Office"]);
        assert_eq!(metadata.icon_path, Some("testapp".to_string()));
    }

    #[test]
    fn metadata_serialization_works() {
        let metadata = Metadata::new("TestApp".to_string(), "abc123".to_string());
        let json = metadata.to_json().unwrap();
        let deserialized = Metadata::from_json(&json).unwrap();

        assert_eq!(metadata.name, deserialized.name);
        assert_eq!(metadata.checksum, deserialized.checksum);
    }

    #[test]
    fn metadata_setters_work() {
        let mut metadata = Metadata::new("TestApp".to_string(), "abc123".to_string());

        metadata.set_version("1.0.0".to_string());
        assert_eq!(metadata.version, Some("1.0.0".to_string()));

        metadata.set_icon_path("/path/to/icon.png".to_string());
        assert_eq!(metadata.icon_path, Some("/path/to/icon.png".to_string()));
    }
}
