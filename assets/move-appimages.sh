#!/bin/bash

RAW_DIR="/opt/applications/raw"

# Find all user home directories under /home (excluding system users)
USER_HOMES=$(find /home -mindepth 1 -maxdepth 1 -type d)

for user_home in $USER_HOMES; do
    if [ -d "$user_home" ]; then
        find "$user_home" -type f -iname '*.AppImage' -exec mv -v {} "$RAW_DIR/" \;
    fi

done

chown root:root "$RAW_DIR"/*.AppImage 2>/dev/null || true
chmod 755 "$RAW_DIR"/*.AppImage 2>/dev/null || true