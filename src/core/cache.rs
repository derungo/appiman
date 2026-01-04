use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum CacheError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serde(#[from] serde_json::Error),

    #[error("Invalid cache entry")]
    #[allow(dead_code)]
    InvalidEntry,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheEntry {
    pub checksum: String,
    pub mtime: u64,
    pub processed_at: u64,
    pub normalized_name: String,
    pub version: String,
}

#[derive(Debug)]
pub struct MetadataCache {
    cache_file: PathBuf,
    entries: HashMap<String, CacheEntry>,
}

impl MetadataCache {
    pub fn new(cache_dir: &Path) -> Self {
        let cache_file = cache_dir.join("metadata_cache.json");
        let entries = Self::load_cache(&cache_file).unwrap_or_default();

        MetadataCache {
            cache_file,
            entries,
        }
    }

    fn load_cache(cache_file: &Path) -> Result<HashMap<String, CacheEntry>, CacheError> {
        if !cache_file.exists() {
            return Ok(HashMap::new());
        }

        let content = fs::read_to_string(cache_file)?;
        let entries: HashMap<String, CacheEntry> = serde_json::from_str(&content)?;
        Ok(entries)
    }

    pub fn is_cached(&self, path: &Path, checksum: &str) -> bool {
        if let Some(entry) = self.entries.get(&path.display().to_string()) {
            entry.checksum == checksum
        } else {
            false
        }
    }

    pub fn get_cached_entry(&self, path: &Path) -> Option<&CacheEntry> {
        self.entries.get(&path.display().to_string())
    }

    pub fn add_entry(
        &mut self,
        path: &Path,
        checksum: String,
        mtime: u64,
        normalized_name: String,
        version: String,
    ) {
        let processed_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let entry = CacheEntry {
            checksum,
            mtime,
            processed_at,
            normalized_name,
            version,
        };

        self.entries.insert(path.display().to_string(), entry);
    }

    pub fn save(&self) -> Result<(), CacheError> {
        if let Some(parent) = self.cache_file.parent() {
            fs::create_dir_all(parent)?;
        }

        let content = serde_json::to_string_pretty(&self.entries)?;
        fs::write(&self.cache_file, content)?;
        Ok(())
    }

    #[allow(dead_code)]
    pub fn cleanup_stale_entries(&mut self, raw_dir: &Path) -> Result<(), CacheError> {
        if !raw_dir.exists() {
            return Ok(());
        }

        let mut to_remove = Vec::new();
        for (path_str, _) in &self.entries {
            let path = Path::new(path_str);
            if !path.exists() {
                to_remove.push(path_str.clone());
            }
        }

        for key in to_remove {
            self.entries.remove(&key);
        }

        Ok(())
    }

    #[allow(dead_code)]
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    #[allow(dead_code)]
    pub fn len(&self) -> usize {
        self.entries.len()
    }
}
