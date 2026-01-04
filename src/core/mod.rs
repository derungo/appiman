pub mod appimage;
pub mod cache;
pub mod metadata;
pub mod normalization;
pub mod version_manager;

pub use appimage::{AppImage, AppImageError};
pub use cache::{CacheError, MetadataCache};
pub use metadata::{AppMetadata, Metadata, VersionInfo};
pub use normalization::normalize_appimage_name;
pub use version_manager::{VersionError, VersionManager};
