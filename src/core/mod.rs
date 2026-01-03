pub mod appimage;
pub mod metadata;
pub mod normalization;

pub use appimage::{AppImage, AppImageError};
pub use metadata::Metadata;
pub use normalization::normalize_appimage_name;
