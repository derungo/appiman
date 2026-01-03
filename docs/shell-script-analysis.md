# Shell Script Analysis

## Overview
This document analyzes the current shell scripts used by Appiman to understand all functionality, dependencies, and edge cases before migration to Rust.

## Scripts Analyzed
1. `assets/move-appimages.sh` - Moves AppImages from user home dirs to staging
2. `assets/register-appimages.sh` - Processes AppImages and registers system-wide

---

## move-appimages.sh Analysis

### Location
`assets/move-appimages.sh`

### Environment Variables
- `RAW_DIR` - Target staging directory (default: `/opt/applications/raw`)
- `HOME_ROOT` - Base directory for user homes (default: `/home`)

### External Dependencies
- `bash` - Shell interpreter
- `mkdir` - Create directories
- `mv` - Move files
- `chown` - Change ownership
- `chmod` - Change permissions
- `find` - Find files recursively
- `shopt` - Shell options (nullglob)

### Functionality Breakdown

#### 1. Directory Creation
```bash
mkdir -p "$RAW_DIR"
```
**Purpose**: Create staging directory if it doesn't exist
**Edge Cases**:
- Directory already exists (idempotent)
- Permission denied on creation
- Parent directory doesn't exist

#### 2. Nullglob Enablement
```bash
shopt -s nullglob
```
**Purpose**: Treat unmatched globs as empty strings instead of literal patterns
**Critical for**: Preventing errors when no AppImages found
**Impact**: Prevents `mv` from executing with literal "*.AppImage" when no matches

#### 3. User Directory Iteration
```bash
for user_home in "$HOME_ROOT"/*; do
    [[ -d "$user_home" ]] || continue
    # ... process ...
done
```
**Purpose**: Iterate through all user home directories
**Filters**: Only processes directories (skips files)
**Edge Cases**:
- Empty home_root directory
- Symlinks in home_root
- Non-readable directories

#### 4. AppImage Finding and Moving
```bash
while IFS= read -r -d '' appimage; do
    base=$(basename "$appimage")
    stem="${base%.*}"
    ext="${base##*.}"
    dest="$RAW_DIR/$base"

    if [[ -e "$dest" ]]; then
        i=1
        while [[ -e "$RAW_DIR/${stem}-$i.$ext" ]]; do
            i=$((i + 1))
        done
        dest="$RAW_DIR/${stem}-$i.$ext"
    fi

    mv -v "$appimage" "$dest"
done < <(find "$user_home" -type f -iname '*.AppImage' -print0)
```
**Purpose**: Find and move all AppImages to staging directory
**Key Features**:
- **Recursive search**: `-type f -iname '*.AppImage'`
- **Null-terminated output**: `-print0` with `read -d ''` for safe filename handling
- **Case-insensitive matching**: `-iname '*.AppImage'`
- **Collision handling**: Renames with incrementing numbers if file exists

**Edge Cases**:
- Spaces in filenames (handled by -print0)
- Newlines in filenames (handled by -print0)
- Permission denied on read
- Same AppImage downloaded by multiple users
- Non-AppImage files with .AppImage extension

#### 5. Ownership and Permissions
```bash
chown root:root "$RAW_DIR"/*.AppImage 2>/dev/null || true
chmod 755 "$RAW_DIR"/*.AppImage 2>/dev/null || true
```
**Purpose**: Set ownership to root and make executable
**Error Handling**: Silently ignores failures (files already moved/permission issues)
**Edge Cases**:
- Empty directory (glob fails silently)
- Already correct ownership
- Permission denied on chown/chmod

### Test Cases Required
1. Multiple users with same AppImage
2. Recursive nested directories
3. Spaces in filenames
4. Unicode in filenames
5. Permission denied on read
6. No AppImages found
7. Collision with existing files
8. Symlinks in path
9. File system errors during move

---

## register-appimages.sh Analysis

### Location
`assets/register-appimages.sh`

### Environment Variables
- `RAW_DIR` - Source directory for AppImages (default: `/opt/applications/raw`)
- `BIN_DIR` - Target directory for processed AppImages (default: `/opt/applications/bin`)
- `ICON_DIR` - Target directory for icons (default: `/opt/applications/icons`)
- `DESKTOP_DIR` - Target directory for .desktop files (default: `/usr/share/applications`)
- `SYMLINK_DIR` - Target directory for symlinks (default: `/usr/local/bin`)

### External Dependencies
- `bash` - Shell interpreter
- `mkdir` - Create directories
- `cp` - Copy files
- `chmod` - Change permissions
- `ln` - Create symlinks
- `rm` - Remove files/directories
- `find` - Find files
- `grep` - Search in files
- `head` - Get first N lines
- `cut` - Extract fields
- `cat` - Concatenate files
- `tr` - Translate characters
- `sed` - Stream editor
- `mktemp` - Create temporary directory
- `cd` - Change directory
- `pushd`/`popd` - Directory stack
- `shopt` - Shell options (nullglob)

### Functionality Breakdown

#### 1. Directory Creation
```bash
mkdir -p "$RAW_DIR" "$BIN_DIR" "$ICON_DIR" "$DESKTOP_DIR" "$SYMLINK_DIR"
```
**Purpose**: Ensure all target directories exist
**Idempotent**: Yes (mkdir -p)

#### 2. Name Normalization Function
```bash
normalize_name() {
    local input="$1"
    echo "$input" | sed -E 's/(x86_64|amd64|i386|linux|setup)//Ig' \
        | sed -E 's/-?v?[0-9]+(\.[0-9]+)*//g' \
        | sed -E 's/[-_.]+/-/g' \
        | sed -E 's/^-+|-+$//g' \
        | tr A-Z a-z
}
```
**Purpose**: Normalize AppImage names by removing version/arch and standardizing separators
**Transformations** (in order):
1. Remove: x86_64, amd64, i386, linux, setup (case-insensitive)
2. Remove version patterns like -v1.2.3, -1.0.0, v2
3. Replace multiple separators (-, _, .) with single hyphen
4. Remove leading/trailing hyphens
5. Convert to lowercase

**Edge Cases**:
- Empty input
- Input with only version/arch info
- Mixed separator types
- Already normalized names

#### 3. Clean Mode
```bash
if [[ "$CLEAN" == "--clean" ]]; then
    # Remove versioned bins
    for file in "$BIN_DIR"/*.AppImage; do
        base=$(basename "$file" .AppImage)
        clean=$(normalize_name "$base")
        if [[ "$base" != "$clean" ]]; then
            echo "Removing: $file"
            rm -f "$file"
        fi
    done

    # Remove broken symlinks
    find "$SYMLINK_DIR" -type l -lname "$BIN_DIR/*" \
        -exec bash -c '[[ ! -e "$(readlink "{}")" ]] && echo "Removing symlink: {}" && rm -f "{}"' \; || true

    # Remove legacy desktop entries
    find "$DESKTOP_DIR" -type f -name "*.desktop" \
        -exec grep -Fl "$BIN_DIR/" {} \; | \
        xargs -r grep -lE '(x86_64|amd64|linux|v[0-9])' | \
        xargs -r rm -v

    # Remove versioned icons
    for icon in "$ICON_DIR"/*.png "$ICON_DIR"/*.svg; do
        [[ -f "$icon" ]] || continue
        base=$(basename "$icon")
        base_no_ext="${base%.*}"
        clean=$(normalize_name "$base_no_ext")
        if [[ "$base_no_ext" != "$clean" ]]; then
            echo "Removing: $icon"
            rm -f "$icon"
        fi
    done
fi
```
**Purpose**: Remove old/legacy AppImages, broken symlinks, versioned files
**Operations**:
1. Remove bins with version/arch in name
2. Remove broken symlinks
3. Remove .desktop files for versioned apps
4. Remove icons with version/arch in name

**Edge Cases**:
- Empty directories (glob fails, should handle gracefully)
- Already cleaned (idempotent)
- Permission denied on removal

#### 4. AppImage Processing Loop
```bash
for app in "$RAW_DIR"/*.AppImage; do
    [[ -f "$app" ]] || continue
    base=$(basename "$app" .AppImage)
    clean=$(normalize_name "$base")

    if [[ -z "$clean" ]]; then
        echo "Skipping $app (empty normalized name)"
        continue
    fi

    dest="$BIN_DIR/$clean.AppImage"

    cp --update=none "$app" "$dest" 2>/dev/null || true
    chmod +x "$dest" 2>/dev/null || true
    ln -sf "$dest" "$SYMLINK_DIR/$clean" 2>/dev/null || true
```
**Purpose**: Process each AppImage in raw directory
**Operations**:
1. Validate file exists
2. Normalize name
3. Skip if empty normalized name
4. Copy to bin directory (only if newer)
5. Make executable
6. Create symlink in /usr/local/bin

**Key Features**:
- `--update=none`: Only copy if destination doesn't exist
- Silent failures with `2>/dev/null || true`
- Idempotent operations

#### 5. AppImage Extraction
```bash
    tmp_dir=$(mktemp -d)
    if ! (cd "$tmp_dir" && "$dest" --appimage-extract > /dev/null 2>&1); then
        echo "❌ Failed to extract $app"
        rm -rf "$tmp_dir"
        continue
    fi

    app_root="$tmp_dir/squashfs-root"
    if [[ ! -d "$app_root" ]]; then
        echo "❌ Failed to extract $app (missing squashfs-root)"
        rm -rf "$tmp_dir"
        continue
    fi
```
**Purpose**: Extract AppImage contents to read metadata
**Error Handling**:
- Extraction failure → skip and cleanup
- Missing squashfs-root → skip and cleanup
- Silent extraction (output discarded)

**Edge Cases**:
- Not a valid AppImage
- Corrupted AppImage
- Permission denied on extraction
- Missing --appimage-extract support

#### 6. Desktop Entry Parsing
```bash
    pushd "$app_root" > /dev/null || {
        echo "❌ Failed to enter extracted root for $app"
        rm -rf "$tmp_dir"
        continue
    }

    app_desktop=$(find . -name "*.desktop" | head -n 1)
    name="${clean^}"
    category="Utility"
    icon_key=""

    if [[ -f "$app_desktop" ]]; then
        name=$(grep -m1 "^Name=" "$app_desktop" | cut -d= -f2- || echo "$name")
        category=$(grep -m1 "^Categories=" "$app_desktop" | cut -d= -f2- || echo "$category")
        icon_key=$(grep -m1 "^Icon=" "$app_desktop" | cut -d= -f2- || echo "")
    fi
```
**Purpose**: Parse .desktop entry for metadata
**Extracts**:
- Name (default: capitalized normalized name)
- Categories (default: Utility)
- Icon key (for icon file lookup)

**Edge Cases**:
- Multiple .desktop files (takes first)
- Missing .desktop file (uses defaults)
- Missing Name/Categories/Icon fields (uses defaults)
- Unicode in values

#### 7. Icon Extraction
```bash
    icon_src=""
    if [[ -n "$icon_key" ]]; then
        icon_src=$(find . -type f \( \
            -iname "${icon_key}.png" -o \
            -iname "${icon_key}.svg" -o \
            -iname "${icon_key}*.svg" \
        \) | head -n 1 || true)
    fi

    if [[ -z "$icon_src" ]]; then
        icon_src=$(find . -maxdepth 1 -type f \( -iname "*.png" -o -iname "*.svg" \) | head -n 1 || true)
    fi

    icon_path="$ICON_DIR/$clean.png"
    if [[ -n "$icon_src" ]]; then
        ext="${icon_src##*.}"
        ext="${ext,,}"
        icon_path="$ICON_DIR/$clean.$ext"
        cp --update=none "$icon_src" "$icon_path" 2>/dev/null || true
    fi
```
**Purpose**: Find and extract icon
**Strategy**:
1. Try to find icon matching Icon key from .desktop
2. Fallback to any PNG/SVG at root of extracted dir
3. Copy with same extension or default to PNG

**Edge Cases**:
- No icon found (icon_path remains set but empty src)
- Multiple icons (takes first)
- Non-standard icon names

#### 8. Desktop Entry Creation
```bash
    popd > /dev/null
    rm -rf "$tmp_dir"

    cat > "$DESKTOP_DIR/$clean.desktop" <<EOF
[Desktop Entry]
Type=Application
Name=$name
Exec=$dest
Icon=$icon_path
Terminal=false
Categories=$category
EOF

    chmod +x "$DESKTOP_DIR/$clean.desktop" 2>/dev/null || true
    "$dest" --appimage-update > /dev/null 2>&1 || true
```
**Purpose**: Create .desktop entry for system-wide integration
**Operations**:
1. Return to original directory
2. Cleanup temp directory
3. Write .desktop file
4. Make executable
5. Attempt to run --appimage-update

**Desktop Entry Format**:
- Type: Application
- Exec: Full path to AppImage
- Icon: Full path to icon
- Terminal: false
- Categories: From .desktop or "Utility"

**Edge Cases**:
- Icon path empty (desktop entry created anyway)
- AppImage doesn't support --appimage-update (ignored)

### Test Cases Required
1. Clean mode removes versioned files
2. Clean mode removes broken symlinks
3. AppImage without .desktop file
4. AppImage with missing Name field
5. AppImage with missing icon
6. Multiple .desktop files
7. AppImage extraction failure
8. Non-executable AppImage
9. Unicode in names
10. Special characters in paths
11. Permission denied on write
12. Read-only current directory
13. AppImage that doesn't support --appimage-update

---

## Migration Considerations

### Complexity
Both scripts use advanced bash features and have complex logic for edge cases. The migration must:

1. **Preserve behavior exactly** - No regression in functionality
2. **Improve error handling** - Better error messages, no silent failures
3. **Enhance testing** - Test coverage for all edge cases
4. **Maintain performance** - No slowdown in Rust version

### Critical Edge Cases
- **Nullglob**: Essential for handling empty directories
- **Null-terminated output**: Critical for handling special characters in filenames
- **Silent failures**: Shell script uses `2>/dev/null || true` extensively - Rust should handle errors explicitly
- **Idempotency**: Both scripts should be safe to run multiple times

### Dependency Mapping

| Shell Command | Rust Equivalent |
|---------------|-----------------|
| `find -type f -print0` | `walkdir` with `WalkDir::new()` |
| `basename` | `Path::file_name()` |
| `mkdir -p` | `fs::create_dir_all()` |
| `mv` | `fs::rename()` or `fs::copy()` + `fs::remove_file()` |
| `cp --update=none` | `fs::copy()` with existence check |
| `chmod +x` | `std::os::unix::fs::PermissionsExt` |
| `ln -sf` | `std::os::unix::fs::symlink()` |
| `grep`, `sed`, `cut` | String parsing with regex |
| `mktemp -d` | `tempfile::TempDir` |
| `find -type l -lname` | `symlink_detection()` logic |
| `shopt -s nullglob` | Handle empty iterators gracefully |

---

## Testing Strategy

### Unit Tests
- Name normalization logic (property-based tests)
- Path handling
- Collision resolution
- Icon file finding

### Integration Tests
- Full workflow: ingest → scan
- Multiple users downloading same AppImage
- Clean mode operations
- Permission handling

### Comparison Tests
- Run both shell and Rust implementations
- Compare output files
- Compare filesystem state

### Property-Based Tests
- Normalization always produces valid lowercase alphanumeric + hyphens
- Normalization of normalized name returns same result (idempotent)
- Normalization is order-independent for separators

---

## Performance Baseline

Current shell script performance (to be measured):
- Move script time for N AppImages: ___ seconds
- Register script time for N AppImages: ___ seconds
- Peak memory usage: ___ MB

Target Rust performance:
- Match or beat shell script performance
- Memory usage < 100MB
- Parallel processing for large batches

---

## Risks and Mitigation

### Risk: Subtle differences in string handling
**Mitigation**: Property-based tests, comparison tests with real AppImages

### Risk: Filesystem traversal differences
**Mitigation**: Use `walkdir` for consistent behavior across filesystems

### Risk: Permission handling differences
**Mitigation**: Test as root, test as regular user, test with permission denied scenarios

### Risk: Performance regression
**Mitigation**: Benchmark at each step, optimize hot paths, consider parallel processing

---

## Next Steps

1. [ ] Create comprehensive test suite covering all edge cases
2. [ ] Implement mover module (move-appimages.sh migration)
3. [ ] Implement registrar module (register-appimages.sh migration)
4. [ ] Run comparison tests with shell scripts
5. [ ] Performance benchmarking
6. [ ] Remove shell script dependencies

---

*Last Updated: January 2, 2026*
*Author: Lead SWE*
