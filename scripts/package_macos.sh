#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

VERSION="$(sed -n 's/^version = "\([^"]*\)"/\1/p' Cargo.toml | head -n 1)"
VERSION="\${VERSION:-0.7.0}"
export MACOSX_DEPLOYMENT_TARGET="\${MACOSX_DEPLOYMENT_TARGET:-11.0}"

DIST="$ROOT/dist"
APP="$DIST/cybOS.app"

rm -rf "$APP" "$DIST/cybOS-\${VERSION}-macOS.zip"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"

cargo build --release
cp "target/release/cybos" "$APP/Contents/MacOS/cybos"

swiftc macos/CybOSBLEAdvertiser.swift \
  -o "$APP/Contents/MacOS/cybOS-ble-advertiser" \
  -framework CoreBluetooth

cat > "$APP/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleDisplayName</key>
    <string>cybOS</string>
    <key>CFBundleExecutable</key>
    <string>cybos</string>
    <key>CFBundleIdentifier</key>
    <string>to.cicada.cybos</string>
    <key>CFBundleName</key>
    <string>cybOS</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>CFBundleShortVersionString</key>
    <string>$VERSION</string>
    <key>CFBundleVersion</key>
    <string>$VERSION</string>
    <key>LSMinimumSystemVersion</key>
    <string>11.0</string>
    <key>NSBluetoothAlwaysUsageDescription</key>
    <string>cybOS uses Bluetooth only for opt-in CYB RADAR proximity discovery.</string>
</dict>
</plist>
PLIST

if command -v xattr >/dev/null 2>&1; then
  xattr -cr "$APP" || true
fi

ditto -c -k --sequesterRsrc --keepParent "$APP" "$DIST/cybOS-\${VERSION}-macOS.zip"

echo "Created:"
echo "  $APP"
echo "  $DIST/cybOS-\${VERSION}-macOS.zip"
