appiman

Appiman is a compact Rust utility for system-wide AppImage lifecycle management on multi-user Linux workstations.
It centralizes discovery, registration, updates, and cleanup of AppImages—without requiring AppImageLauncher, manual .desktop file creation, or scattered downloads in $HOME.

Appiman ships with opinionated helper scripts and systemd units that:

sweep users' home directories for newly downloaded .AppImage files

ingest them into a shared /opt/applications/raw staging area

register each AppImage as a normalized executable under /opt/applications/bin

extract icons, create .desktop files, and maintain /usr/local/bin symlinks

automatically react to new downloads through systemd .path watchers

provide simple CLI commands for initialization, enabling/disabling units, manual rescans, and cleanup

The current released version is 0.2.0.
Work for 0.3.0 (major improvements to the registrar, icon handling, and watcher logic) is underway.

🔧 Features

System-wide ingestion of user-downloaded AppImages

Automatic .desktop entry generation

Automatic icon extraction

Consistent renaming (removal of version/arch cruft)

One binary (Rust); helper scripts + systemd units installed via `appiman init` (embedded in the binary)

Systemd-based auto-registration whenever a new AppImage appears

Manual scan and clean commands for maintenance

Safe for multi-user machines — no touching user configs or non-AppImage files

Ships as an AppImage for easy self-contained installation

📦 Preferred Installation: AppImage

The recommended and simplest installation method is the prebuilt AppImage release.

Download the latest AppImage from Releases:

chmod +x appiman-*.AppImage
sudo ./appiman-*.AppImage init
sudo ./appiman-*.AppImage enable


Using the AppImage bundle ensures:

No Rust toolchain needed

No local installation clutter

Always portable and self-contained

Perfectly mirrors the environment appiman manages for other AppImages

📁 Directory Layout

Appiman manages a fixed system directory tree:

/opt/applications/
    raw/    # Staging area for newly discovered AppImages
    bin/    # Normalized AppImages ready to run
    icons/  # Extracted icons in PNG/SVG form
/usr/share/applications/   # Desktop entries created automatically
/usr/local/bin/            # Canonical symlinks for CLI access

📂 Repository Layout
assets/    # Systemd unit files + helper scripts (embedded + installed via `appiman init`)
src/       # Rust CLI implementation

Key scripts and units
File	Purpose
assets/move-appimages.sh	Recursively finds user-owned AppImages and moves them into /opt/applications/raw.
assets/register-appimages.sh	Normalizes names, installs AppImages under bin/, extracts icons, creates .desktop entries, and cleans stale symlinks/icons.
assets/*.service	Systemd services that execute the scripts.
assets/*.path	Systemd path watchers that monitor raw/ and react instantly to new files.
🚀 Commands
appiman <command>

Command	Description
init	Creates /opt/applications/*, installs helper scripts and systemd units. Requires root.
enable	Enables and starts the watcher service + path units. Requires root.
status	Shows the health of all watcher paths and services.
scan	Manually re-runs the registrar to process all AppImages.
clean	Removes stale entries, versioned duplicates, and legacy artifacts. Requires root.
help	Prints built-in help.

Typical first-time setup:

sudo appiman init
sudo appiman enable


After that, any .AppImage downloaded by any user will be ingested and registered automatically.

🏗️ Building from Source

Requires Rust 2024 edition (Rust 1.85+).

cargo build --release
install -Dm755 target/release/appiman /usr/local/bin/appiman

Note: `appiman init` installs the embedded helper scripts + systemd unit files, so copying just the `appiman` binary is sufficient (you do not need a separate `assets/` directory on disk).

If you prefer to install system-wide without the AppImage bundle, this is the supported method.

🧰 Development Workflow

Format:

cargo fmt


Lint & tests:

cargo clippy --all-targets
cargo test

Script integration tests (also run via `cargo test`):
- `register-appimages.sh`: RAW_DIR, BIN_DIR, ICON_DIR, DESKTOP_DIR, SYMLINK_DIR
- `move-appimages.sh`: RAW_DIR, HOME_ROOT

Run locally:

cargo run -- <command>

📝 License

MIT