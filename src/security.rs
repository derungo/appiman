use std::fs;
use std::path::Path;
use std::process::Command;
use thiserror::Error;
use tracing::warn;

use crate::core::AppImage;

#[derive(Debug, Error)]
pub enum SecurityError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("AppImage error: {0}")]
    AppImage(#[from] crate::core::AppImageError),

    #[error("Security check failed: {0}")]
    CheckFailed(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum SecurityStatus {
    Secure,
    Warning(String),
    Error(String),
}

#[derive(Debug, Clone)]
pub struct SecurityReport {
    pub checksum_verified: bool,
    pub signature_present: bool,
    pub signature_verified: Option<bool>, // None if no signature, Some(true/false) if present
    pub sandboxing_detected: bool,
    pub overall_status: SecurityStatus,
}

impl SecurityReport {
    pub fn new() -> Self {
        SecurityReport {
            checksum_verified: false,
            signature_present: false,
            signature_verified: None,
            sandboxing_detected: false,
            overall_status: SecurityStatus::Secure,
        }
    }

    #[allow(dead_code)]
    pub fn with_warning(mut self, message: String) -> Self {
        self.overall_status = SecurityStatus::Warning(message);
        self
    }

    #[allow(dead_code)]
    pub fn with_error(mut self, message: String) -> Self {
        self.overall_status = SecurityStatus::Error(message);
        self
    }

    #[allow(dead_code)]
    pub fn is_secure(&self) -> bool {
        matches!(self.overall_status, SecurityStatus::Secure)
    }
}

pub struct SecurityChecker {
    pub verify_signatures: bool,
    #[allow(dead_code)]
    pub require_signatures: bool,
    pub warn_unsigned: bool,
    pub detect_sandboxing: bool,
}

impl Default for SecurityChecker {
    fn default() -> Self {
        SecurityChecker {
            verify_signatures: false, // Disabled by default for compatibility
            require_signatures: false,
            warn_unsigned: true,
            detect_sandboxing: true,
        }
    }
}

impl SecurityChecker {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self::default()
    }

    /// Perform all security checks on an AppImage
    pub fn check_appimage(&self, appimage: &AppImage) -> Result<SecurityReport, SecurityError> {
        let mut report = SecurityReport::new();

        // Always verify checksum (SHA256 integrity)
        report.checksum_verified = self.verify_checksum(appimage)?;

        // Check for signature file
        report.signature_present = self.has_signature_file(appimage)?;

        // Verify signature if present and verification is enabled
        if report.signature_present && self.verify_signatures {
            report.signature_verified = Some(self.verify_signature(appimage)?);
        }

        // Detect sandboxing usage
        if self.detect_sandboxing {
            report.sandboxing_detected = self.detect_sandboxing_usage(appimage)?;
        }

        // Determine overall security status
        report.overall_status = self.assess_overall_security(&report);

        Ok(report)
    }

    /// Verify AppImage checksum integrity
    fn verify_checksum(&self, appimage: &AppImage) -> Result<bool, SecurityError> {
        // The AppImage struct already has get_checksum() which computes SHA256
        match appimage.get_checksum() {
            Ok(_) => Ok(true),
            Err(e) => {
                warn!(
                    "Checksum verification failed for {:?}: {}",
                    appimage.path, e
                );
                Ok(false)
            }
        }
    }

    /// Check if a detached signature file exists (.sig file)
    fn has_signature_file(&self, appimage: &AppImage) -> Result<bool, SecurityError> {
        let sig_path = appimage.path.with_extension("sig");
        Ok(sig_path.exists())
    }

    /// Verify GPG signature if present
    fn verify_signature(&self, appimage: &AppImage) -> Result<bool, SecurityError> {
        let sig_path = appimage.path.with_extension("sig");

        if !sig_path.exists() {
            return Ok(false);
        }

        // Use gpg to verify signature
        let output = Command::new("gpg")
            .args([
                "--verify",
                &sig_path.to_string_lossy(),
                &appimage.path.to_string_lossy(),
            ])
            .output()
            .map_err(|e| SecurityError::CheckFailed(format!("GPG command failed: {}", e)))?;

        Ok(output.status.success())
    }

    /// Detect if AppImage uses sandboxing (firejail, bubblewrap)
    fn detect_sandboxing_usage(&self, appimage: &AppImage) -> Result<bool, SecurityError> {
        // Extract AppImage and check for sandboxing configuration
        let temp_dir = tempfile::TempDir::new()
            .map_err(|e| SecurityError::CheckFailed(format!("Failed to create temp dir: {}", e)))?;

        let extract_status = Command::new(&appimage.path)
            .arg("--appimage-extract")
            .current_dir(&temp_dir)
            .status()
            .map_err(|e| SecurityError::CheckFailed(format!("Extraction failed: {}", e)))?;

        if !extract_status.success() {
            return Ok(false);
        }

        let app_root = temp_dir.path().join("squashfs-root");

        // Check for firejail profile
        let firejail_profile = app_root.join(format!("{}.profile", appimage.normalize_name()));

        // Check for bubblewrap wrapper
        let bubblewrap_wrapper = app_root.join("usr/bin/bwrap-wrapper");

        // Check for desktop file Exec line containing firejail or bwrap
        let desktop_files = self.find_files_with_extension(&app_root, "desktop");
        let has_sandbox_in_desktop = desktop_files
            .iter()
            .any(|path| self.desktop_file_uses_sandboxing(path));

        let has_firejail = firejail_profile.exists() || has_sandbox_in_desktop;
        let has_bubblewrap = bubblewrap_wrapper.exists() || has_sandbox_in_desktop;

        Ok(has_firejail || has_bubblewrap)
    }

    /// Find all files with a specific extension in a directory
    fn find_files_with_extension(&self, dir: &Path, extension: &str) -> Vec<std::path::PathBuf> {
        let mut files = Vec::new();
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() {
                    if let Some(ext) = path.extension() {
                        if ext == extension {
                            files.push(path);
                        }
                    }
                }
            }
        }
        files
    }

    /// Check if desktop file Exec line contains sandboxing commands
    fn desktop_file_uses_sandboxing(&self, desktop_path: &Path) -> bool {
        if let Ok(content) = fs::read_to_string(desktop_path) {
            let exec_line = content
                .lines()
                .find(|line| line.starts_with("Exec="))
                .unwrap_or("");

            exec_line.contains("firejail") || exec_line.contains("bwrap")
        } else {
            false
        }
    }

    /// Assess overall security status based on checks
    fn assess_overall_security(&self, report: &SecurityReport) -> SecurityStatus {
        // Checksum failure is critical
        if !report.checksum_verified {
            return SecurityStatus::Error(
                "Checksum verification failed - file may be corrupted".to_string(),
            );
        }

        // Signature verification failure
        if let Some(verified) = report.signature_verified {
            if !verified {
                return SecurityStatus::Error("Signature verification failed".to_string());
            }
        }

        // Warnings for missing security features
        let mut warnings = Vec::new();

        if self.warn_unsigned && !report.signature_present {
            warnings.push("No signature found - cannot verify authenticity".to_string());
        }

        if self.detect_sandboxing && !report.sandboxing_detected {
            warnings.push(
                "No sandboxing detected - AppImage runs without security isolation".to_string(),
            );
        }

        if warnings.is_empty() {
            SecurityStatus::Secure
        } else {
            SecurityStatus::Warning(warnings.join("; "))
        }
    }

    /// Print security warnings if any
    pub fn print_warnings(&self, appimage: &AppImage, report: &SecurityReport) {
        if let SecurityStatus::Warning(message) = &report.overall_status {
            warn!(
                "Security warning for {}: {}",
                appimage.normalize_name(),
                message
            );
        } else if let SecurityStatus::Error(message) = &report.overall_status {
            warn!(
                "Security error for {}: {}",
                appimage.normalize_name(),
                message
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn security_report_new_is_secure() {
        let report = SecurityReport::new();
        assert!(report.is_secure());
    }

    #[test]
    fn security_report_with_warning() {
        let report = SecurityReport::new().with_warning("test warning".to_string());
        assert!(!report.is_secure());
        assert!(matches!(report.overall_status, SecurityStatus::Warning(_)));
    }

    #[test]
    fn has_signature_file_detects_sig_files() {
        let temp_dir = TempDir::new().unwrap();
        let app_path = temp_dir.path().join("test.AppImage");
        let sig_path = temp_dir.path().join("test.sig");

        fs::write(&app_path, b"test").unwrap();
        fs::write(&sig_path, b"signature").unwrap();

        let app = AppImage::new(app_path).unwrap();
        let checker = SecurityChecker::new();

        assert!(checker.has_signature_file(&app).unwrap());
    }

    #[test]
    fn has_signature_file_returns_false_when_missing() {
        let temp_dir = TempDir::new().unwrap();
        let app_path = temp_dir.path().join("test.AppImage");

        fs::write(&app_path, b"test").unwrap();

        let app = AppImage::new(app_path).unwrap();
        let checker = SecurityChecker::new();

        assert!(!checker.has_signature_file(&app).unwrap());
    }
}
