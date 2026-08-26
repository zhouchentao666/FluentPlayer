#!/bin/bash
# dev.sh — tauri dev with automatic microphone entitlement codesigning
# Usage: ./scripts/dev.sh   (instead of `tauri dev`)

cd "$(dirname "$0")/.."

BINARY="src-tauri/target/debug/mio"
ENTITLEMENTS="src-tauri/Mio Music.entitlements"

# Background watcher: codesign binary whenever it changes
(
    LAST=""
    while true; do
        if [ -f "$BINARY" ]; then
            CUR=$(stat -f "%m" "$BINARY" 2>/dev/null)
            if [ "$CUR" != "$LAST" ]; then
                sleep 2
                if codesign --force --sign - --entitlements "$ENTITLEMENTS" "$BINARY" 2>/dev/null; then
                    echo "✓ Binary codesigned with microphone entitlement"
                fi
                LAST=$(stat -f "%m" "$BINARY" 2>/dev/null)
            fi
        fi
        sleep 2
    done
) &
WATCHER_PID=$!

cleanup() {
    kill "$WATCHER_PID" 2>/dev/null
    wait "$WATCHER_PID" 2>/dev/null
}
trap cleanup EXIT INT TERM

echo "Starting tauri dev with auto-codesign watcher..."
npx tauri dev "$@"
