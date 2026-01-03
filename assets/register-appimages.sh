#!/usr/bin/env bash

RAW_DIR="${RAW_DIR:-/opt/applications/raw}"
BIN_DIR="${BIN_DIR:-/opt/applications/bin}"
ICON_DIR="${ICON_DIR:-/opt/applications/icons}"
DESKTOP_DIR="${DESKTOP_DIR:-/usr/share/applications}"
SYMLINK_DIR="${SYMLINK_DIR:-/usr/local/bin}"

mkdir -p "$RAW_DIR" "$BIN_DIR" "$ICON_DIR" "$DESKTOP_DIR" "$SYMLINK_DIR"

normalize_name() {
    local input="$1"

    echo "$input" | sed -E 's/(x86_64|amd64|i386|linux|setup)//Ig' \
        | sed -E 's/-?v?[0-9]+(\.[0-9]+)*//g' \
        | sed -E 's/[-_.]+/-/g' \
        | sed -E 's/^-+|-+$//g' \
        | tr A-Z a-z
}

CLEAN="${1:-}"

# Avoid treating unmatched globs as literals.
shopt -s nullglob

if [[ "$CLEAN" == "--clean" ]]; then
    echo "Cleaning old AppImages and symlinks..."

    for file in "$BIN_DIR"/*.AppImage; do
        base=$(basename "$file" .AppImage)
        clean=$(normalize_name "$base")
        if [[ "$base" != "$clean" ]]; then
            echo "Removing: $file"
            rm -f "$file"
        fi
    done

    find "$SYMLINK_DIR" -type l -lname "$BIN_DIR/*" \
        -exec bash -c '[[ ! -e "$(readlink "{}")" ]] && echo "Removing symlink: {}" && rm -f "{}"' \; || true

    find "$DESKTOP_DIR" -type f -name "*.desktop" \
        -exec grep -Fl "$BIN_DIR/" {} \; | \
        xargs -r grep -lE '(x86_64|amd64|linux|v[0-9])' | \
        xargs -r rm -v

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

    echo "Clean complete."
fi

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

done
