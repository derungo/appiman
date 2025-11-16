#!/bin/bash

RAW_DIR="/opt/applications/raw"
BIN_DIR="/opt/applications/bin"
ICON_DIR="/opt/applications/icons"
DESKTOP_DIR="/usr/share/applications"
SYMLINK_DIR="/usr/local/bin"

mkdir -p "$BIN_DIR" "$ICON_DIR" "$DESKTOP_DIR"

CLEAN="$1"

if [[ "$CLEAN" == "--clean" ]]; then
    echo "Cleaning old AppImages and symlinks..."
    for file in "$BIN_DIR"/*.AppImage; do
        base=$(basename "$file" .AppImage)
        clean=$(echo "$base" | sed -E 's/(x86_64|amd64|i386|linux|setup)//Ig' \
                                | sed -E 's/-?v?[0-9]+(\.[0-9]+)*//g' \
                                | sed -E 's/[-_.]+/-/g' \
                                | sed -E 's/^-+|-+$//g' \
                                | tr A-Z a-z)
        if [[ "$base" != "$clean" ]]; then
            echo "Removing: $file"
            rm -f "$file"
        fi
    done

    find "$SYMLINK_DIR" -type l -lname "$BIN_DIR/*" \
        -exec bash -c '[[ ! -e "$(readlink {})" ]] && echo "Removing symlink: {}" && rm -f {}' \; || true

    find "$DESKTOP_DIR" -type f -name "*.desktop" \
        -exec grep -l "/opt/applications/bin/" {} \; | \
        xargs -r grep -lE '(x86_64|amd64|linux|v[0-9])' | \
        xargs -r rm -v

    for icon in "$ICON_DIR"/*.png; do
        base=$(basename "$icon" .png)
        clean=$(echo "$base" | sed -E 's/(x86_64|amd64|i386|linux|setup)//Ig' \
                                | sed -E 's/-?v?[0-9]+(\.[0-9]+)*//g' \
                                | sed -E 's/[-_.]+/-/g' \
                                | sed -E 's/^-+|-+$//g' \
                                | tr A-Z a-z)
        [[ "$base" != "$clean" ]] && echo "Removing: $icon" && rm -f "$icon"
    done

    echo "Clean complete."
fi

for app in "$RAW_DIR"/*.AppImage; do
    [ -f "$app" ] || continue

    base=$(basename "$app" .AppImage)
    clean=$(echo "$base" | sed -E 's/(x86_64|amd64|i386|linux|setup)//Ig' \
                         | sed -E 's/-?v?[0-9]+(\.[0-9]+)*//g' \
                         | sed -E 's/[-_.]+/-/g' \
                         | sed -E 's/^-+|-+$//g' \
                         | tr A-Z a-z)

    dest="$BIN_DIR/$clean.AppImage"
    cp --update=none "$app" "$dest"
    chmod +x "$dest"
    ln -sf "$dest" "$SYMLINK_DIR/$clean"

    tmp_dir=$(mktemp -d)
    "$dest" --appimage-extract > /dev/null 2>&1
    if ! cd squashfs-root; then
     echo "❌ Failed to extract $app"
     rm -rf "$tmp_dir"
    continue
    fi

    app_desktop=$(find . -name "*.desktop" | head -n 1)
    name="${clean^}"
    icon_path="$ICON_DIR/$clean.png"
    category="Utility"

    if [[ -f "$app_desktop" ]]; then
        name=$(grep -m1 "^Name=" "$app_desktop" | cut -d= -f2- || echo "$name")
        category=$(grep -m1 "^Categories=" "$app_desktop" | cut -d= -f2- || echo "$category")
        icon_key=$(grep -m1 "^Icon=" "$app_desktop" | cut -d= -f2- || echo "")

        if [[ -n "$icon_key" ]]; then
            found=$(find . -type f \( -iname "$icon_key.png" -o -iname "$icon_key*.svg" \) | head -n1)
            [[ -n "$found" ]] && cp --update=none "$found" "$icon_path"
        fi
    fi

    [[ ! -f "$icon_path" ]] && cp --update=none *.png "$icon_path" 2>/dev/null || true

    cd .. && rm -rf squashfs-root "$tmp_dir"

    cat > "$DESKTOP_DIR/$clean.desktop" <<EOF
[Desktop Entry]
Type=Application
Name=$name
Exec=$dest
Icon=$icon_path
Terminal=false
Categories=$category
EOF

    chmod +x "$DESKTOP_DIR/$clean.desktop"
    "$dest" --appimage-update > /dev/null 2>&1 || true

done