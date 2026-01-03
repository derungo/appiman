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
- New dependencies: thiserror, tracing, serde, toml, walkdir, chrono, lazy_static
- Comprehensive unit tests for core modules (29 tests passing)

### Changed
- Updated Cargo.toml with new dependencies
- Enhanced module structure with core/ directory

### Technical Debt
- Shell scripts still in use (migration in progress per ADR-001)
- Logging still uses println!/eprintln! (tracing integration planned)

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
