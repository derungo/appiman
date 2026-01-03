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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Metadata {
    pub name: String,
    pub version: Option<String>,
    pub categories: Vec<String>,
    pub icon_path: Option<String>,
    pub extracted_at: DateTime<Utc>,
    pub checksum: String,
}

impl Metadata {
    pub fn new(name: String) -> Self {
        Metadata {
            name,
            version: None,
            categories: vec!["Utility".to_string()],
            icon_path: None,
            extracted_at: Utc::now(),
            checksum: String::new(),
        }
    }

    pub fn from_desktop_entry(path: &Path) -> Result<Self, MetadataError> {
        let content = std::fs::read_to_string(path)?;
        let mut metadata = Metadata::new("Unknown".to_string());

        for line in content.lines() {
            if line.starts_with("Name=") {
                metadata.name = line[5..].trim().to_string();
            } else if line.starts_with("Categories=") {
                metadata.categories = line[11..]
                    .split(';')
                    .filter(|s| !s.is_empty())
                    .map(|s| s.trim().to_string())
                    .collect();
            } else if line.starts_with("Icon=") {
                metadata.icon_path = Some(line[5..].trim().to_string());
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
        let metadata = Metadata::new("TestApp".to_string());
        let json = metadata.to_json().unwrap();
        let deserialized = Metadata::from_json(&json).unwrap();

        assert_eq!(metadata.name, deserialized.name);
    }

    #[test]
    fn metadata_setters_work() {
        let mut metadata = Metadata::new("TestApp".to_string());

        metadata.set_version("1.0.0".to_string());
        assert_eq!(metadata.version, Some("1.0.0".to_string()));

        metadata.set_icon_path("/path/to/icon.png".to_string());
        assert_eq!(metadata.icon_path, Some("/path/to/icon.png".to_string()));
    }
}
