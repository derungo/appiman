#[derive(Debug, Clone)]
pub struct DesktopEntry {
    pub name: String,
    pub exec_path: String,
    pub icon_path: String,
    pub terminal: bool,
    pub categories: Vec<String>,
}

impl DesktopEntry {
    #[allow(dead_code)]
    pub fn new(name: String, exec_path: String, icon_path: String) -> Self {
        DesktopEntry {
            name,
            exec_path,
            icon_path,
            terminal: false,
            categories: vec!["Utility".to_string()],
        }
    }

    pub fn with_categories(
        name: String,
        exec_path: String,
        icon_path: String,
        categories: Vec<String>,
    ) -> Self {
        DesktopEntry {
            name,
            exec_path,
            icon_path,
            terminal: false,
            categories,
        }
    }

    pub fn to_file_content(&self) -> String {
        let name = sanitize_desktop_value(&self.name);
        let exec_path = sanitize_desktop_value(&self.exec_path);
        let icon_path = sanitize_desktop_value(&self.icon_path);
        let categories = self
            .categories
            .iter()
            .map(|c| sanitize_desktop_value(c))
            .collect::<Vec<_>>()
            .join(";");

        format!(
            "[Desktop Entry]\n\
            Type=Application\n\
            Name={}\n\
            Exec={}\n\
            Icon={}\n\
            Terminal={}\n\
            Categories={}\n",
            name,
            exec_path,
            icon_path,
            if self.terminal { "true" } else { "false" },
            categories
        )
    }
}

fn sanitize_desktop_value(value: &str) -> String {
    value.replace('\\', "\\\\").replace(['\n', '\r'], " ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn desktop_entry_generates_valid_content() {
        let entry = DesktopEntry::new(
            "Test App".to_string(),
            "/opt/applications/bin/testapp.AppImage".to_string(),
            "/opt/applications/icons/testapp.png".to_string(),
        );

        let content = entry.to_file_content();

        assert!(content.contains("[Desktop Entry]"));
        assert!(content.contains("Type=Application"));
        assert!(content.contains("Name=Test App"));
        assert!(content.contains("Exec=/opt/applications/bin/testapp.AppImage"));
        assert!(content.contains("Icon=/opt/applications/icons/testapp.png"));
        assert!(content.contains("Terminal=false"));
        assert!(content.contains("Categories=Utility"));
    }

    #[test]
    fn desktop_entry_with_categories() {
        let entry = DesktopEntry::with_categories(
            "Test App".to_string(),
            "/opt/applications/bin/testapp.AppImage".to_string(),
            "/opt/applications/icons/testapp.png".to_string(),
            vec!["Utility".to_string(), "Office".to_string()],
        );

        let content = entry.to_file_content();

        assert!(content.contains("Categories=Utility;Office"));
    }

    #[test]
    fn desktop_entry_sanitizes_newlines() {
        let entry = DesktopEntry::with_categories(
            "Bad\nName".to_string(),
            "/usr/local/bin/safe\nexec".to_string(),
            "icon\rpath".to_string(),
            vec!["Utility\nInjected".to_string()],
        );

        let content = entry.to_file_content();

        assert!(!content.contains("\nName=Bad\nName"));
        assert!(!content.contains("\nExec=/usr/local/bin/safe\nexec"));
        assert!(content.contains("Name=Bad Name"));
        assert!(content.contains("Exec=/usr/local/bin/safe exec"));
        assert!(content.contains("Icon=icon path"));
    }
}
