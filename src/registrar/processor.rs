use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use thiserror::Error;
use tracing::{debug, error, info, instrument, warn};

use crate::core::{normalize_appimage_name, AppImage, AppImageError, Metadata, MetadataCache, VersionManager, VersionError};
use crate::registrar::desktop_entry::DesktopEntry;
use crate::registrar::icon_extractor;
use crate::security::SecurityChecker;

use rayon::prelude::*;

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

    #[error("Version error: {0}")]
    Version(#[from] VersionError),
}

#[derive(Debug)]
pub struct ProcessedApp {
    pub normalized_name: String,
    #[allow(dead_code)]
    pub appimage_path: PathBuf,
}

#[derive(Debug)]
pub struct ProcessReport {
    pub processed: Vec<ProcessedApp>,
    pub failed: Vec<(PathBuf, String)>,
    pub skipped: Vec<PathBuf>,
    pub processing_time: Duration,
    pub cached_hits: usize,
    pub parallel_workers: usize,
}

impl ProcessReport {
    pub fn new() -> Self {
        ProcessReport {
            processed: Vec::new(),
            failed: Vec::new(),
            skipped: Vec::new(),
            processing_time: Duration::default(),
            cached_hits: 0,
            parallel_workers: 1,
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
    #[allow(dead_code)]
    pub bin_dir: PathBuf,
    pub icon_dir: PathBuf,
    pub desktop_dir: PathBuf,
    pub symlink_dir: PathBuf,
    pub version_manager: VersionManager,
    pub security_checker: SecurityChecker,
    pub dry_run: bool,
    pub cache: Option<Arc<Mutex<MetadataCache>>>,
    pub parallel_enabled: bool,
    pub incremental_scan: bool,
    pub last_scan_time: Option<u64>,
}

impl Processor {
    pub fn new(
        raw_dir: PathBuf,
        bin_dir: PathBuf,
        icon_dir: PathBuf,
        desktop_dir: PathBuf,
        symlink_dir: PathBuf,
        version_manager: VersionManager,
        security_checker: SecurityChecker,
    ) -> Self {
        Processor {
            raw_dir,
            bin_dir,
            icon_dir,
            desktop_dir,
            symlink_dir,
            version_manager,
            security_checker,
            dry_run: false,
            cache: None,
            parallel_enabled: true,
            incremental_scan: true,
            last_scan_time: None,
        }
    }

    pub fn with_performance_config(
        mut self,
        cache_dir: Option<PathBuf>,
        parallel_enabled: bool,
        incremental_scan: bool,
        last_scan_time: Option<u64>,
    ) -> Self {
        if let Some(cache_dir) = cache_dir {
            self.cache = Some(Arc::new(Mutex::new(MetadataCache::new(&cache_dir))));
        }
        self.parallel_enabled = parallel_enabled;
        self.incremental_scan = incremental_scan;
        self.last_scan_time = last_scan_time;
        self
    }

    #[allow(dead_code)]
    pub fn with_dry_run(mut self, dry_run: bool) -> Self {
        self.dry_run = dry_run;
        self
    }

    #[instrument(skip(self))]
    pub fn process_all(&self) -> Result<ProcessReport, ProcessError> {
        info!("Processing all AppImages in {:?}", self.raw_dir);

        let start_time = Instant::now();
        let mut report = ProcessReport::new();

        if !self.raw_dir.exists() {
            warn!("Raw directory does not exist: {:?}", self.raw_dir);
            return Ok(report);
        }

        // Collect AppImage paths
        let mut appimage_paths = Vec::new();
        for entry in fs::read_dir(&self.raw_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path
                .extension()
                .is_some_and(|e| e.eq_ignore_ascii_case("AppImage"))
            {
                // Incremental scan: skip if modified before last scan
                if self.incremental_scan && self.should_skip_incremental(&path)? {
                    report.skipped.push(path);
                    continue;
                }

                appimage_paths.push(path);
            }
        }

        // Process in parallel if enabled
        let processed_results = if self.parallel_enabled {
            self.process_parallel(appimage_paths)
        } else {
            self.process_sequential(appimage_paths)
        };

        // Collect results
        for result in processed_results {
            match result {
                Ok(processed) => {
                    info!("Processed: {}", processed.normalized_name);
                    report.processed.push(processed);
                }
                Err((path, e)) => {
                    error!("Failed to process {:?}: {}", path, e);
                    report.failed.push((path, e.to_string()));
                }
            }
        }

        report.processing_time = start_time.elapsed();
        report.parallel_workers = if self.parallel_enabled { rayon::current_num_threads() } else { 1 };

        if report.failed.is_empty() {
            info!(
                "Successfully processed {} AppImages in {:.2}s ({} cached hits)",
                report.success_count(),
                report.processing_time.as_secs_f64(),
                report.cached_hits
            );
        } else {
            error!("Completed with {} failures", report.failure_count());
        }

        // Save cache if enabled
        if let Some(ref cache) = self.cache {
            if let Ok(cache) = cache.lock() {
                if let Err(e) = cache.save() {
                    warn!("Failed to save metadata cache: {}", e);
                }
            }
        }

        Ok(report)
    }

    #[instrument(skip(self, app_path))]
    pub fn process_single_appimage(&self, app_path: &Path) -> Result<ProcessedApp, ProcessError> {
        let app = AppImage::new(app_path.to_path_buf())?;
        app.validate()?;

        // Perform security checks
        let security_report = self.security_checker.check_appimage(&app)
            .map_err(|e| ProcessError::DesktopEntry(format!("Security check failed: {}", e)))?;

        // Print warnings if any
        self.security_checker.print_warnings(&app, &security_report);

        let normalized_name =
            normalize_appimage_name(app_path.file_stem().and_then(|s| s.to_str()).unwrap_or(""));

        if normalized_name.is_empty() {
            return Err(ProcessError::DesktopEntry(
                "Empty normalized name".to_string(),
            ));
        }

        debug!("Processing AppImage: {:?} -> {}", app_path, normalized_name);

        if self.dry_run {
            info!("[DRY RUN] Would process: {}", normalized_name);
            return Ok(ProcessedApp {
                normalized_name: normalized_name.clone(),
                appimage_path: app_path.to_path_buf(),
            });
        }

        // Extract version from AppImage if possible
        let version = self.extract_version_from_appimage(app_path, &normalized_name);

        // Install using version manager
        self.version_manager.install_version(&normalized_name, &version, app_path)?;

        // Extract metadata and create desktop entry
        let (metadata, icon_path) = self.extract_metadata(app_path, &normalized_name)?;

        let current_appimage = self.version_manager.get_appimage_path(&normalized_name, &version);
        let symlink_path = self.symlink_dir.join(&normalized_name);
        self.create_symlink(&current_appimage, &symlink_path)?;

        let desktop_path = self
            .desktop_dir
            .join(format!("{}.desktop", normalized_name));
        self.create_desktop_entry(&metadata, &icon_path, &symlink_path, &desktop_path)?;

        info!("Running appimage-update check for {}", normalized_name);
        let _ = Command::new(&current_appimage).arg("--appimage-update").output();

        Ok(ProcessedApp {
            normalized_name,
            appimage_path: app_path.to_path_buf(),
        })
    }

    fn extract_version_from_appimage(&self, app_path: &Path, normalized_name: &str) -> String {
        // Try to extract version from filename
        let filename = app_path.file_stem().and_then(|s| s.to_str()).unwrap_or("");

        // Look for version patterns like -v1.2.3, -1.0.0, v2
        if let Some(pos) = filename.rfind('-') {
            let potential_version = &filename[pos + 1..];
            let version = potential_version
                .strip_prefix('v')
                .unwrap_or(potential_version);
            if version.chars().all(|c| c.is_numeric() || c == '.') {
                return version.to_string();
            }
        }

        // If no version found, try to get from AppImage itself
        if let Ok(_app) = AppImage::new(app_path.to_path_buf()) {
            // For now, use a timestamp-based version for new apps
            // In the future, this could extract version from AppImage metadata
            use chrono::Utc;
            format!("{}-{}", normalized_name, Utc::now().format("%Y%m%d%H%M%S"))
        } else {
            // Fallback
            "latest".to_string()
        }
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
            .stdout(Stdio::null())
            .stderr(Stdio::null())
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
        let checksum = app.get_checksum().map_err(ProcessError::AppImage)?;

        let desktop_file = self.find_desktop_entry(&app_root)?;
        let icon_path = icon_extractor::extract_icon(&app_root, &self.icon_dir, normalized_name)
            .map_err(|e| {
                ProcessError::Io(std::io::Error::other(
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

            if path.is_file()
                && let Some(ext) = path.extension()
                    && ext == "desktop" {
                        return Ok(Some(path));
                    }
        }

        Ok(None)
    }

    fn create_desktop_entry(
        &self,
        metadata: &Metadata,
        icon_path: &Option<PathBuf>,
        exec_path: &Path,
        desktop_path: &Path,
    ) -> Result<(), ProcessError> {
        let icon_str = icon_path
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_default();

        let entry = DesktopEntry::with_categories(
            metadata.name.clone(),
            exec_path.display().to_string(),
            icon_str,
            metadata.categories.clone(),
        );

        if self.dry_run {
            info!("[DRY RUN] Would create desktop entry: {:?}", desktop_path);
            return Ok(());
        }

        debug!("Creating desktop entry: {:?}", desktop_path);
        fs::write(desktop_path, entry.to_file_content())?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(desktop_path)?.permissions();
            perms.set_mode(0o644);
            fs::set_permissions(desktop_path, perms)?;
        }

        Ok(())
    }

    fn should_skip_incremental(&self, path: &Path) -> Result<bool, ProcessError> {
        if let Some(last_scan) = self.last_scan_time {
            if let Ok(metadata) = fs::metadata(path) {
                if let Ok(modified) = metadata.modified() {
                    let mtime = modified.duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs();
                    return Ok(mtime < last_scan);
                }
            }
        }
        Ok(false)
    }

    fn process_parallel(&self, paths: Vec<PathBuf>) -> Vec<Result<ProcessedApp, (PathBuf, ProcessError)>> {
        paths.into_par_iter()
            .map(|path| match self.process_single_appimage_cached(&path) {
                Ok(app) => Ok(app),
                Err(e) => Err((path.clone(), e)),
            })
            .collect()
    }

    fn process_sequential(&self, paths: Vec<PathBuf>) -> Vec<Result<ProcessedApp, (PathBuf, ProcessError)>> {
        paths.into_iter()
            .map(|path| match self.process_single_appimage_cached(&path) {
                Ok(app) => Ok(app),
                Err(e) => Err((path, e)),
            })
            .collect()
    }

    fn process_single_appimage_cached(&self, app_path: &Path) -> Result<ProcessedApp, ProcessError> {
        let app = AppImage::new(app_path.to_path_buf())?;
        let checksum = app.get_checksum().map_err(ProcessError::AppImage)?;

        // Check cache
        if let Some(ref cache) = self.cache {
            if let Ok(cache) = cache.lock() {
                if cache.is_cached(app_path, &checksum) {
                    if let Some(cached) = cache.get_cached_entry(app_path) {
                        debug!("Cache hit for: {:?}", app_path);
                        if self.cache_entry_is_usable(&cached.normalized_name) {
                            return Ok(ProcessedApp {
                                normalized_name: cached.normalized_name.clone(),
                                appimage_path: app_path.to_path_buf(),
                            });
                        }
                        debug!(
                            "Cache hit requires repair for {} (desktop entry or symlink stale)",
                            cached.normalized_name
                        );
                    }
                }
            }
        }

        // Process normally
        let result = self.process_single_appimage(app_path)?;

        // Update cache
        if let Some(ref cache) = self.cache {
            if let Ok(mut cache) = cache.lock() {
                if let Ok(metadata) = fs::metadata(app_path) {
                    if let Ok(mtime) = metadata.modified() {
                        let mtime_secs = mtime.duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_secs();
                        let normalized_name = normalize_appimage_name(
                            app_path.file_stem().and_then(|s| s.to_str()).unwrap_or("")
                        );
                        let version = self.extract_version_from_appimage(app_path, &normalized_name);
                        cache.add_entry(app_path, checksum, mtime_secs, normalized_name, version);
                    }
                }
            }
        }

        Ok(result)
    }

    fn cache_entry_is_usable(&self, normalized_name: &str) -> bool {
        let desktop_path = self.desktop_dir.join(format!("{}.desktop", normalized_name));
        let symlink_path = self.symlink_dir.join(normalized_name);
        if !desktop_path.exists() || !symlink_path.exists() {
            return false;
        }

        let expected_exec = format!("Exec={}", symlink_path.display());
        match fs::read_to_string(&desktop_path) {
            Ok(content) => content.lines().any(|line| line.trim() == expected_exec),
            Err(_) => false,
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn desktop_entry_exec_points_to_symlink_target() {
        let temp = TempDir::new().unwrap();
        let raw_dir = temp.path().join("raw");
        let bin_dir = temp.path().join("bin");
        let icon_dir = temp.path().join("icons");
        let desktop_dir = temp.path().join("desktop");
        let symlink_dir = temp.path().join("symlinks");

        fs::create_dir_all(&raw_dir).unwrap();
        fs::create_dir_all(&bin_dir).unwrap();
        fs::create_dir_all(&icon_dir).unwrap();
        fs::create_dir_all(&desktop_dir).unwrap();
        fs::create_dir_all(&symlink_dir).unwrap();

        let mut config = Config::default();
        config.directories.raw = raw_dir.display().to_string();
        config.directories.bin = bin_dir.display().to_string();
        config.directories.icons = icon_dir.display().to_string();
        config.directories.desktop = desktop_dir.display().to_string();
        config.directories.symlink = symlink_dir.display().to_string();

        let processor = Processor::new(
            raw_dir,
            bin_dir,
            icon_dir,
            desktop_dir.clone(),
            symlink_dir.clone(),
            VersionManager::new(config),
            SecurityChecker::new(),
        );

        let metadata = Metadata::new("Test App".to_string(), "checksum".to_string());
        let exec_path = symlink_dir.join("test-app");
        let desktop_path = desktop_dir.join("test-app.desktop");

        processor
            .create_desktop_entry(&metadata, &None, &exec_path, &desktop_path)
            .unwrap();

        let content = fs::read_to_string(desktop_path).unwrap();
        assert!(content.contains(&format!("Exec={}", exec_path.display())));
        assert!(!content.contains("Exec=/usr/share/applications/"));
    }

    #[test]
    fn cache_entry_is_usable_requires_expected_exec_and_symlink() {
        let temp = TempDir::new().unwrap();
        let raw_dir = temp.path().join("raw");
        let bin_dir = temp.path().join("bin");
        let icon_dir = temp.path().join("icons");
        let desktop_dir = temp.path().join("desktop");
        let symlink_dir = temp.path().join("symlinks");

        fs::create_dir_all(&raw_dir).unwrap();
        fs::create_dir_all(&bin_dir).unwrap();
        fs::create_dir_all(&icon_dir).unwrap();
        fs::create_dir_all(&desktop_dir).unwrap();
        fs::create_dir_all(&symlink_dir).unwrap();

        let mut config = Config::default();
        config.directories.raw = raw_dir.display().to_string();
        config.directories.bin = bin_dir.display().to_string();
        config.directories.icons = icon_dir.display().to_string();
        config.directories.desktop = desktop_dir.display().to_string();
        config.directories.symlink = symlink_dir.display().to_string();

        let processor = Processor::new(
            raw_dir,
            bin_dir,
            icon_dir,
            desktop_dir.clone(),
            symlink_dir.clone(),
            VersionManager::new(config),
            SecurityChecker::new(),
        );

        let name = "demoapp";
        let symlink_path = symlink_dir.join(name);
        let target = temp.path().join("demoapp.AppImage");
        fs::write(&target, b"fake").unwrap();
        std::os::unix::fs::symlink(&target, &symlink_path).unwrap();

        let desktop_path = desktop_dir.join(format!("{}.desktop", name));
        fs::write(
            &desktop_path,
            format!(
                "[Desktop Entry]\nType=Application\nName=Demo\nExec={}\n",
                symlink_path.display()
            ),
        )
        .unwrap();
        assert!(processor.cache_entry_is_usable(name));

        fs::write(
            &desktop_path,
            format!(
                "[Desktop Entry]\nType=Application\nName=Demo\nExec={}\n",
                desktop_path.display()
            ),
        )
        .unwrap();
        assert!(!processor.cache_entry_is_usable(name));
    }
}



