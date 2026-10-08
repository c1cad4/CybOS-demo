#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

VERSION="$(sed -n 's/^version = "\([^"]*\)"/\1/p' Cargo.toml | head -n 1)"
VERSION="${VERSION:-0.7.2}"
export MACOSX_DEPLOYMENT_TARGET="${MACOSX_DEPLOYMENT_TARGET:-11.0}"

DIST="$ROOT/dist"
APP="$DIST/cybOS.app"
BUILD="$DIST/.universal-build"

rm -rf "$APP" "$BUILD" "$DIST/cybOS-${VERSION}-macOS.zip"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources" "$BUILD"

echo "Installing Rust targets..."
rustup target add aarch64-apple-darwin x86_64-apple-darwin

echo "Building cybOS for Apple Silicon..."
cargo build --release --target aarch64-apple-darwin

echo "Building cybOS for Intel..."
cargo build --release --target x86_64-apple-darwin

echo "Creating universal cybOS binary..."
lipo -create "target/aarch64-apple-darwin/release/cybos" "target/x86_64-apple-darwin/release/cybos" -output "$APP/Contents/MacOS/cybos"
chmod +x "$APP/Contents/MacOS/cybos"

echo "Building universal CoreBluetooth helper..."
swiftc -arch arm64 macos/CybOSBLEAdvertiser.swift -o "$BUILD/cybOS-ble-arm64" -framework CoreBluetooth
swiftc -arch x86_64 macos/CybOSBLEAdvertiser.swift -o "$BUILD/cybOS-ble-x86_64" -framework CoreBluetooth
lipo -create "$BUILD/cybOS-ble-arm64" "$BUILD/cybOS-ble-x86_64" -output "$APP/Contents/MacOS/cybOS-ble-advertiser"
chmod +x "$APP/Contents/MacOS/cybOS-ble-advertiser"

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
    <string>${VERSION}</string>
    <key>CFBundleVersion</key>
    <string>${VERSION}</string>
    <key>LSMinimumSystemVersion</key>
    <string>11.0</string>
    <key>NSHighResolutionCapable</key>
    <true/>
    <key>NSBluetoothAlwaysUsageDescription</key>
    <string>cybOS uses Bluetooth only for opt-in CYB RADAR proximity discovery.</string>
</dict>
</plist>
PLIST

echo "Verifying universal binaries..."
lipo -info "$APP/Contents/MacOS/cybos"
lipo -info "$APP/Contents/MacOS/cybOS-ble-advertiser"

if command -v xattr >/dev/null 2>&1; then
  xattr -cr "$APP" || true
fi

ditto -c -k --sequesterRsrc --keepParent "$APP" "$DIST/cybOS-${VERSION}-macOS.zip"

rm -rf "$BUILD"

echo "Created:"
echo "  $APP"
echo "  $DIST/cybOS-${VERSION}-macOS.zip"
