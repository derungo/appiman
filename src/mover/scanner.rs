use std::path::PathBuf;
use thiserror::Error;
use walkdir::{DirEntry, WalkDir};

use crate::core::AppImage;

#[derive(Debug, Error)]
pub enum ScanError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Walk error: {0}")]
    Walk(#[from] walkdir::Error),

    #[error("Home directory not found: {0}")]
    HomeDirNotFound(String),
}

pub struct Scanner {
    pub home_root: PathBuf,
    pub exclude_dirs: Vec<PathBuf>,
}

impl Scanner {
    pub fn new(home_root: PathBuf) -> Self {
        let exclude_dirs = vec![home_root.join(".cache"), home_root.join(".local/share")];

        Scanner {
            home_root,
            exclude_dirs,
        }
    }

    #[allow(dead_code)]
    pub fn with_excludes(home_root: PathBuf, exclude_dirs: Vec<PathBuf>) -> Self {
        Scanner {
            home_root,
            exclude_dirs,
        }
    }

    pub fn find_appimages(&self) -> Result<Vec<AppImage>, ScanError> {
        let mut appimages = Vec::new();

        if !self.home_root.exists() {
            return Err(ScanError::HomeDirNotFound(
                self.home_root.display().to_string(),
            ));
        }

        for entry in WalkDir::new(&self.home_root)
            .follow_links(false)
            .into_iter()
            .filter_entry(|e| !self.is_excluded(e))
            .filter_map(|e| e.ok())
        {
            if entry.file_type().is_file()
                && let Some(ext) = entry.path().extension()
                    && ext.eq_ignore_ascii_case("AppImage")
                        && let Ok(app) = AppImage::new(entry.path().to_path_buf()) {
                            appimages.push(app);
                        }
        }

        Ok(appimages)
    }

    #[allow(dead_code)]
    pub fn find_user_dirs(&self) -> Result<Vec<PathBuf>, ScanError> {
        if !self.home_root.exists() {
            return Err(ScanError::HomeDirNotFound(
                self.home_root.display().to_string(),
            ));
        }

        let mut user_dirs = Vec::new();

        for entry in std::fs::read_dir(&self.home_root)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() {
                user_dirs.push(path);
            }
        }

        Ok(user_dirs)
    }

    fn is_excluded(&self, entry: &DirEntry) -> bool {
        let path = entry.path();

        for exclude in &self.exclude_dirs {
            if path.starts_with(exclude) {
                return true;
            }
        }

        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn scanner_finds_appimages_in_user_homes() {
        let temp = TempDir::new().unwrap();
        let home_root = temp.path().join("home");
        fs::create_dir_all(&home_root).unwrap();

        let alice = home_root.join("alice");
        let bob = home_root.join("bob");
        fs::create_dir_all(&alice).unwrap();
        fs::create_dir_all(&bob).unwrap();

        let app1 = alice.join("App1.AppImage");
        let app2 = bob.join("App2.AppImage");
        let app3 = bob.join("not-an-app.txt");
        fs::write(&app1, b"app1").unwrap();
        fs::write(&app2, b"app2").unwrap();
        fs::write(&app3, b"text").unwrap();

        let scanner = Scanner::new(home_root);
        let found = scanner.find_appimages().unwrap();

        assert_eq!(found.len(), 2);
        assert!(found.iter().any(|a| a.path == app1));
        assert!(found.iter().any(|a| a.path == app2));
        assert!(!found.iter().any(|a| a.path == app3));
    }

    #[test]
    fn scanner_excludes_directories() {
        let temp = TempDir::new().unwrap();
        let home_root = temp.path().join("home");
        let cache_dir = home_root.join(".cache");
        fs::create_dir_all(&cache_dir).unwrap();

        let app_in_cache = cache_dir.join("Cached.AppImage");
        fs::write(&app_in_cache, b"cached").unwrap();

        let scanner = Scanner::new(home_root);
        let found = scanner.find_appimages().unwrap();

        assert_eq!(found.len(), 0);
    }

    #[test]
    fn scanner_finds_user_dirs() {
        let temp = TempDir::new().unwrap();
        let home_root = temp.path().join("home");
        fs::create_dir_all(&home_root).unwrap();

        let alice = home_root.join("alice");
        let bob = home_root.join("bob");
        fs::create_dir_all(&alice).unwrap();
        fs::create_dir_all(&bob).unwrap();

        let scanner = Scanner::new(home_root);
        let dirs = scanner.find_user_dirs().unwrap();

        assert_eq!(dirs.len(), 2);
        assert!(dirs.contains(&alice));
        assert!(dirs.contains(&bob));
    }

    #[test]
    fn scanner_handles_nonexistent_home_root() {
        let temp = TempDir::new().unwrap();
        let home_root = temp.path().join("nonexistent");

        let scanner = Scanner::new(home_root);
        let result = scanner.find_appimages();

        assert!(matches!(result, Err(ScanError::HomeDirNotFound(_))));
    }
}
