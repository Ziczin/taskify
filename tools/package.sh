#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT="${1:-$ROOT/dist}"
mkdir -p "$OUT"

VERSION="$(awk -F'"' '/^version = / { print $2; exit }' "$ROOT/Cargo.toml")"
BUILD="${VERSION%%.*}"
if [ -z "$BUILD" ]; then
    echo "cannot read version from Cargo.toml" >&2
    exit 1
fi

if [ "${SKIP_BUILD:-0}" != "1" ]; then
    cargo build --release --locked --manifest-path "$ROOT/Cargo.toml"
fi

BIN="$ROOT/target/release/taskify"
if [ ! -x "$BIN" ]; then
    echo "not found: $BIN" >&2
    exit 1
fi

OS="$(uname -s)"
ARCH="$(uname -m)"

case "$OS" in
    Linux)
        NAME="taskify-$BUILD-linux-$ARCH"
        DIR="$OUT/$NAME"
        rm -rf "$DIR"
        mkdir -p "$DIR"
        cp "$BIN" "$DIR/taskify"
        chmod +x "$DIR/taskify"
        cp -R "$ROOT/assets" "$DIR/assets"
        cp "$ROOT/README.md" "$ROOT/README.ru.md" "$ROOT/LICENSE" "$DIR/"
        rm -f "$OUT/$NAME.tar.gz"
        tar -czf "$OUT/$NAME.tar.gz" -C "$OUT" "$NAME"
        echo "$OUT/$NAME.tar.gz"
        ;;
    Darwin)
        NAME="taskify-$BUILD-macos-$ARCH"
        DIR="$OUT/$NAME"
        APP="$DIR/Taskify.app"
        rm -rf "$DIR"
        mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
        cp "$BIN" "$APP/Contents/MacOS/taskify"
        chmod +x "$APP/Contents/MacOS/taskify"
        cp -R "$ROOT/assets" "$APP/Contents/Resources/assets"
        cp "$ROOT/README.md" "$ROOT/README.ru.md" "$ROOT/LICENSE" "$APP/Contents/Resources/"
        cat > "$APP/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleDevelopmentRegion</key>
    <string>en</string>
    <key>CFBundleExecutable</key>
    <string>taskify</string>
    <key>CFBundleIdentifier</key>
    <string>io.github.ziczin.taskify</string>
    <key>CFBundleInfoDictionaryVersion</key>
    <string>6.0</string>
    <key>CFBundleName</key>
    <string>Taskify</string>
    <key>CFBundleDisplayName</key>
    <string>Taskify</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>CFBundleShortVersionString</key>
    <string>$VERSION</string>
    <key>CFBundleVersion</key>
    <string>$BUILD</string>
    <key>LSMinimumSystemVersion</key>
    <string>11.0</string>
    <key>NSHighResolutionCapable</key>
    <true/>
</dict>
</plist>
PLIST
        if command -v codesign >/dev/null 2>&1; then
            # Ad-hoc signature: enough for a local build, not notarization.
            codesign --force --deep --sign - "$APP" >/dev/null 2>&1 || true
        fi
        rm -f "$OUT/$NAME.zip"
        ditto -c -k --sequesterRsrc --keepParent "$APP" "$OUT/$NAME.zip"
        echo "$OUT/$NAME.zip"
        ;;
    *)
        echo "unsupported OS: $OS" >&2
        exit 1
        ;;
esac
