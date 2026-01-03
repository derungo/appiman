use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use thiserror::Error;
use tracing::{info, debug, warn, error, instrument};

use crate::core::{AppImage, normalize_appimage_name, Metadata};
use crate::registrar::desktop_entry::DesktopEntry;
use crate::registrar::icon_extractor::extract_icon;
use crate::registrar::symlink::create_symlink;

#[derive(Debug, Error)]
pub enum ProcessError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("AppImage error: {0}")]
    AppImage(#[from] crate::core::AppImageError),

    #[error("Extraction failed: {0}")]
    ExtractionFailed(String),

    #[error("Desktop entry error: {0}")]
    DesktopEntry(String),

    #[error("Icon extraction error: {0}")]
    IconExtract(#[from] crate::registrar::icon_extractor::IconExtractError),

    #[error("Symlink error: {0}")]
    Symlink(#[from] crate::registrar::symlink::SymlinkError),
}

#[derive(Debug)]
pub struct ProcessedApp {
    pub normalized_name: String,
    pub appimage_path: PathBuf,
    pub icon_path: Option<PathBuf>,
    pub desktop_path: PathBuf,
    pub symlink_path: PathBuf,
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

    pub fn has_failures(&self) -> bool {
        !self.failed.is_empty()
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

            if path.extension().map_or(false, |e| e.eq_ignore_ascii_case("AppImage")) {
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

        if report.has_failures() {
            error!("Completed with {} failures", report.failure_count());
        } else {
            info!("Successfully processed {} AppImages", report.success_count());
        }

        Ok(report)
    }

    #[instrument(skip(self, app_path))]
    pub fn process_single_appimage(&self, app_path: &Path) -> Result<ProcessedApp, ProcessError> {
        let app = AppImage::new(app_path.to_path_buf())?;
        app.validate()?;

        let normalized_name = normalize_appimage_name(
            app_path.file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("")
        );

        if normalized_name.is_empty() {
            return Err(ProcessError::DesktopEntry(
                "Empty normalized name".to_string()
            ));
        }

        let normalized_name_clone = normalized_name.clone();
        debug!("Processing AppImage: {:?} -> {}", app_path, normalized_name);

        let dest = self.bin_dir.join(format!("{}.AppImage", normalized_name));

        if self.dry_run {
            info!("[DRY RUN] Would process: {}", normalized_name_clone);
            return Ok(self.build_processed_app(
                normalized_name_clone,
                app_path.to_path_buf(),
                None,
                self.desktop_dir.join(format!("{}.desktop", normalized_name_clone)),
                self.symlink_dir.join(&normalized_name_clone),
            ));
        }

        self.copy_appimage(&app_path, &dest)?;
        self.make_executable(&dest)?;

        let metadata = self.extract_metadata(app_path, &normalized_name_clone)?;

        let icon_path = extract_icon(app_path.parent().unwrap(), &self.icon_dir, &normalized_name_clone)?;

        let desktop_path = self.create_desktop_entry(&metadata, &dest, icon_path.as_deref(), &normalized_name_clone)?;

        let symlink_path = self.symlink_dir.join(&normalized_name_clone);
        create_symlink(&dest, &symlink_path)?;

        info!("Running appimage-update check for {}", normalized_name_clone);
        let _ = Command::new(&dest)
            .arg("--appimage-update")
            .output();

        Ok(self.build_processed_app(
            normalized_name_clone,
            app_path.to_path_buf(),
            icon_path,
            desktop_path,
            symlink_path,
        ))
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

    fn extract_metadata(&self, app_path: &Path, normalized_name: &str) -> Result<Metadata, ProcessError> {
        let tmp_dir = tempfile::TempDir::new()?;
        let app_root = tmp_dir.path().join("squashfs-root");

        debug!("Extracting AppImage: {:?}", app_path);

        let output = Command::new(app_path)
            .arg("--appimage-extract")
            .current_dir(tmp_dir.path())
            .output();

        if !output.status.success() {
            return Err(ProcessError::ExtractionFailed(
                format!("AppImage extract failed: {}", output.status)
            ));
        }

        if !app_root.exists() {
            return Err(ProcessError::ExtractionFailed(
                "squashfs-root not found after extraction".to_string()
            ));
        }

        let desktop_file = self.find_desktop_entry(&app_root)?;

        match desktop_file {
            Some(path) => {
                debug!("Found desktop entry: {:?}", path);
                Metadata::from_desktop_entry(&path).map_err(|e| ProcessError::DesktopEntry(e.to_string()))?
            }
            None => {
                debug!("No desktop entry found, using defaults");
                let mut metadata = Metadata::new(
                    normalized_name.chars().next()
                        .unwrap()
                        .to_uppercase()
                        .collect::<String>() + &normalized_name[1..]
                );
                metadata.name = format!("{}{}",
                    normalized_name.chars().next().unwrap().to_uppercase(),
                    &normalized_name[1..]
                );
                Ok(metadata)
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
        exec_path: &Path,
        icon_path: Option<&Path>,
        normalized_name: &str,
    ) -> Result<PathBuf, ProcessError> {
        let desktop_path = self.desktop_dir.join(format!("{}.desktop", normalized_name));

        let icon = match icon_path {
            Some(p) => p.display().to_string(),
            None => "".to_string(),
        };

        let entry = DesktopEntry::with_categories(
            metadata.name.clone(),
            exec_path.display().to_string(),
            icon,
            metadata.categories.clone(),
        );

        if self.dry_run {
            info!("[DRY RUN] Would create desktop entry: {:?}", desktop_path);
            return Ok(desktop_path);
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

        Ok(desktop_path)
    }

    fn build_processed_app(
        &self,
        normalized_name: String,
        appimage_path: PathBuf,
        icon_path: Option<PathBuf>,
        desktop_path: PathBuf,
        symlink_path: PathBuf,
    ) -> ProcessedApp {
        ProcessedApp {
            normalized_name,
            appimage_path,
            icon_path,
            desktop_path,
            symlink_path,
        }
    }
}
