# Appiman

Appiman is a compact Rust utility for system-wide AppImage lifecycle management on multi-user Linux workstations. It centralizes discovery, registration, updates, and cleanup of AppImages—without requiring AppImageLauncher, manual .desktop file creation, or scattered downloads in `$HOME`.

## Features

- **System-wide ingestion** of user-downloaded AppImages
- **Automatic .desktop entry** generation
- **Automatic icon extraction**
- **Consistent renaming** (removal of version/architecture cruft)
- **Single binary** (Rust) with embedded helper scripts and systemd units
- **Systemd-based auto-registration** whenever a new AppImage appears
- **Manual scan and clean** commands for maintenance
- **Multi-user safe** — no touching user configs or non-AppImage files
- **AppImage distribution** for easy self-contained installation

## How It Works

Appiman ships with opinionated helper scripts and systemd units that:

1. **Sweep users' home directories** for newly downloaded `.AppImage` files
2. **Ingest them** into a shared `/opt/applications/raw` staging area
3. **Register each AppImage** as a normalized executable under `/opt/applications/bin`
4. **Extract icons**, create `.desktop` files, and maintain `/usr/local/bin` symlinks
5. **Automatically react** to new downloads through systemd `.path` watchers
6. **Provide simple CLI** commands for initialization, enabling/disabling units, manual rescans, and cleanup

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

**Note:** `appiman init` installs the embedded helper scripts + systemd unit files, so copying just the `appiman` binary is sufficient (you do not need a separate `assets/` directory on disk).

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
| `init` | Creates `/opt/applications/*`, installs helper scripts and systemd units. Requires root. |
| `enable` | Enables and starts the watcher service + path units. Requires root. |
| `disable` | Disables and stops watcher path units. Requires root. |
| `status` | Shows the health of all watcher paths and services. |
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

After that, any `.AppImage` downloaded by any user will be ingested and registered automatically.

### Manual One-Shot Processing

To process AppImages without enabling the watchers:

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

Current version: 0.2.0

Work for 0.3.0 (major improvements to the registrar, icon handling, and watcher logic) is underway.

## License

MIT
