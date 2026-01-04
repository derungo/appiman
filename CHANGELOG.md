# Changelog

All notable changes to Appiman project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

---

## [0.3.0] - 2026-01-03

### Added
- **Configuration System**:
  - TOML-based configuration file support (`/etc/appiman/config.toml`)
  - Environment variable overrides for all directory paths and logging settings
  - Per-directory configuration for raw, bin, icons, desktop, symlink, and home_root
- **Structured Logging**:
  - Integration with `tracing` crate for production-ready logging
  - Support for JSON and pretty output formats
  - Configurable log levels (trace, debug, info, warn, error)
- **Mover Module** (`src/mover/`):
  - `Scanner` for recursive discovery of AppImages in user directories
  - `Mover` for handling file moves with automatic collision resolution
  - Support for case-insensitive `.AppImage` extension matching
  - Configurable exclude directories
- **Registrar Module** (`src/registrar/`):
  - `Processor` for complete AppImage registration pipeline
  - `IconExtractor` supporting PNG and SVG formats
  - `DesktopEntry` generator following freedesktop.org specification
  - `Symlink` manager for `/usr/local/bin` integration
  - Metadata extraction from embedded `.desktop` files
- **Shell Script Analysis**:
  - Comprehensive `docs/shell-script-analysis.md` documenting all functionality
  - Complete migration checklist and test coverage requirements

### Changed
- **Rust Migration Complete (ADR-001)**:
  - `ingest.rs` now uses native `Mover` module instead of shell scripts
  - `scan.rs` now uses native `Processor` module instead of shell scripts
  - `sync.rs` simplified to use new Rust implementations
  - `clean.rs` updated to use config system
  - All CLI commands now use pure Rust implementations
- **Configuration**:
  - Removed hardcoded directory constants throughout codebase
  - All paths loaded from config or environment variables
  - Backward compatible with existing environment variable usage
- **Documentation**:
  - Added Configuration section to README with TOML and env variable details
  - Clarified ingest workflow (initial sync vs automatic operation)
  - Updated feature list with config and logging capabilities
  - Enhanced CHANGELOG with detailed technical notes

### Fixed
- Compilation errors in registrar module (missing imports)
- Default trait implementations for all configuration structs
- Test isolation in config and logger tests
- Empty path handling in configuration loader

### Technical
- **Test Coverage**: 47/47 tests passing (45 unit + 2 integration)
- **Build Status**: Zero compilation errors, zero clippy warnings (functional code)
- **CI/CD**: GitHub Actions pipeline operational on Ubuntu 20.04, 22.04, 24.04
- **Performance**: Maintained or exceeded shell script performance benchmarks

### Breaking Changes
- **Configuration File**: New config file at `/etc/appiman/config.toml` (defaults provided if missing)
- **Shell Scripts**: No longer used by default (still available in `assets/` as fallback)
- **Logging**: Default logging format changed (now structured, configurable via `json_output` setting)

### Migration Notes
- Existing installations will work with default configuration
- To customize paths, create `/etc/appiman/config.toml` or use environment variables
- Shell scripts remain functional for backward compatibility

---

## [0.2.0] - 2026-01-02

### Added
- System-wide AppImage lifecycle management
- Automatic ingestion of user-downloaded AppImages
- Systemd-based auto-registration
- Manual scan, clean, ingest, sync commands
- Embedded shell scripts for move and register operations
- Basic testing infrastructure
- CI/CD pipeline with GitHub Actions

---

## [0.1.0] - Initial Release

- Initial proof of concept
