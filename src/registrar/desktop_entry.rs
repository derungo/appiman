#[derive(Debug, Clone)]
pub struct DesktopEntry {
    pub name: String,
    pub exec_path: String,
    pub icon_path: String,
    pub terminal: bool,
    pub categories: Vec<String>,
}

impl DesktopEntry {
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
        format!(
            "[Desktop Entry]\n\
            Type=Application\n\
            Name={}\n\
            Exec={}\n\
            Icon={}\n\
            Terminal={}\n\
            Categories={}\n",
            self.name,
            self.exec_path,
            self.icon_path,
            if self.terminal { "true" } else { "false" },
            self.categories.join(";")
        )
    }
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
}
