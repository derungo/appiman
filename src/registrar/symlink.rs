use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;
use tracing::debug;

#[derive(Debug, Error)]
#[allow(dead_code)]
pub enum SymlinkError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Symlink creation failed for {path}: {reason}")]
    CreationFailed { path: PathBuf, reason: String },
}

#[allow(dead_code)]
pub fn create_symlink(target: &Path, link_path: &Path) -> Result<(), SymlinkError> {
    debug!("Creating symlink: {:?} -> {:?}", link_path, target);

    if link_path.exists() {
        fs::remove_file(link_path)?;
    }

    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(target, link_path)?;
    }

    #[cfg(not(unix))]
    {
        return Err(SymlinkError::CreationFailed {
            path: link_path.to_path_buf(),
            reason: "Symlinks not supported on this platform".to_string(),
        });
    }

    debug!("Created symlink: {:?}", link_path);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn create_symlink_creates_link() {
        let temp = TempDir::new().unwrap();
        let target = temp.path().join("target");
        let link = temp.path().join("link");
        fs::write(&target, b"content").unwrap();

        #[cfg(unix)]
        {
            create_symlink(&target, &link).unwrap();
            assert!(link.exists());
            assert!(link.is_symlink());
        }
    }

    #[test]
    fn create_symlink_replaces_existing() {
        let temp = TempDir::new().unwrap();
        let target = temp.path().join("target");
        let link = temp.path().join("link");
        fs::write(&target, b"new content").unwrap();
        fs::write(&link, b"old content").unwrap();

        #[cfg(unix)]
        {
            create_symlink(&target, &link).unwrap();
            let target_content = fs::read_to_string(&link).unwrap();
            assert_eq!(target_content, "new content");
        }
    }
}
