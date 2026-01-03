use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;
use tracing::debug;

#[derive(Debug, Error)]
pub enum IconExtractError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("No icon found in {path}")]
    NotFound { path: PathBuf },
}

pub fn extract_icon(
    app_dir: &Path,
    icon_dir: &Path,
    normalized_name: &str,
) -> Result<Option<PathBuf>, IconExtractError> {
    debug!("Extracting icon from {:?}", app_dir);

    let icon_path = find_icon_in_dir(app_dir)?;

    match icon_path {
        Some(src) => {
            let extension = src.extension().and_then(|e| e.to_str()).unwrap_or("png");

            let dest = icon_dir.join(format!("{}.{}", normalized_name, extension));

            if dest.exists() {
                debug!("Icon already exists: {:?}", dest);
                return Ok(Some(dest));
            }

            fs::copy(&src, &dest)?;

            debug!("Extracted icon: {:?} -> {:?}", src, dest);
            Ok(Some(dest))
        }
        None => {
            debug!("No icon found in {:?}", app_dir);
            Ok(None)
        }
    }
}

fn find_icon_in_dir(dir: &Path) -> Result<Option<PathBuf>, IconExtractError> {
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(e) => {
            if e.kind() == std::io::ErrorKind::NotFound {
                return Ok(None);
            }
            return Err(IconExtractError::Io(e));
        }
    };

    for entry in entries {
        let entry = entry?;
        let path = entry.path();

        if path.is_file() {
            if let Some(ext) = path.extension() {
                if ext.eq_ignore_ascii_case("png") || ext.eq_ignore_ascii_case("svg") {
                    return Ok(Some(path));
                }
            }
        }
    }

    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn extract_icon_finds_png() {
        let temp = TempDir::new().unwrap();
        let app_dir = temp.path().join("app");
        let icon_dir = temp.path().join("icons");
        fs::create_dir_all(&app_dir).unwrap();
        fs::create_dir_all(&icon_dir).unwrap();

        let icon = app_dir.join("icon.png");
        fs::write(&icon, b"fake icon").unwrap();

        let result = extract_icon(&app_dir, &icon_dir, "testapp").unwrap();

        assert!(result.is_some());
        assert!(result.unwrap().ends_with("testapp.png"));
    }

    #[test]
    fn extract_icon_finds_svg() {
        let temp = TempDir::new().unwrap();
        let app_dir = temp.path().join("app");
        let icon_dir = temp.path().join("icons");
        fs::create_dir_all(&app_dir).unwrap();
        fs::create_dir_all(&icon_dir).unwrap();

        let icon = app_dir.join("icon.svg");
        fs::write(&icon, b"fake icon").unwrap();

        let result = extract_icon(&app_dir, &icon_dir, "testapp").unwrap();

        assert!(result.is_some());
        assert!(result.unwrap().ends_with("testapp.svg"));
    }

    #[test]
    fn extract_icon_skips_non_image_files() {
        let temp = TempDir::new().unwrap();
        let app_dir = temp.path().join("app");
        let icon_dir = temp.path().join("icons");
        fs::create_dir_all(&app_dir).unwrap();
        fs::create_dir_all(&icon_dir).unwrap();

        let text_file = app_dir.join("readme.txt");
        fs::write(&text_file, b"readme").unwrap();

        let result = extract_icon(&app_dir, &icon_dir, "testapp").unwrap();

        assert!(result.is_none());
    }

    #[test]
    fn extract_icon_preserves_existing() {
        let temp = TempDir::new().unwrap();
        let app_dir = temp.path().join("app");
        let icon_dir = temp.path().join("icons");
        fs::create_dir_all(&app_dir).unwrap();
        fs::create_dir_all(&icon_dir).unwrap();

        let icon = app_dir.join("icon.png");
        fs::write(&icon, b"fake icon").unwrap();

        let existing_icon = icon_dir.join("testapp.png");
        fs::write(&existing_icon, b"existing").unwrap();

        let result = extract_icon(&app_dir, &icon_dir, "testapp").unwrap();

        assert!(result.is_some());
        assert_eq!(result.unwrap(), existing_icon);
    }

    #[test]
    fn extract_icon_returns_none_for_empty_dir() {
        let temp = TempDir::new().unwrap();
        let app_dir = temp.path().join("app");
        let icon_dir = temp.path().join("icons");
        fs::create_dir_all(&app_dir).unwrap();
        fs::create_dir_all(&icon_dir).unwrap();

        let result = extract_icon(&app_dir, &icon_dir, "testapp").unwrap();

        assert!(result.is_none());
    }
}
