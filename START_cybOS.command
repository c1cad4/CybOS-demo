#!/bin/bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")" && pwd)"
cd "$ROOT"

APP="$ROOT/cybOS.app"
BIN="$ROOT/target/release/cybos"

# Prefer an existing Cargo installation, then the standard rustup location.
if ! command -v cargo >/dev/null 2>&1; then
  if [ -x "$HOME/.cargo/bin/cargo" ]; then
    export PATH="$HOME/.cargo/bin:$PATH"
  else
    echo "Rust/Cargo is not installed."
    echo "Install Rust from https://rustup.rs/ and run this launcher again."
    exit 1
  fi
fi

need_build=0
if [ ! -x "$BIN" ]; then
  need_build=1
elif find src macos -type f -newer "$BIN" -print -quit | grep -q .; then
  need_build=1
elif [ "Cargo.toml" -nt "$BIN" ] || [ "Cargo.lock" -nt "$BIN" ]; then
  need_build=1
fi

if [ "$need_build" = "1" ]; then
  echo "Building cybOS release..."
  cargo build --release
fi

if [ ! -x "$BIN" ]; then
  echo "Release binary was not produced: $BIN"
  exit 1
fi

echo "Creating cybOS.app..."
rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
cp "$BIN" "$APP/Contents/MacOS/cybOS"
chmod +x "$APP/Contents/MacOS/cybOS"

BLE_HELPER="$APP/Contents/MacOS/cybOS-ble-advertiser"
if command -v swiftc >/dev/null 2>&1; then
  echo "Building native CoreBluetooth advertiser..."
  swiftc macos/CybOSBLEAdvertiser.swift -o "$BLE_HELPER" -framework CoreBluetooth
  chmod +x "$BLE_HELPER"
else
  echo "Swift compiler not found; BLE advertising helper will be unavailable."
fi

cat > "$APP/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
<key>CFBundleName</key><string>cybOS</string>
<key>CFBundleDisplayName</key><string>cybOS — CicadaFarm + RobotCYB</string>
<key>CFBundleIdentifier</key><string>to.cicada.cybos</string>
<key>CFBundleVersion</key><string>0.7.0</string>
<key>CFBundleShortVersionString</key><string>0.7.0</string>
<key>CFBundleExecutable</key><string>cybOS</string>
<key>CFBundlePackageType</key><string>APPL</string>
<key>LSMinimumSystemVersion</key><string>11.0</string>
<key>NSHighResolutionCapable</key><true/>
<key>NSBluetoothAlwaysUsageDescription</key><string>cybOS uses Bluetooth Low Energy for opt-in CYB RADAR proximity discovery.</string>
</dict></plist>
PLIST

# Finder may quarantine files downloaded from the internet. Clear quarantine
# only for the app we just built; this does not disable Gatekeeper globally.
xattr -dr com.apple.quarantine "$APP" 2>/dev/null || true

echo "Launching cybOS..."
open "$APP"
