pub mod desktop_entry;
pub mod icon_extractor;
pub mod processor;
pub mod symlink;

pub use desktop_entry::DesktopEntry;
pub use icon_extractor::extract_icon;
pub use processor::{ProcessReport, ProcessedApp, Processor};
pub use symlink::create_symlink;
