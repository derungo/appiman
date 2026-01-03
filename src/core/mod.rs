pub mod appimage;
pub mod metadata;
pub mod normalization;

pub use appimage::AppImage;
pub use metadata::Metadata;
pub use normalization::normalize_appimage_name;
