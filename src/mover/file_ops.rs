use std::path::{Path, PathBuf};
use thiserror::Error;
use tracing::{debug, error, info, warn};

use crate::core::AppImage;
use crate::mover::conflict::handle_collision;
use crate::mover::scanner::Scanner;

impl From<crate::mover::conflict::CollisionError> for MoveError {
    fn from(err: crate::mover::conflict::CollisionError) -> Self {
        MoveError::CollisionFailed {
            path: PathBuf::new(),
            reason: err.to_string(),
        }
    }
}

#[derive(Debug, Error)]
pub enum MoveError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Scan error: {0}")]
    Scan(#[from] crate::mover::scanner::ScanError),

    #[error("Collision resolution failed for {path}: {reason}")]
    CollisionFailed { path: PathBuf, reason: String },
}

pub struct MoveReport {
    pub moved: Vec<PathBuf>,
    pub skipped: Vec<PathBuf>,
    pub errors: Vec<(PathBuf, String)>,
}

impl MoveReport {
    pub fn new() -> Self {
        MoveReport {
            moved: Vec::new(),
            skipped: Vec::new(),
            errors: Vec::new(),
        }
    }

    pub fn success_count(&self) -> usize {
        self.moved.len()
    }

    pub fn error_count(&self) -> usize {
        self.errors.len()
    }

    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }
}

pub struct Mover {
    pub source_dir: PathBuf,
    pub dest_dir: PathBuf,
    pub dry_run: bool,
}

impl Mover {
    pub fn new(source_dir: PathBuf, dest_dir: PathBuf) -> Self {
        Mover {
            source_dir,
            dest_dir,
            dry_run: false,
        }
    }

    pub fn with_dry_run(mut self, dry_run: bool) -> Self {
        self.dry_run = dry_run;
        self
    }

    pub fn move_appimages(&self, appimages: &[AppImage]) -> Result<MoveReport, MoveError> {
        info!(
            "Moving {} AppImages from {:?} to {:?}",
            appimages.len(),
            self.source_dir,
            self.dest_dir
        );

        let mut report = MoveReport::new();

        if !self.dest_dir.exists() {
            debug!("Creating destination directory: {:?}", self.dest_dir);
            std::fs::create_dir_all(&self.dest_dir)?;
        }

        for app in appimages {
            match self.move_single_appimage(app) {
                Ok(dest) => {
                    info!("Moved {:?} to {:?}", app.path, dest);
                    report.moved.push(dest);
                }
                Err(e) => {
                    warn!("Failed to move {:?}: {}", app.path, e);
                    report.errors.push((app.path.clone(), e.to_string()));
                }
            }
        }

        if report.has_errors() {
            error!("Completed with {} errors", report.error_count());
        } else {
            info!("Successfully moved {} AppImages", report.success_count());
        }

        Ok(report)
    }

    pub fn scan_and_move(&self, home_root: PathBuf) -> Result<MoveReport, MoveError> {
        let scanner = Scanner::new(home_root);
        let appimages = scanner.find_appimages()?;

        self.move_appimages(&appimages)
    }

    fn move_single_appimage(&self, app: &AppImage) -> Result<PathBuf, MoveError> {
        let dest = self.determine_destination(app)?;

        if self.dry_run {
            info!("[DRY RUN] Would move {:?} to {:?}", app.path, dest);
            return Ok(dest);
        }

        if dest.exists() {
            let resolved_dest = handle_collision(&app.path, &dest)?;
            if resolved_dest != app.path {
                std::fs::rename(&app.path, &resolved_dest)?;
                self.set_permissions(&resolved_dest)?;
            }
            Ok(resolved_dest)
        } else {
            std::fs::rename(&app.path, &dest)?;
            self.set_permissions(&dest)?;
            Ok(dest)
        }
    }

    fn determine_destination(&self, app: &AppImage) -> Result<PathBuf, MoveError> {
        let filename = app.path.file_name().ok_or_else(|| {
            MoveError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Invalid filename",
            ))
        })?;

        Ok(self.dest_dir.clone().join(filename))
    }

    #[cfg(unix)]
    fn set_permissions(&self, path: &Path) -> Result<(), MoveError> {
        use std::fs::{metadata, set_permissions};
        use std::os::unix::fs::{chown, PermissionsExt};

        let mut perms = metadata(path)?.permissions();
        perms.set_mode(0o755);
        set_permissions(path, perms)?;

        if let Err(e) = chown(path, Some(0), Some(0)) {
            warn!("Failed to chown {:?} to root:root: {}", path, e);
        }

        Ok(())
    }

    #[cfg(not(unix))]
    fn set_permissions(&self, _path: &Path) -> Result<(), MoveError> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn create_appimage(path: &Path) {
        fs::write(path, b"fake appimage").unwrap();
    }

    #[test]
    fn mover_moves_single_appimage() {
        let temp = TempDir::new().unwrap();
        let source = temp.path().join("source");
        let dest = temp.path().join("dest");
        fs::create_dir_all(&source).unwrap();
        fs::create_dir_all(&dest).unwrap();

        let app = source.join("Test.AppImage");
        create_appimage(&app);

        let mover = Mover::new(source.clone(), dest.clone());
        let report = mover
            .move_appimages(&[AppImage::new(app.clone()).unwrap()])
            .unwrap();

        assert_eq!(report.moved.len(), 1);
        assert!(report.moved[0].starts_with(&dest));
        assert!(!app.exists());
        assert!(dest.join("Test.AppImage").exists());
    }

    #[test]
    fn mover_handles_collisions() {
        let temp = TempDir::new().unwrap();
        let source = temp.path().join("source");
        let dest = temp.path().join("dest");
        fs::create_dir_all(&source).unwrap();
        fs::create_dir_all(&dest).unwrap();

        let app1 = source.join("Same.AppImage");
        create_appimage(&app1);
        create_appimage(&dest.join("Same.AppImage"));

        let mover = Mover::new(source.clone(), dest.clone());
        let report = mover
            .move_appimages(&[AppImage::new(app1).unwrap()])
            .unwrap();

        assert_eq!(report.moved.len(), 1);
        assert!(dest.join("Same-1.AppImage").exists());
    }

    #[test]
    fn mover_dry_run_does_not_move() {
        let temp = TempDir::new().unwrap();
        let source = temp.path().join("source");
        let dest = temp.path().join("dest");
        fs::create_dir_all(&source).unwrap();
        fs::create_dir_all(&dest).unwrap();

        let app = source.join("Test.AppImage");
        create_appimage(&app);

        let mover = Mover::new(source.clone(), dest.clone()).with_dry_run(true);
        let report = mover
            .move_appimages(&[AppImage::new(app.clone()).unwrap()])
            .unwrap();

        assert_eq!(report.moved.len(), 1);
        assert!(app.exists());
        assert!(!dest.join("Test.AppImage").exists());
    }

    #[test]
    fn mover_creates_dest_directory() {
        let temp = TempDir::new().unwrap();
        let source = temp.path().join("source");
        let dest = temp.path().join("dest/nested/path");
        fs::create_dir_all(&source).unwrap();

        let app = source.join("Test.AppImage");
        create_appimage(&app);

        let mover = Mover::new(source.clone(), dest.clone());
        mover
            .move_appimages(&[AppImage::new(app.clone()).unwrap()])
            .unwrap();

        assert!(dest.exists());
        assert!(dest.join("Test.AppImage").exists());
    }
}
