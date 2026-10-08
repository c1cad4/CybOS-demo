#!/bin/bash
set -euo pipefail

cd "$(dirname "$0")"

TARGET_DIR="$PWD/target/cybos-qa"
BIN="$TARGET_DIR/release/cybos"
APP="$PWD/cybOS-QA.app"

if ! command -v cargo >/dev/null 2>&1; then
  echo "Rust/Cargo is not installed."
  echo "Install Rust from https://rustup.rs/ and run this script again."
  exit 1
fi

echo "== cybOS QA build =="
CARGO_TARGET_DIR="$TARGET_DIR" cargo build --release --locked --features qa --bin cybos

echo "== Packaging cybOS-QA.app =="
rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
cp "$BIN" "$APP/Contents/MacOS/cybOS"
cat > "$APP/Contents/Info.plist" <<'PLIST'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
<key>CFBundleName</key><string>cybOS QA</string>
<key>CFBundleDisplayName</key><string>cybOS QA — CicadaFarm + RobotCYB</string>
<key>CFBundleIdentifier</key><string>to.cicada.cybos.qa</string>
<key>CFBundleVersion</key><string>0.7.0</string>
<key>CFBundleShortVersionString</key><string>0.7.0</string>
<key>CFBundleExecutable</key><string>cybOS</string>
<key>CFBundlePackageType</key><string>APPL</string>
<key>LSMinimumSystemVersion</key><string>11.0</string>
<key>NSHighResolutionCapable</key><true/>
</dict></plist>
PLIST

chmod +x "$APP/Contents/MacOS/cybOS"

if command -v codesign >/dev/null 2>&1; then
  codesign --force --deep --sign - "$APP" >/dev/null 2>&1 || true
fi

if command -v xattr >/dev/null 2>&1; then
  xattr -dr com.apple.quarantine "$APP" >/dev/null 2>&1 || true
fi

echo "== cybOS QA: two-hop onion self-test =="
"$BIN" --self-test onion

echo "== cybOS QA: relay failure recovery =="
"$BIN" --self-test onion-process

echo
echo "QA SELF-TESTS PASSED"
echo "Launching cybOS-QA.app..."
open "$APP"
