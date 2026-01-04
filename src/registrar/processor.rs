use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use thiserror::Error;
use tracing::{debug, error, info, instrument, warn};

use crate::core::{normalize_appimage_name, AppImage, AppImageError, Metadata};
use crate::registrar::desktop_entry::DesktopEntry;
use crate::registrar::icon_extractor;

#[derive(Debug, Error)]
pub enum ProcessError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("AppImage error: {0}")]
    AppImage(#[from] AppImageError),

    #[error("Extraction failed: {0}")]
    ExtractionFailed(String),

    #[error("Desktop entry error: {0}")]
    DesktopEntry(String),
}

#[derive(Debug)]
pub struct ProcessedApp {
    pub normalized_name: String,
    pub appimage_path: PathBuf,
}

#[derive(Debug)]
pub struct ProcessReport {
    pub processed: Vec<ProcessedApp>,
    pub failed: Vec<(PathBuf, String)>,
    pub skipped: Vec<PathBuf>,
}

impl ProcessReport {
    pub fn new() -> Self {
        ProcessReport {
            processed: Vec::new(),
            failed: Vec::new(),
            skipped: Vec::new(),
        }
    }

    pub fn success_count(&self) -> usize {
        self.processed.len()
    }

    pub fn failure_count(&self) -> usize {
        self.failed.len()
    }
}

pub struct Processor {
    pub raw_dir: PathBuf,
    pub bin_dir: PathBuf,
    pub icon_dir: PathBuf,
    pub desktop_dir: PathBuf,
    pub symlink_dir: PathBuf,
    pub dry_run: bool,
}

impl Processor {
    pub fn new(
        raw_dir: PathBuf,
        bin_dir: PathBuf,
        icon_dir: PathBuf,
        desktop_dir: PathBuf,
        symlink_dir: PathBuf,
    ) -> Self {
        Processor {
            raw_dir,
            bin_dir,
            icon_dir,
            desktop_dir,
            symlink_dir,
            dry_run: false,
        }
    }

    pub fn with_dry_run(mut self, dry_run: bool) -> Self {
        self.dry_run = dry_run;
        self
    }

    #[instrument(skip(self))]
    pub fn process_all(&self) -> Result<ProcessReport, ProcessError> {
        info!("Processing all AppImages in {:?}", self.raw_dir);

        let mut report = ProcessReport::new();

        if !self.raw_dir.exists() {
            warn!("Raw directory does not exist: {:?}", self.raw_dir);
            return Ok(report);
        }

        for entry in fs::read_dir(&self.raw_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path
                .extension()
                .map_or(false, |e| e.eq_ignore_ascii_case("AppImage"))
            {
                match self.process_single_appimage(&path) {
                    Ok(processed) => {
                        info!("Processed: {}", processed.normalized_name);
                        report.processed.push(processed);
                    }
                    Err(e) => {
                        error!("Failed to process {:?}: {}", path, e);
                        report.failed.push((path, e.to_string()));
                    }
                }
            }
        }

        if report.failed.is_empty() {
            info!(
                "Successfully processed {} AppImages",
                report.success_count()
            );
        } else {
            error!("Completed with {} failures", report.failure_count());
        }

        Ok(report)
    }

    #[instrument(skip(self, app_path))]
    pub fn process_single_appimage(&self, app_path: &Path) -> Result<ProcessedApp, ProcessError> {
        let app = AppImage::new(app_path.to_path_buf())?;
        app.validate()?;

        let normalized_name =
            normalize_appimage_name(app_path.file_stem().and_then(|s| s.to_str()).unwrap_or(""));

        if normalized_name.is_empty() {
            return Err(ProcessError::DesktopEntry(
                "Empty normalized name".to_string(),
            ));
        }

        debug!("Processing AppImage: {:?} -> {}", app_path, normalized_name);

        let dest = self.bin_dir.join(format!("{}.AppImage", normalized_name));

        if self.dry_run {
            info!("[DRY RUN] Would process: {}", normalized_name);
            return Ok(ProcessedApp {
                normalized_name,
                appimage_path: app_path.to_path_buf(),
            });
        }

        self.copy_appimage(&app_path, &dest)?;
        self.make_executable(&dest)?;

        let (metadata, icon_path) = self.extract_metadata(app_path, &normalized_name)?;

        let desktop_path = self
            .desktop_dir
            .join(format!("{}.desktop", normalized_name));
        self.create_desktop_entry(&metadata, &icon_path, &desktop_path)?;

        let symlink_path = self.symlink_dir.join(&normalized_name);
        self.create_symlink(&dest, &symlink_path)?;

        info!("Running appimage-update check for {}", normalized_name);
        let _ = Command::new(&dest).arg("--appimage-update").output();

        Ok(ProcessedApp {
            normalized_name,
            appimage_path: app_path.to_path_buf(),
        })
    }

    fn copy_appimage(&self, src: &Path, dest: &Path) -> Result<(), ProcessError> {
        if dest.exists() {
            debug!("Destination already exists, skipping copy: {:?}", dest);
            return Ok(());
        }

        debug!("Copying {:?} to {:?}", src, dest);
        fs::copy(src, dest)?;
        Ok(())
    }

    fn make_executable(&self, path: &Path) -> Result<(), ProcessError> {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(path)?.permissions();
            perms.set_mode(0o755);
            fs::set_permissions(path, perms)?;
            debug!("Set executable permissions: {:?}", path);
        }

        Ok(())
    }

    fn extract_metadata(
        &self,
        app_path: &Path,
        normalized_name: &str,
    ) -> Result<(Metadata, Option<PathBuf>), ProcessError> {
        let tmp_dir = tempfile::TempDir::new()?;
        let app_root = tmp_dir.path().join("squashfs-root");

        debug!("Extracting AppImage: {:?}", app_path);

        let status = Command::new(app_path)
            .arg("--appimage-extract")
            .current_dir(tmp_dir.path())
            .status()?;

        if !status.success() {
            return Err(ProcessError::ExtractionFailed(format!(
                "AppImage extract failed: {}",
                status
            )));
        }

        if !app_root.exists() {
            return Err(ProcessError::ExtractionFailed(
                "squashfs-root not found after extraction".to_string(),
            ));
        }

        let app = AppImage::new(app_path.to_path_buf())?;
        let checksum = app.get_checksum().map_err(|e| ProcessError::AppImage(e))?;

        let desktop_file = self.find_desktop_entry(&app_root)?;
        let icon_path = icon_extractor::extract_icon(&app_root, &self.icon_dir, normalized_name)
            .map_err(|e| {
                ProcessError::Io(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    e.to_string(),
                ))
            })?;

        match desktop_file {
            Some(path) => {
                debug!("Found desktop entry: {:?}", path);
                let mut metadata = Metadata::from_desktop_entry(&path)
                    .map_err(|e| ProcessError::DesktopEntry(e.to_string()))?;
                metadata.checksum = checksum.clone();
                Ok((metadata, icon_path))
            }
            None => {
                debug!("No desktop entry found, using defaults");
                let display_name = format!(
                    "{}{}",
                    normalized_name.chars().next().unwrap().to_uppercase(),
                    &normalized_name[1..]
                );
                let mut metadata = Metadata::new(display_name, checksum);
                metadata.name = format!(
                    "{}{}",
                    normalized_name.chars().next().unwrap().to_uppercase(),
                    &normalized_name[1..]
                );
                Ok((metadata, icon_path))
            }
        }
    }

    fn find_desktop_entry(&self, root: &Path) -> Result<Option<PathBuf>, ProcessError> {
        for entry in fs::read_dir(root)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() {
                if let Some(ext) = path.extension() {
                    if ext == "desktop" {
                        return Ok(Some(path));
                    }
                }
            }
        }

        Ok(None)
    }

    fn create_desktop_entry(
        &self,
        metadata: &Metadata,
        icon_path: &Option<PathBuf>,
        desktop_path: &Path,
    ) -> Result<(), ProcessError> {
        let icon_str = icon_path
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_else(String::new);

        let entry = DesktopEntry::with_categories(
            metadata.name.clone(),
            desktop_path.display().to_string(),
            icon_str,
            metadata.categories.clone(),
        );

        if self.dry_run {
            info!("[DRY RUN] Would create desktop entry: {:?}", desktop_path);
            return Ok(());
        }

        debug!("Creating desktop entry: {:?}", desktop_path);
        fs::write(&desktop_path, entry.to_file_content())?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&desktop_path)?.permissions();
            perms.set_mode(0o644);
            fs::set_permissions(&desktop_path, perms)?;
        }

        Ok(())
    }

    fn create_symlink(&self, target: &Path, link_path: &Path) -> Result<(), ProcessError> {
        use std::os::unix::fs::symlink as unix_symlink;

        if link_path.exists() {
            fs::remove_file(link_path)?;
        }

        debug!("Creating symlink: {:?} -> {:?}", link_path, target);

        #[cfg(unix)]
        {
            unix_symlink(target, link_path)?;
        }

        #[cfg(not(unix))]
        {
            return Err(ProcessError::Io(std::io::Error::new(
                std::io::ErrorKind::Unsupported,
                "Symlinks not supported on this platform",
            )));
        }

        Ok(())
    }
}
