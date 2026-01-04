# Appiman

Appiman is a compact Rust utility for system-wide AppImage lifecycle management on multi-user Linux workstations. It centralizes discovery, registration, updates, and cleanup of AppImages—without requiring AppImageLauncher, manual .desktop file creation, or scattered downloads in `$HOME`.

## Features

- **System-wide ingestion** of user-downloaded AppImages
- **Automatic .desktop entry** generation
- **Automatic icon extraction**
- **Consistent renaming** (removal of version/architecture cruft)
- **Single binary** (Rust) with embedded systemd units
- **Systemd-based auto-registration** whenever a new AppImage appears
- **Manual scan and clean** commands for maintenance
- **Multi-user safe** — no touching user configs or non-AppImage files
- **AppImage distribution** for easy self-contained installation
- **Configuration system** with TOML config file support and environment variable overrides
- **Structured logging** with configurable levels and JSON/pretty output formats

## How It Works

Appiman ships with systemd units and configurable settings that:

1. **Load configuration** from `/etc/appiman/config.toml` or environment variables
2. **Sweep users' home directories** for newly downloaded `.AppImage` files
3. **Ingest them** into a shared `/opt/applications/raw` staging area
4. **Register each AppImage** as a normalized executable under `/opt/applications/bin`
5. **Extract icons**, create `.desktop` files, and maintain `/usr/local/bin` symlinks
6. **Automatically react** to new downloads through systemd `.path` watchers
7. **Provide simple CLI** commands for initialization, enabling/disabling units, manual rescans, and cleanup

## Installation

### Preferred: AppImage

The recommended and simplest installation method is the prebuilt AppImage release.

```bash
# Download the latest AppImage from Releases
chmod +x appiman-*.AppImage
sudo ./appiman-*.AppImage init
sudo ./appiman-*.AppImage enable
```

**Using the AppImage bundle ensures:**
- No Rust toolchain needed
- No local installation clutter
- Always portable and self-contained
- Perfectly mirrors the environment appiman manages for other AppImages

### Building from Source

Requires Rust 2024 edition (Rust 1.85+).

```bash
cargo build --release
install -Dm755 target/release/appiman /usr/local/bin/appiman
```

**Note:** `appiman init` installs systemd unit files that call the appiman binary directly, so copying just the `appiman` binary is sufficient (you do not need a separate `assets/` directory on disk).

## Configuration

Appiman can be configured via:

### Configuration File

Create `/etc/appiman/config.toml` with the following structure:

```toml
[directories]
raw = "/opt/applications/raw"
bin = "/opt/applications/bin"
icons = "/opt/applications/icons"
desktop = "/usr/share/applications"
symlink = "/usr/local/bin"
home_root = "/home"

[logging]
level = "info"
json_output = false
```

### Environment Variables

All configuration values can be overridden with environment variables:

- `APPIMAN_CONFIG` - Path to config file (default: `/etc/appiman/config.toml`)
- `APPIMAN_RAW_DIR` - Staging directory for AppImages
- `APPIMAN_BIN_DIR` - Processed AppImages directory
- `APPIMAN_ICON_DIR` - Icon storage directory
- `APPIMAN_DESKTOP_DIR` - Desktop entries directory
- `APPIMAN_SYMLINK_DIR` - Symlink directory
- `APPIMAN_HOME_ROOT` - User home directories root
- `RUST_LOG` - Logging level (trace, debug, info, warn, error)

## Directory Layout

Appiman manages a fixed system directory tree:

```
/opt/applications/
    raw/    # Staging area for newly discovered AppImages
    bin/    # Normalized AppImages ready to run
    icons/  # Extracted icons in PNG/SVG form
/usr/share/applications/   # Desktop entries created automatically
/usr/local/bin/            # Canonical symlinks for CLI access
```

## Usage

### Commands

| Command | Description |
|---------|-------------|
| `init` | Creates `/opt/applications/*`, installs systemd units. Requires root. |
| `enable` | Enables and starts the watcher service + path units. Requires root. |
| `disable` | Disables and stops watcher path units. Requires root. |
| `status` | Shows the health of watcher paths, services, and registered AppImages. Supports `--json` flag. |
| `ingest` | Moves user-downloaded AppImages into `/opt/applications/raw`. Requires root. |
| `scan` | Manually re-runs the registrar to process all AppImages. Requires root. |
| `sync` | Runs ingest + scan (full manual ingestion + registration). Requires root. |
| `clean` | Removes stale entries, versioned duplicates, and legacy artifacts. Requires root. |
| `help` | Prints built-in help. |

### Typical First-Time Setup

```bash
sudo appiman init
sudo appiman enable
```

After this, any `.AppImage` downloaded by any user will be automatically ingested and registered. You should immediately find them in your application launcher (menu, search, etc.).

### Initial Ingestion (if you have existing AppImages)

If you have AppImages already downloaded before installing appiman, you'll need to trigger a manual ingestion once:

```bash
sudo appiman sync    # One-time ingestion of existing AppImages
```

After this first manual sync, the systemd watchers will handle all future downloads automatically.

### Manual Processing

To process AppImages without enabling the watchers (e.g., for testing or if watchers fail):

```bash
sudo appiman sync
```

## Repository Layout

```
assets/    # Systemd unit files + helper scripts (embedded + installed via `appiman init`)
src/       # Rust CLI implementation
docs/       # Documentation and architecture decisions
tests/      # Integration tests for shell scripts
```

### Key Scripts and Units

| File | Purpose |
|------|---------|
| `assets/move-appimages.sh` | Recursively finds user-owned AppImages and moves them into `/opt/applications/raw` |
| `assets/register-appimages.sh` | Normalizes names, installs AppImages under `bin/`, extracts icons, creates `.desktop` entries, and cleans stale symlinks/icons |
| `assets/*.service` | Systemd services that execute the scripts |
| `assets/*.path` | Systemd path watchers that monitor `raw/` and react instantly to new files |

## Development

See [CONTRIBUTING.md](CONTRIBUTING.md) for detailed development guidelines.

### Format

```bash
cargo fmt
```

### Lint & Tests

```bash
cargo clippy --all-targets
cargo test
```

### Script Integration Tests

Also run via `cargo test`:
- `register-appimages.sh`: `RAW_DIR`, `BIN_DIR`, `ICON_DIR`, `DESKTOP_DIR`, `SYMLINK_DIR`
- `move-appimages.sh`: `RAW_DIR`, `HOME_ROOT`

### Run Locally

```bash
cargo run -- <command>
```

## Roadmap

See [ROADMAP.md](ROADMAP.md) for the full development plan from v0.3.0 to v1.0 and beyond.

Current version: 0.3.1

Phase 1 Foundation & Modernization (v0.3.0) completed. Currently working on Phase 2: Feature Expansion (v0.4.0).

## License

MIT
