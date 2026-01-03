# Contributing to Appiman

Thank you for your interest in contributing to Appiman! This guide will help you get started with the development process.

## Table of Contents
- [Getting Started](#getting-started)
- [Development Workflow](#development-workflow)
- [Code Style Guidelines](#code-style-guidelines)
- [Testing Guidelines](#testing-guidelines)
- [Pull Request Process](#pull-request-process)
- [Release Process](#release-process)

---

## Getting Started

### Prerequisites
- Rust 2024 edition (Rust 1.85+)
- Linux system (for testing AppImage functionality)
- Git
- bash, find, grep, sed, coreutils (for testing shell scripts during v0.3.0 migration)

### Installation

```bash
# Clone the repository
git clone https://github.com/derungo/appiman.git
cd appiman

# Install development tools
cargo install cargo-watch cargo-audit cargo-edit

# Install pre-commit hooks (optional but recommended)
pip install pre-commit
pre-commit install
```

### Building

```bash
# Debug build
cargo build

# Release build
cargo build --release

# Run directly
cargo run -- <command>
```

### Running Tests

```bash
# Run all tests
cargo test

# Run tests with output
cargo test -- --nocapture

# Run specific test
cargo test test_name

# Run integration tests
cargo test --test '*'

# Run with coverage (requires tarpaulin)
cargo tarpaulin --out Html
```

---

## Development Workflow

### 1. Create a Branch

```bash
# Create a new feature branch
git checkout -b feature/your-feature-name

# Or a bugfix branch
git checkout -b fix/issue-description
```

### 2. Make Changes

- Follow the [Code Style Guidelines](#code-style-guidelines)
- Add tests for new functionality
- Update documentation as needed
- Run `cargo fmt` to format your code
- Run `cargo clippy` to check for warnings

### 3. Test Your Changes

```bash
# Run tests
cargo test

# Run clippy
cargo clippy --all-targets --all-features

# Run pre-commit hooks
pre-commit run --all-files
```

### 4. Commit Your Changes

```bash
# Stage your changes
git add .

# Commit with a descriptive message
git commit -m "Add new feature: description"

# Or use conventional commits
git commit -m "feat: add automatic update checking"
git commit -m "fix: resolve race condition in mover"
git commit -m "docs: update README with new commands"
```

### 5. Push and Create Pull Request

```bash
# Push to origin
git push origin feature/your-feature-name

# Create a pull request on GitHub
```

---

## Code Style Guidelines

### Rust Code Style

- Use `rustfmt` for formatting
- Run `cargo fmt` before committing
- Address all `cargo clippy` warnings
- Use `thiserror` for error types
- Use `tracing` for logging

### Naming Conventions

- **Modules**: snake_case (`appimage_processor`)
- **Types**: PascalCase (`AppImage`, `Metadata`)
- **Functions**: snake_case (`process_appimage`, `find_icons`)
- **Constants**: SCREAMING_SNAKE_CASE (`DEFAULT_CONFIG_PATH`)
- **Acronyms**: Treat as words (`Http` not `HTTP`, `AppImage` not `AppIMAGE`)

### Documentation

- Document all public items with `///` doc comments
- Use `//!` for module-level documentation
- Include examples for complex functions
- Run `cargo doc --no-deps --open` to generate and view docs

```rust
/// Processes an AppImage file and registers it system-wide.
///
/// # Arguments
///
/// * `app` - The AppImage to process
/// * `config` - Configuration for processing options
///
/// # Errors
///
/// Returns an error if:
/// - The AppImage file is invalid
/// - Metadata extraction fails
/// - Icon extraction fails
///
/// # Examples
///
/// ```no_run
/// use appiman::registrar::Processor;
/// let processor = Processor::new(config);
/// processor.process_appimage(&app)?;
/// ```
pub fn process_appimage(&self, app: &AppImage) -> Result<ProcessedApp, ProcessError> {
    // ...
}
```

### Error Handling

- Use `thiserror` for custom error types
- Provide helpful error messages with context
- Use `?` operator for error propagation
- Wrap external errors in context where appropriate

```rust
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ProcessError {
    #[error("Failed to process AppImage {path}: {reason}")]
    ProcessingFailed { path: PathBuf, reason: String },

    #[error("IO error while accessing {path}: {source}")]
    IoError {
        path: PathBuf,
        source: std::io::Error,
    },
}
```

### Logging

- Use `tracing` for structured logging
- Log at appropriate levels:
  - `error!`: Critical errors requiring attention
  - `warn!`: Non-critical issues
  - `info!`: Normal operations
  - `debug!`: Detailed diagnostics
  - `trace!`: Very detailed diagnostics

```rust
use tracing::{info, debug, instrument};

#[instrument(skip(self))]
pub fn process_all(&self) -> Result<ProcessReport, ProcessError> {
    info!("Starting batch processing of AppImages");
    debug!("Scanning directory: {:?}", self.raw_dir);

    // Processing logic...

    info!("Completed processing: {count} AppImages", count = report.processed.len());
    Ok(report)
}
```

---

## Testing Guidelines

### Unit Tests

- Test public APIs comprehensively
- Test edge cases and error conditions
- Use descriptive test names
- Keep tests fast (< 1s each)

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_name_removes_version_suffixes() {
        assert_eq!(
            normalize_name("TestApp-v1.2.3-x86_64.AppImage"),
            "testapp"
        );
    }

    #[test]
    fn normalize_name_handles_empty_input() {
        assert_eq!(normalize_name(""), "");
    }

    #[test]
    fn normalize_name_preserves_valid_names() {
        assert_eq!(
            normalize_name("MyApplication.AppImage"),
            "myapplication"
        );
    }
}
```

### Integration Tests

- Place in `tests/` directory
- Test full workflows
- Use temporary directories for isolation
- Clean up after tests

```rust
#[cfg(unix)]
mod tests {
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn full_ingest_and_scan_workflow() {
        let temp_dir = TempDir::new().unwrap();
        let config = test_config(&temp_dir);

        // Set up test AppImages
        create_test_appimage(&temp_dir, "Test.AppImage");

        // Run ingest
        let mover = Mover::new(&config);
        let report = mover.move_appimages().unwrap();
        assert_eq!(report.moved.len(), 1);

        // Run scan
        let registrar = Registrar::new(&config);
        let report = registrar.process_all().unwrap();
        assert_eq!(report.processed.len(), 1);

        // Verify results
        assert!(temp_dir.path().join("bin/testapp.AppImage").exists());
        assert!(temp_dir.path().join("icons/testapp.png").exists());
    }
}
```

### Property-Based Testing

- Use `proptest` for property-based tests
- Test invariants and properties
- Good for normalization and transformation logic

```rust
#[cfg(test)]
mod proptests {
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn normalize_name_output_is_alphanumeric_hyphens(name in "[A-Za-z0-9._-]+") {
            let normalized = normalize_name(&name);
            assert!(normalized.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-'));
        }
    }
}
```

---

## Pull Request Process

### Before Submitting

1. **Run all checks**
   ```bash
   cargo fmt
   cargo clippy --all-targets --all-features
   cargo test
   ```

2. **Update documentation**
   - Add/change doc comments
   - Update README if needed
   - Update CHANGELOG.md

3. **Write a good PR description**
   - Describe the what and why
   - Link to related issues
   - Include screenshots if UI changes
   - Add testing instructions

### PR Description Template

```markdown
## Description
Brief description of the changes made.

## Type of Change
- [ ] Bug fix
- [ ] New feature
- [ ] Breaking change
- [ ] Documentation update

## Related Issue
Fixes #123

## Changes Made
- List of major changes

## Testing
- How this was tested
- Test cases added

## Checklist
- [ ] Code follows style guidelines
- [ ] Self-review completed
- [ ] Comments added for complex logic
- [ ] Documentation updated
- [ ] No new warnings generated
- [ ] Tests added/updated
- [ ] All tests passing
```

### Review Process

1. **Automated checks** must pass
2. **Code review** by at least one maintainer
3. **Address feedback** from reviewers
4. **Squash and merge** when approved

---

## Release Process

### Version Numbers

Appiman follows [Semantic Versioning](https://semver.org/):
- **MAJOR**: Incompatible API changes
- **MINOR**: Backwards-compatible functionality additions
- **PATCH**: Backwards-compatible bug fixes

### Release Checklist

1. **Update version** in `Cargo.toml`
2. **Update CHANGELOG.md**
3. **Tag the release**
   ```bash
   git tag -a v0.3.0 -m "Release v0.3.0"
   git push origin v0.3.0
   ```
4. **Build release artifacts**
   ```bash
   cargo build --release
   ./scripts/build-appimage.sh
   ```
5. **Create GitHub release**
   - Upload release artifacts
   - Copy CHANGELOG entry
   - Verify checksums

### Post-Release

1. Update documentation website
2. Announce on GitHub Discussions
3. Update ROADMAP.md
4. Close completed issues

---

## Communication

- **GitHub Issues**: For bug reports and feature requests
- **GitHub Discussions**: For questions and ideas
- **Pull Requests**: For code contributions

---

## Getting Help

If you need help:
1. Check the [documentation](https://github.com/derungo/appiman/docs)
2. Search [existing issues](https://github.com/derungo/appiman/issues)
3. Start a [GitHub Discussion](https://github.com/derungo/appiman/discussions)
4. Contact maintainers via `@mentions` in issues/PRs

---

## Code of Conduct

Be respectful, inclusive, and constructive. We reserve the right to remove any content or users that violate this principle.

---

*Last Updated: January 2, 2026*
