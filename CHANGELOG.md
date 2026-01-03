# Changelog

All notable changes to the Appiman project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Comprehensive roadmap document (ROADMAP.md) outlining v0.3.0-v1.0 development plan
- Detailed Phase 1 planning document (docs/phase1-v0.3.0.md)
- ADR (Architecture Decision Record) framework and initial ADR-001
- CONTRIBUTING.md with development workflow, code style, and testing guidelines
- GitHub Actions CI/CD pipeline with multi-distro testing
- Pre-commit hooks for code quality (fmt, clippy, tests)
- Core library modules:
  - `src/core/appimage.rs` - AppImage struct with validation
  - `src/core/metadata.rs` - Metadata extraction and JSON serialization
  - `src/core/normalization.rs` - Name normalization with regex
- Configuration system with TOML support
  - `src/config.rs` - Load config from /etc/appiman/config.toml
  - Environment variable overrides for all paths
  - Structured logging configuration
- Structured logging with tracing
  - `src/logger.rs` - JSON and pretty output formats
  - Configurable log levels (trace, debug, info, warn, error)
- Mover module (`src/mover/`):
  - Scanner for finding AppImages in user home directories
  - Mover for handling file operations with collision resolution
  - Conflict resolution with automatic numbering
- Registrar module (`src/registrar/`):
  - Processor for AppImage registration pipeline
  - Icon extractor supporting PNG/SVG formats
  - Desktop entry generation following freedesktop.org spec
  - Symlink management for /usr/local/bin
- Shell script analysis document (docs/shell-script-analysis.md)
- New dependencies: thiserror, tracing, serde, toml, walkdir, chrono, lazy_static
- Comprehensive unit tests for core modules (47 tests passing)

### Changed
- Updated Cargo.toml with new dependencies
- Enhanced module structure with core/, mover/, registrar/ directories
- Refactored ingest, scan, clean, sync to use Rust modules instead of shell scripts
- Eliminated shell script execution from main CLI commands
- Updated README with configuration system documentation

### Technical Debt
- Shell scripts still available as fallback but no longer used by default

---

## [0.2.0] - (Release Date TBD)

### Added
- System-wide AppImage lifecycle management
- Automatic ingestion of user-downloaded AppImages
- Systemd-based auto-registration
- Manual scan, clean, ingest, sync commands
- Embedded shell scripts for move and register operations
- Basic testing infrastructure

---

## [0.1.0] - Initial Release
- Initial proof of concept
