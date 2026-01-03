use std::path::Path;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum CollisionError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Failed to find unique name for {base}")]
    NoUniqueName { base: String },
}

pub fn handle_collision(_source: &Path, dest: &Path) -> Result<std::path::PathBuf, CollisionError> {
    let stem =
        dest.file_stem()
            .and_then(|s| s.to_str())
            .ok_or_else(|| CollisionError::NoUniqueName {
                base: dest.display().to_string(),
            })?;

    let extension = dest.extension().and_then(|e| e.to_str()).unwrap_or("");

    let mut counter = 1;
    loop {
        let new_name = format!("{}-{}", stem, counter);
        let new_path = dest
            .parent()
            .unwrap_or(Path::new("/"))
            .join(&new_name)
            .with_extension(extension);

        if !new_path.exists() {
            return Ok(new_path);
        }

        counter += 1;

        if counter > 10000 {
            return Err(CollisionError::NoUniqueName {
                base: dest.display().to_string(),
            });
        }
    }
}
