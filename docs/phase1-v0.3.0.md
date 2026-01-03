# Phase 1: Foundation & Modernization (v0.3.0)

## Overview
This phase focuses on eliminating technical debt, establishing robust development practices, and creating a solid foundation for future development. This is the most critical phase as it sets the stage for all subsequent work.

**Target Release**: v0.3.0
**Timeline**: 6-8 weeks
**Primary Goals**:
- Migrate from shell scripts to pure Rust
- Establish comprehensive testing infrastructure
- Set up CI/CD pipeline
- Improve developer experience

---

## 1.1 Shell Script Migration to Pure Rust

### Current Architecture
The application currently uses shell scripts embedded in the binary:
- `assets/move-appimages.sh` - Moves AppImages from user home dirs to staging
- `assets/register-appimages.sh` - Normalizes, extracts icons, creates desktop entries

### Migration Strategy

#### Step 1: Shell Script Analysis (Week 1)
**Task**: Document all shell script functionality and edge cases
- **Deliverable**: `docs/shell-script-analysis.md`
- **Approach**:
  - Parse both shell scripts line-by-line
  - Document all command substitutions
  - Identify all external dependencies (find, grep, sed, etc.)
  - Map error handling patterns
  - Create test cases for each function

**Acceptance Criteria**:
- Complete functional documentation of both scripts
- List of all external dependencies with versions
- Test coverage matrix for shell scripts

#### Step 2: Rust Module Structure Design (Week 1-2)
**Task**: Design Rust module architecture
- **Deliverable**: `docs/architecture/v0.3.0-module-design.md`
- **New Modules**:
  ```
  src/
    ├── core/
    │   ├── mod.rs
    │   ├── appimage.rs          // AppImage parsing, extraction, validation
    │   ├── metadata.rs          // Metadata extraction and caching
    │   └── normalization.rs     // Name normalization logic
    ├── mover/
    │   ├── mod.rs
    │   ├── scanner.rs           // File system scanning
    │   ├── mover.rs             // File moving logic
    │   └── conflict.rs          // Conflict resolution
    ├── registrar/
    │   ├── mod.rs
    │   ├── processor.rs         // AppImage processing pipeline
    │   ├── icon_extractor.rs    // Icon extraction
    │   ├── desktop_entry.rs     // .desktop file generation
    │   └── symlink.rs           // Symlink management
    ├── config/
    │   ├── mod.rs
    │   � parser.rs               // Config file parsing
    │   └── defaults.rs           // Default values
    └── logging/
        ├── mod.rs
        ├── logger.rs             // Logging setup
        └── telemetry.rs          // Metrics collection
  ```

**Acceptance Criteria**:
- Module boundaries clearly defined
- Public API documentation
- Integration points identified
- Error propagation strategy documented

#### Step 3: Core Library Implementation (Week 2-3)
**Task**: Implement reusable core functionality

**`core/appimage.rs`**:
```rust
pub struct AppImage {
    pub path: PathBuf,
    pub metadata: Option<Metadata>,
}

impl AppImage {
    pub fn new(path: PathBuf) -> Result<Self, AppImageError>;
    pub fn validate(&self) -> Result<(), ValidationError>;
    pub fn extract(&self, dest: &Path) -> Result<(), ExtractError>;
    pub fn get_desktop_entry(&self) -> Result<Option<PathBuf>, ExtractError>;
    pub fn find_icon(&self) -> Result<Option<PathBuf>, ExtractError>;
    pub fn normalize_name(&self) -> String;
    pub fn get_checksum(&self) -> Result<String, IoError>;
}
```

**`core/metadata.rs`**:
```rust
pub struct Metadata {
    pub name: String,
    pub version: Option<String>,
    pub categories: Vec<String>,
    pub icon_path: Option<PathBuf>,
    pub extracted_at: DateTime<Utc>,
    pub checksum: String,
}

impl Metadata {
    pub fn from_extracted(root: &Path) -> Result<Self, MetadataError>;
    pub fn to_json(&self) -> String;
    pub fn from_json(s: &str) -> Result<Self, JsonError>;
}
```

**`core/normalization.rs`**:
```rust
pub fn normalize_appimage_name(name: &str) -> String {
    // Implementation from register-appimages.sh
    // Remove: x86_64, amd64, i386, linux, setup
    // Remove: version numbers
    // Normalize separators to hyphens
    // Convert to lowercase
}
```

**Acceptance Criteria**:
- All core functions unit tested
- Property-based tests for normalization
- Error types documented with `thiserror`
- Benchmark baselines established

#### Step 4: Mover Module Implementation (Week 3-4)
**Task**: Port `move-appimages.sh` functionality to Rust

**`mover/scanner.rs`**:
```rust
pub struct Scanner {
    pub home_root: PathBuf,
    pub exclude_dirs: Vec<PathBuf>,
}

impl Scanner {
    pub fn find_appimages(&self) -> Result<Vec<AppImage>, ScanError>;
    pub fn find_user_dirs(&self) -> Result<Vec<PathBuf>, IoError>;
}
```

**`mover/mover.rs`**:
```rust
pub struct Mover {
    pub source_dir: PathBuf,
    pub dest_dir: PathBuf,
}

impl Mover {
    pub fn move_appimages(&self, appimages: &[AppImage]) -> Result<MoveReport, MoveError>;
    pub fn handle_collision(&self, src: &Path, dest: &Path) -> Result<PathBuf, IoError>;
}

pub struct MoveReport {
    pub moved: Vec<PathBuf>,
    pub skipped: Vec<PathBuf>,
    pub errors: Vec<(PathBuf, String)>,
}
```

**Testing Strategy**:
- Integration tests with temporary directory trees
- Mock file system for edge cases
- Permission handling tests
- Collision resolution tests

**Acceptance Criteria**:
- All shell script functionality replicated
- Same output behavior (verified with comparison tests)
- Error handling improved (no silent failures)
- Performance >= shell script version

#### Step 5: Registrar Module Implementation (Week 4-5)
**Task**: Port `register-appimages.sh` functionality to Rust

**`registrar/processor.rs`**:
```rust
pub struct Processor {
    pub raw_dir: PathBuf,
    pub bin_dir: PathBuf,
    pub icon_dir: PathBuf,
    pub desktop_dir: PathBuf,
    pub symlink_dir: PathBuf,
}

impl Processor {
    pub fn process_all(&self) -> Result<ProcessReport, ProcessError>;
    pub fn process_appimage(&self, app: &AppImage) -> Result<ProcessedApp, ProcessError>;
}

pub struct ProcessReport {
    pub processed: Vec<ProcessedApp>,
    pub failed: Vec<(PathBuf, String)>,
    pub skipped: Vec<PathBuf>,
}
```

**`registrar/icon_extractor.rs`**:
```rust
pub fn extract_icon(
    app: &AppImage,
    dest_dir: &Path,
    normalized_name: &str,
) -> Result<PathBuf, IconError>;
```

**`registrar/desktop_entry.rs`**:
```rust
pub fn generate_desktop_entry(
    name: &str,
    exec_path: &Path,
    icon_path: &Path,
    categories: &[String],
) -> Result<String, DesktopError>;
```

**Testing Strategy**:
- Integration tests with fake AppImages
- Desktop entry validation against freedesktop.org spec
- Icon extraction tests (PNG, SVG)
- Symlink creation and cleanup tests

**Acceptance Criteria**:
- All shell script functionality replicated
- Same output behavior (verified with comparison tests)
- Better error messages with context
- Performance >= shell script version

#### Step 6: Integration Testing (Week 5-6)
**Task**: Comprehensive end-to-end testing

**Test Scenarios**:
1. Fresh install scenario
2. Multiple users downloading same AppImage
3. Batch ingestion of 100+ AppImages
4. Cleanup of stale entries
5. Permission denied scenarios
6. Corrupted AppImage handling
7. Unicode filename handling

**Acceptance Criteria**:
- 100% test coverage for critical paths
- All shell script tests pass with Rust implementation
- No regression in functionality

#### Step 7: Migration & Cleanup (Week 6)
**Task**: Remove shell script dependencies

- Remove `assets/*.sh` files
- Remove `setup.rs` script embedding
- Update documentation
- Deprecation notice for v0.2.0 users

**Acceptance Criteria**:
- Zero shell script dependencies
- Single binary distribution
- Migration guide provided

---

## 1.2 Structured Logging Implementation

### Requirements
- Replace `println!/eprintln!` with structured logging
- Support multiple log levels
- JSON output for production
- Configurable log destinations

### Implementation

**`logging/logger.rs`**:
```rust
use tracing_subscriber::{fmt, EnvFilter};

pub fn init_logger() -> Result<(), LoggerError> {
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info"));

    let fmt_layer = fmt::layer()
        .with_target(true)
        .with_thread_ids(true)
        .with_file(true)
        .with_line_number(true);

    let subscriber = tracing_subscriber::registry()
        .with(env_filter)
        .with(fmt_layer);

    tracing::subscriber::set_global_default(subscriber)?;

    Ok(())
}
```

**Logging throughout codebase**:
```rust
use tracing::{info, warn, error, debug, instrument};

#[instrument(skip(self))]
pub fn run_scan(&self) -> Result<(), ScanError> {
    info!("Starting AppImage scan");
    debug!("Scanning directory: {:?}", self.raw_dir);

    for app in &self.appimages {
        match self.process_appimage(app) {
            Ok(result) => {
                info!(app = %app.path.display(), "Processed successfully");
            }
            Err(e) => {
                error!(
                    app = %app.path.display(),
                    error = %e,
                    "Failed to process"
                );
            }
        }
    }

    Ok(())
}
```

**Acceptance Criteria**:
- All debug/info/warn/error logging implemented
- JSON output works with `RUST_LOG=json`
- Log levels configurable via `RUST_LOG`

---

## 1.3 Enhanced Error Handling

### Requirements
- Use `thiserror` for error types
- Helpful error messages with context
- Error chains for debugging
- Structured error output

### Implementation

**Error Types per Module**:

```rust
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppImageError {
    #[error("Invalid AppImage format: {0}")]
    InvalidFormat(String),

    #[error("Failed to extract AppImage: {0}")]
    ExtractFailed(#[from] ExtractError),

    #[error("Metadata not found")]
    MetadataNotFound,

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Debug, Error)]
pub enum ProcessError {
    #[error("Failed to process AppImage {path}: {reason}")]
    ProcessingFailed { path: PathBuf, reason: String },

    #[error("Icon extraction failed for {app}: {source}")]
    IconExtractionFailed { app: String, source: Box<dyn Error + Send + Sync> },

    #[error("Desktop entry generation failed: {0}")]
    DesktopEntryFailed(#[from] DesktopError),
}
```

**Acceptance Criteria**:
- All error types use `thiserror`
- Error messages include context
- Error chains work for debugging
- User-friendly error messages

---

## 1.4 Configuration System

### Requirements
- TOML configuration file support
- Environment variable overrides
- Command-line flag overrides
- Validation on load

### Implementation

**`config/config.toml`**:
```toml
[directories]
raw = "/opt/applications/raw"
bin = "/opt/applications/bin"
icons = "/opt/applications/icons"
desktop = "/usr/share/applications"
symlink = "/usr/local/bin"
home_root = "/home"

[mover]
scan_interval = "1m"
exclude_dirs = ["/home/.cache", "/home/.local/share"]

[registrar]
enable_update_check = true
update_check_interval = "24h"
max_versions_per_app = 3

[logging]
level = "info"
json_output = false
file = "/var/log/appiman.log"
```

**`config/parser.rs`**:
```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
    pub directories: Directories,
    pub mover: MoverConfig,
    pub registrar: RegistrarConfig,
    pub logging: LoggingConfig,
}

impl Config {
    pub fn load() -> Result<Self, ConfigError> {
        let path = std::env::var("APPIMAN_CONFIG")
            .unwrap_or_else(|_| "/etc/appiman/config.toml".to_string());

        let content = std::fs::read_to_string(&path)?;
        let mut config: Self = toml::from_str(&content)?;

        // Apply environment variable overrides
        config.apply_env_overrides();

        Ok(config)
    }

    fn apply_env_overrides(&mut self) {
        if let Ok(raw_dir) = std::env::var("APPIMAN_RAW_DIR") {
            self.directories.raw = raw_dir;
        }
        // ... other overrides
    }
}
```

**Acceptance Criteria**:
- Config file works
- Environment variable overrides work
- Validation catches invalid configs
- Defaults are sensible

---

## 1.5 CI/CD Pipeline

### GitHub Actions Workflow

**`.github/workflows/ci.yml`**:
```yaml
name: CI

on:
  push:
    branches: [master, develop]
  pull_request:
    branches: [master, develop]

jobs:
  test:
    runs-on: ${{ matrix.os }}
    strategy:
      matrix:
        os: [ubuntu-20.04, ubuntu-22.04, ubuntu-24.04, fedora-38]
        rust: [stable, nightly]

    steps:
      - uses: actions/checkout@v3

      - uses: actions-rs/toolchain@v1
        with:
          toolchain: ${{ matrix.rust }}
          override: true
          components: clippy, rustfmt

      - name: Install dependencies
        run: |
          sudo apt-get update
          sudo apt-get install -y bash findutils grep sed coreutils

      - name: Cache cargo
        uses: actions/cache@v3
        with:
          path: ~/.cargo/registry
          key: ${{ runner.os }}-cargo-${{ hashFiles('**/Cargo.lock') }}

      - name: Format check
        run: cargo fmt --all -- --check

      - name: Clippy
        run: cargo clippy --all-targets --all-features -- -D warnings

      - name: Run tests
        run: cargo test --all-features

      - name: Run integration tests
        run: cargo test --test '*'

  audit:
    runs-on: ubuntu-latest

    steps:
      - uses: actions/checkout@v3

      - name: Run cargo-audit
        uses: actions-rs/audit@v1
        with:
          token: ${{ secrets.GITHUB_TOKEN }}

  release:
    needs: [test, audit]
    if: startsWith(github.ref, 'refs/tags/v')
    runs-on: ubuntu-latest

    steps:
      - uses: actions/checkout@v3

      - name: Build release
        run: cargo build --release

      - name: Create AppImage
        run: ./scripts/build-appimage.sh

      - name: Release
        uses: softprops/action-gh-release@v1
        with:
          files: target/appiman-*.AppImage
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
```

**Acceptance Criteria**:
- CI runs on all PRs
- Multiple distros tested
- Audit runs on each commit
- Release automation works

---

## 1.6 Pre-commit Hooks

### Setup Script

**`.pre-commit-config.yaml`**:
```yaml
repos:
  - repo: local
    hooks:
      - id: rust-fmt
        name: rust fmt
        entry: cargo fmt --all -- --
        language: system
        files: \.rs$
        pass_filenames: false

      - id: rust-clippy
        name: rust clippy
        entry: cargo clippy --all-targets --all-features -- -D warnings
        language: system
        files: \.rs$
        pass_filenames: false

      - id: cargo-test
        name: cargo test
        entry: cargo test --all-features
        language: system
        files: \.rs$
        pass_filenames: false
```

**Acceptance Criteria**:
- Pre-commit hooks install with `pre-commit install`
- Hooks run automatically
- Failed hooks block commit

---

## 1.7 Documentation

### Required Documents

**`CONTRIBUTING.md`**:
- Development environment setup
- Code style guidelines
- Testing strategy
- Pull request process
- Release process

**`docs/architecture/v0.3.0.md`**:
- Module architecture
- Data flow diagrams
- Error handling strategy
- Configuration system

**`docs/migration/v0.2.0-to-v0.3.0.md`**:
- Breaking changes
- Migration steps
- Testing checklist

**Acceptance Criteria**:
- All documents created
- Links in README.md updated
- Docs pass spell check

---

## Deliverables Checklist

### Core Functionality
- [ ] Shell script analysis document
- [ ] Core library modules (appimage, metadata, normalization)
- [ ] Mover module with full functionality
- [ ] Registrar module with full functionality
- [ ] Configuration system
- [ ] Structured logging
- [ ] Enhanced error handling

### Testing
- [ ] Unit tests for all modules (80%+ coverage)
- [ ] Integration tests for full workflows
- [ ] Property-based tests for normalization
- [ ] Comparison tests with shell scripts
- [ ] Performance benchmarks

### Infrastructure
- [ ] CI/CD pipeline (GitHub Actions)
- [ ] Pre-commit hooks
- [ ] Security scanning (cargo-audit)
- [ ] Dependabot configuration

### Documentation
- [ ] CONTRIBUTING.md
- [ ] Architecture documentation
- [ ] Migration guide
- [ ] Updated README.md
- [ ] ADR template

### Release
- [ ] Changelog.md
- [ ] Release notes
- [ ] GitHub release with artifacts
- [ ] Documentation website (if applicable)

---

## Success Metrics

**Quality Metrics**:
- 80%+ test coverage
- Zero clippy warnings
- All tests passing in CI
- Security audit passing

**Performance Metrics**:
- Rust version >= shell script version
- Full scan time < 5s for 100 AppImages
- Memory usage < 100MB

**Developer Metrics**:
- Time to run tests < 30s
- Build time < 2 minutes
- Documentation completeness > 90%

---

## Timeline

| Week | Tasks |
|------|-------|
| 1 | Shell script analysis, module design, core library design |
| 2 | Core library implementation (appimage, metadata, normalization) |
| 3 | Mover module implementation |
| 4 | Registrar module implementation |
| 5 | Integration testing, CI/CD setup |
| 6 | Documentation, migration, release preparation |

---

*Last Updated: January 2, 2026*
*Owner: Lead SWE*
