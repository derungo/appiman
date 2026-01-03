#[cfg(unix)]
mod move_appimages_sh {
    use std::fs;
    use std::path::PathBuf;
    use std::process::{Command, Output};

    use tempfile::TempDir;

    fn script_path() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("assets")
            .join("move-appimages.sh")
    }

    fn run_move_script(home_root: &PathBuf, raw_dir: &PathBuf) -> Output {
        Command::new("bash")
            .arg(script_path())
            .env("HOME_ROOT", home_root)
            .env("RAW_DIR", raw_dir)
            .output()
            .expect("run move-appimages.sh")
    }

    #[test]
    fn moves_appimages_from_multiple_users_into_raw_dir() {
        let root = TempDir::new().unwrap();

        let home_root = root.path().join("home");
        let raw_dir = root.path().join("raw");

        let alice = home_root.join("alice");
        let bob = home_root.join("bob");

        fs::create_dir_all(&alice).unwrap();
        fs::create_dir_all(&bob).unwrap();
        fs::create_dir_all(&raw_dir).unwrap();

        fs::write(alice.join("Foo.AppImage"), b"foo").unwrap();
        fs::write(bob.join("Bar.AppImage"), b"bar").unwrap();
        fs::write(bob.join("not-an-appimage.txt"), b"ignored").unwrap();

        let output = run_move_script(&home_root, &raw_dir);
        assert!(
            output.status.success(),
            "stdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );

        assert!(raw_dir.join("Foo.AppImage").exists());
        assert!(raw_dir.join("Bar.AppImage").exists());

        assert!(!alice.join("Foo.AppImage").exists());
        assert!(!bob.join("Bar.AppImage").exists());

        assert!(bob.join("not-an-appimage.txt").exists());
    }

    #[test]
    fn avoids_overwriting_on_name_collision_by_renaming() {
        let root = TempDir::new().unwrap();

        let home_root = root.path().join("home");
        let raw_dir = root.path().join("raw");

        let alice = home_root.join("alice");
        let bob = home_root.join("bob");

        fs::create_dir_all(&alice).unwrap();
        fs::create_dir_all(&bob).unwrap();
        fs::create_dir_all(&raw_dir).unwrap();

        fs::write(alice.join("Same.AppImage"), b"alice").unwrap();
        fs::write(bob.join("Same.AppImage"), b"bob").unwrap();

        let output = run_move_script(&home_root, &raw_dir);
        assert!(
            output.status.success(),
            "stdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );

        assert!(raw_dir.join("Same.AppImage").exists());
        assert!(raw_dir.join("Same-1.AppImage").exists());

        assert_eq!(fs::read(raw_dir.join("Same.AppImage")).unwrap(), b"alice");
        assert_eq!(fs::read(raw_dir.join("Same-1.AppImage")).unwrap(), b"bob");
    }
}

#[cfg(not(unix))]
#[test]
fn move_appimages_sh_tests_skipped_on_non_unix() {
    eprintln!("Skipping move-appimages.sh tests on non-Unix platforms");
}
