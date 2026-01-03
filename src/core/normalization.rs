use regex::Regex;

lazy_static::lazy_static! {
    static ref VERSION_REGEX: Regex = Regex::new(r"-?v?[0-9]+(\.[0-9]+)*").unwrap();
    static ref ARCH_REGEX: Regex = Regex::new(r"(?i)(x86_64|amd64|i386|linux|setup)").unwrap();
    static ref SEPARATOR_REGEX: Regex = Regex::new(r"[-_.]+").unwrap();
}

pub fn normalize_appimage_name(name: &str) -> String {
    let normalized = name
        .to_lowercase()
        .replace(".appimage", "")
        .replace("_appimage", "");

    let without_arch = ARCH_REGEX.replace_all(&normalized, "");
    let without_version = VERSION_REGEX.replace_all(&without_arch, "");
    let normalized_separators = SEPARATOR_REGEX.replace_all(&without_version, "-");

    normalized_separators.trim_matches('-').trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_removes_version() {
        assert_eq!(normalize_appimage_name("TestApp-v1.2.3"), "testapp");
        assert_eq!(normalize_appimage_name("TestApp-1.0.0"), "testapp");
        assert_eq!(normalize_appimage_name("TestApp-v2"), "testapp");
    }

    #[test]
    fn normalize_removes_architecture() {
        assert_eq!(normalize_appimage_name("TestApp-x86_64"), "testapp");
        assert_eq!(normalize_appimage_name("TestApp-amd64"), "testapp");
        assert_eq!(normalize_appimage_name("TestApp-i386"), "testapp");
        assert_eq!(normalize_appimage_name("TestApp-linux"), "testapp");
    }

    #[test]
    fn normalize_removes_appimage_suffix() {
        assert_eq!(normalize_appimage_name("TestApp.AppImage"), "testapp");
        assert_eq!(normalize_appimage_name("TestApp_AppImage"), "testapp");
    }

    #[test]
    fn normalize_normalizes_separators() {
        assert_eq!(
            normalize_appimage_name("My_Application-v1.0.AppImage"),
            "my-application"
        );
        assert_eq!(
            normalize_appimage_name("My--Application___v1.0.AppImage"),
            "my-application"
        );
    }

    #[test]
    fn normalize_handles_complex_names() {
        assert_eq!(
            normalize_appimage_name("MyAwesomeApp-v2.1.0-x86_64.AppImage"),
            "myawesomeapp"
        );
        assert_eq!(
            normalize_appimage_name("MyAwesome_App-1.5.3_amd64.AppImage"),
            "myawesome-app"
        );
    }

    #[test]
    fn normalize_handles_empty_input() {
        assert_eq!(normalize_appimage_name(""), "");
        assert_eq!(normalize_appimage_name("  "), "");
    }

    #[test]
    fn normalize_preserves_valid_names() {
        assert_eq!(normalize_appimage_name("MyApplication"), "myapplication");
        assert_eq!(normalize_appimage_name("test-app"), "test-app");
    }
}
