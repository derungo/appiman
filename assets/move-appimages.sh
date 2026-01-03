#!/usr/bin/env bash

RAW_DIR="${RAW_DIR:-/opt/applications/raw}"
HOME_ROOT="${HOME_ROOT:-/home}"

mkdir -p "$RAW_DIR"

# Avoid treating unmatched globs as literals.
shopt -s nullglob

for user_home in "$HOME_ROOT"/*; do
    [[ -d "$user_home" ]] || continue

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

done

chown root:root "$RAW_DIR"/*.AppImage 2>/dev/null || true
chmod 755 "$RAW_DIR"/*.AppImage 2>/dev/null || true
