#[cfg(unix)]
mod register_appimages_sh {
    use std::fs;
    use std::os::unix::fs::{symlink, PermissionsExt};
    use std::path::{Path, PathBuf};
    use std::process::{Command, Output};

    use tempfile::TempDir;

    struct EnvDirs {
        raw_dir: PathBuf,
        bin_dir: PathBuf,
        icon_dir: PathBuf,
        desktop_dir: PathBuf,
        symlink_dir: PathBuf,
    }

    fn script_path() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("assets")
            .join("register-appimages.sh")
    }

    fn make_env_dirs(root: &Path) -> EnvDirs {
        let raw_dir = root.join("raw");
        let bin_dir = root.join("bin");
        let icon_dir = root.join("icons");
        let desktop_dir = root.join("applications");
        let symlink_dir = root.join("symlinks");

        fs::create_dir_all(&raw_dir).unwrap();
        fs::create_dir_all(&bin_dir).unwrap();
        fs::create_dir_all(&icon_dir).unwrap();
        fs::create_dir_all(&desktop_dir).unwrap();
        fs::create_dir_all(&symlink_dir).unwrap();

        EnvDirs {
            raw_dir,
            bin_dir,
            icon_dir,
            desktop_dir,
            symlink_dir,
        }
    }

    fn write_executable(path: &Path, contents: &str) {
        fs::write(path, contents).expect("write executable file");
        let mut perms = fs::metadata(path).expect("metadata").permissions();
        perms.set_mode(0o755);
        fs::set_permissions(path, perms).expect("set permissions");
    }

    fn write_fake_appimage(raw_dir: &Path) -> PathBuf {
        let app_path = raw_dir.join("TestApp-v1.2.3-x86_64.AppImage");
        let contents = r#"#!/usr/bin/env bash
set -euo pipefail
case "${1:-}" in
  --appimage-extract)
    mkdir -p squashfs-root
    cat > squashfs-root/testapp.desktop <<'EOF'
[Desktop Entry]
Name=Test App
Categories=Utility;
Icon=testapp
EOF
    printf 'not-a-real-png' > squashfs-root/testapp.png
    ;;
  --appimage-update)
    ;;
  *)
    ;;
esac
"#;
        write_executable(&app_path, contents);
        app_path
    }

    fn run_register_script(args: &[&str], env_dirs: &EnvDirs, current_dir: &Path) -> Output {
        Command::new("bash")
            .arg(script_path())
            .args(args)
            .current_dir(current_dir)
            .env("RAW_DIR", &env_dirs.raw_dir)
            .env("BIN_DIR", &env_dirs.bin_dir)
            .env("ICON_DIR", &env_dirs.icon_dir)
            .env("DESKTOP_DIR", &env_dirs.desktop_dir)
            .env("SYMLINK_DIR", &env_dirs.symlink_dir)
            .output()
            .expect("run register-appimages.sh")
    }

    fn assert_file_contains(path: &Path, needle: &str) {
        let content = fs::read_to_string(path).expect("read file");
        assert!(
            content.contains(needle),
            "expected {} to contain {needle:?}, got:\n{content}",
            path.display()
        );
    }

    #[test]
    fn registers_appimage_even_when_cwd_is_readonly() {
        let root = TempDir::new().unwrap();
        let dirs = make_env_dirs(root.path());

        write_fake_appimage(&dirs.raw_dir);

        let work_dir = root.path().join("workdir");
        fs::create_dir_all(&work_dir).unwrap();
        let mut perms = fs::metadata(&work_dir).unwrap().permissions();
        perms.set_mode(0o555);
        fs::set_permissions(&work_dir, perms).unwrap();

        let output = run_register_script(&[], &dirs, &work_dir);

        let clean = "testapp";
        let dest = dirs.bin_dir.join(format!("{clean}.AppImage"));
        let icon = dirs.icon_dir.join(format!("{clean}.png"));
        let desktop = dirs.desktop_dir.join(format!("{clean}.desktop"));
        let link = dirs.symlink_dir.join(clean);

        let missing = [
            (!dest.exists()).then_some(format!("missing dest: {}", dest.display())),
            (!icon.exists()).then_some(format!("missing icon: {}", icon.display())),
            (!desktop.exists()).then_some(format!("missing desktop: {}", desktop.display())),
            (!link.exists()).then_some(format!("missing link: {}", link.display())),
        ]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();

        if !missing.is_empty() {
            panic!(
                "{}\nstdout:\n{}\nstderr:\n{}",
                missing.join("\n"),
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
        }

        assert_file_contains(&desktop, "Name=Test App");
        assert_file_contains(&desktop, &format!("Exec={}", dest.display()));
        assert_file_contains(&desktop, &format!("Icon={}", icon.display()));
    }

    #[test]
    fn clean_removes_versioned_bins_icons_symlinks_and_desktops() {
        let root = TempDir::new().unwrap();
        let dirs = make_env_dirs(root.path());

        fs::write(dirs.bin_dir.join("testapp.AppImage"), "ok").unwrap();
        fs::write(dirs.bin_dir.join("testapp-v1.0.AppImage"), "legacy").unwrap();

        fs::write(dirs.icon_dir.join("testapp.png"), "ok").unwrap();
        fs::write(dirs.icon_dir.join("testapp-v1.0.png"), "legacy").unwrap();

        let good_target = dirs.bin_dir.join("testapp.AppImage");
        symlink(&good_target, dirs.symlink_dir.join("testapp")).unwrap();

        let broken_target = dirs.bin_dir.join("testapp-v1.0.AppImage.missing");
        symlink(&broken_target, dirs.symlink_dir.join("testapp-v1")).unwrap();

        fs::write(
            dirs.desktop_dir.join("testapp.desktop"),
            format!(
                "[Desktop Entry]\nType=Application\nName=Test App\nExec={}\n",
                good_target.display()
            ),
        )
        .unwrap();

        let legacy_desktop_path = dirs.desktop_dir.join("testapp-v1.desktop");
        fs::write(
            &legacy_desktop_path,
            format!(
                "[Desktop Entry]\nType=Application\nName=Test App v1\nExec={}\n",
                dirs.bin_dir.join("testapp-v1.0.AppImage").display()
            ),
        )
        .unwrap();

        let work_dir = root.path().join("workdir");
        fs::create_dir_all(&work_dir).unwrap();

        run_register_script(&["--clean"], &dirs, &work_dir);

        assert!(dirs.bin_dir.join("testapp.AppImage").exists());
        assert!(!dirs.bin_dir.join("testapp-v1.0.AppImage").exists());

        assert!(dirs.icon_dir.join("testapp.png").exists());
        assert!(!dirs.icon_dir.join("testapp-v1.0.png").exists());

        assert!(dirs.symlink_dir.join("testapp").exists());
        assert!(!dirs.symlink_dir.join("testapp-v1").exists());

        assert!(dirs.desktop_dir.join("testapp.desktop").exists());
        assert!(!legacy_desktop_path.exists());
    }
}

#[cfg(not(unix))]
#[test]
fn register_appimages_sh_tests_skipped_on_non_unix() {
    eprintln!("Skipping register-appimages.sh tests on non-Unix platforms");
}
