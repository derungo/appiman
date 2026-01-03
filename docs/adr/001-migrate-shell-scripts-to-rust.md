# ADR-001: Migrate Shell Scripts to Pure Rust

## Metadata
- **Status**: Accepted
- **Date**: 2026-01-02
- **Author**: Lead SWE
- **Related Issue**: (to be created)
- **Superseded By**: N/A

## Context

Appiman currently relies on shell scripts for core functionality:
- `move-appimages.sh`: Scans user home directories and moves AppImages to staging
- `register-appimages.sh`: Processes AppImages, extracts icons, creates desktop entries

These scripts are embedded in the Rust binary and executed via `Command::new()`. While this approach works, it has several limitations:

1. **Error Handling**: Shell scripts have limited error handling, often failing silently or with cryptic messages
2. **Cross-Platform**: Shell scripts are Linux-specific, limiting portability to other Unix systems
3. **Testing**: Testing shell scripts is difficult compared to Rust unit tests
4. **Maintenance**: Maintaining shell script logic alongside Rust code increases cognitive load
5. **Performance**: Shell script execution has overhead and lacks optimization
6. **Dependencies**: External dependencies (bash, find, grep, sed) add complexity

The current v0.2.0 release has these scripts as the core moving parts of the system, making them the single point of failure for most operations.

## Decision

We will migrate all shell script functionality to pure Rust, eliminating external shell script dependencies entirely.

### Scope

**In Scope**:
1. `move-appimages.sh` → `src/mover/` module
2. `register-appimages.sh` → `src/registrar/` module
3. Removal of script embedding in `setup.rs`
4. Removal of `assets/*.sh` files

**Out of Scope**:
- Systemd unit files (these will remain as config files)
- Helper scripts for packaging/CI (these are development tools)

### Architecture

The new architecture will introduce several new modules:

```
src/
  ├── core/
  │   ├── mod.rs
  │   ├── appimage.rs          # AppImage parsing, validation
  │   ├── metadata.rs          # Metadata extraction, caching
  │   └── normalization.rs     # Name normalization logic
  ├── mover/
  │   ├── mod.rs
  │   ├── scanner.rs           # File system scanning
  │   ├── mover.rs             # File moving logic
  │   └── conflict.rs          # Conflict resolution
  └── registrar/
      ├── mod.rs
      ├── processor.rs         # AppImage processing pipeline
      ├── icon_extractor.rs    # Icon extraction
      ├── desktop_entry.rs     # .desktop file generation
      └── symlink.rs           # Symlink management
```

### Implementation Approach

1. **Incremental Migration**: Migrate functionality incrementally with parallel testing
2. **Feature Flags**: Use feature flags to switch between shell and Rust implementations
3. **Comparison Testing**: Run both implementations in parallel to verify identical behavior
4. **Performance Benchmarking**: Ensure Rust version matches or exceeds shell performance

## Consequences

### Positive

1. **Better Error Handling**: Rust's `Result` types provide structured, composable error handling
2. **Testability**: Comprehensive unit and integration tests can be written in Rust
3. **Performance**: No shell overhead, potential for parallel processing
4. **Maintainability**: Single language codebase reduces cognitive load
5. **Cross-Platform**: Easier to support other Unix systems (BSD, macOS)
6. **Type Safety**: Compile-time guarantees catch more bugs
7. **Single Binary**: Distribution becomes simpler with a single binary
8. **Zero External Dependencies**: Eliminates bash, find, grep, sed dependencies

### Negative

1. **Development Effort**: Significant upfront effort (~6 weeks)
2. **Regression Risk**: Complex logic migration could introduce bugs
3. **Testing Burden**: Extensive testing required to ensure parity
4. **Learning Curve**: Team needs to understand new code structure

### Risks

1. **Logic Differences**: Subtle differences in shell vs. Rust string handling
   - **Mitigation**: Comprehensive comparison tests, property-based testing

2. **Filesystem Differences**: Different filesystem traversal behavior
   - **Mitigation**: Use `walkdir` crate for consistent behavior, test on multiple filesystems

3. **Performance Regression**: Rust implementation slower than shell scripts
   - **Mitigation**: Benchmark at each step, optimize hot paths, consider parallel processing

4. **Feature Regression**: Missing functionality from shell scripts
   - **Mitigation**: Detailed functional mapping, feature parity checklist

## Alternatives Considered

### Alternative 1: Keep Shell Scripts
- **Pros**: Works now, no migration effort
- **Cons**: All the limitations described in Context
- **Decision**: Rejected - Technical debt is too high

### Alternative 2: Rewrite in Different Language (Python/Go)
- **Pros**: Better error handling than shell
- **Cons**: Adding another language, distribution complexity
- **Decision**: Rejected - Rust is the primary language

### Alternative 3: Hybrid Approach (Keep Some Scripts)
- **Pros**: Incremental migration, lower risk
- **Cons**: Complexity of two execution paths, maintenance burden
- **Decision**: Rejected - Full migration provides better long-term benefits

### Alternative 4: Use Existing AppImage Libraries
- **Pros**: Leverage existing code
- **Cons**: May not fit use case, dependency risk
- **Decision**: Considered - Will use for AppImage extraction if suitable, but implement core logic ourselves

## Implementation

### Status: In Progress
### Owner: Lead SWE
### Timeline: 6 weeks

### Migration Steps

#### Week 1: Analysis & Design
- [ ] Document all shell script functionality
- [ ] Identify external dependencies
- [ ] Design Rust module structure
- [ ] Create ADR-001

#### Week 2: Core Library
- [ ] Implement `core/appimage.rs` (AppImage struct, validation)
- [ ] Implement `core/metadata.rs` (metadata extraction, caching)
- [ ] Implement `core/normalization.rs` (name normalization)
- [ ] Unit tests for all core modules
- [ ] Property-based tests for normalization

#### Week 3: Mover Module
- [ ] Implement `mover/scanner.rs` (file system scanning)
- [ ] Implement `mover/mover.rs` (file moving logic)
- [ ] Implement `mover/conflict.rs` (conflict resolution)
- [ ] Integration tests for mover
- [ ] Comparison tests with shell script

#### Week 4: Registrar Module
- [ ] Implement `registrar/processor.rs` (processing pipeline)
- [ ] Implement `registrar/icon_extractor.rs` (icon extraction)
- [ ] Implement `registrar/desktop_entry.rs` (desktop entry generation)
- [ ] Implement `registrar/symlink.rs` (symlink management)
- [ ] Integration tests for registrar
- [ ] Comparison tests with shell script

#### Week 5: Integration & Testing
- [ ] End-to-end integration tests
- [ ] Performance benchmarking
- [ ] Edge case testing
- [ ] Fix bugs and regressions

#### Week 6: Cleanup & Release
- [ ] Remove shell script files
- [ ] Update documentation
- [ ] Migration guide for users
- [ ] Release v0.3.0

### Feature Parity Checklist

**move-appimages.sh functionality**:
- [ ] Scan user home directories recursively
- [ ] Find all *.AppImage files
- [ ] Move to raw staging directory
- [ ] Handle name collisions with numbering
- [ ] Change ownership to root:root
- [ ] Set permissions to 755
- [ ] Skip non-AppImage files

**register-appimages.sh functionality**:
- [ ] Normalize AppImage names (remove version, arch)
- [ ] Copy to bin directory
- [ ] Set executable permissions
- [ ] Create symlinks in /usr/local/bin
- [ ] Extract AppImage contents
- [ ] Find and parse .desktop entry
- [ ] Extract application name, categories
- [ ] Find and extract icon (PNG/SVG)
- [ ] Create .desktop entry in /usr/share/applications
- [ ] Clean up old versions (with --clean flag)
- [ ] Run --appimage-update

### Testing Strategy

1. **Unit Tests**: Test each function independently
2. **Integration Tests**: Test full workflows with temp directories
3. **Comparison Tests**: Run both shell and Rust versions, compare results
4. **Property-Based Tests**: Test invariants (normalization always produces valid names)
5. **Performance Tests**: Benchmark against shell scripts
6. **Fuzz Testing**: Fuzz inputs to find edge cases

### Rollback Plan

If critical issues are found in production:
1. Re-tag v0.2.0 as latest stable
2. Keep shell script branch as fallback
3. Fix issues in v0.3.1 branch
4. Re-release once fixed

## References

- [Shell Script Analysis](./shell-script-analysis.md) (to be created)
- [Module Design Document](./architecture/v0.3.0-module-design.md) (to be created)
- [AppImage Specification](https://github.com/AppImage/AppImageSpec)
- [freedesktop.org Desktop Entry Spec](https://specifications.freedesktop.org/desktop-entry-spec/)
- [Phase 1 Roadmap](../phase1-v0.3.0.md)

---

*Last Updated: January 2, 2026*
