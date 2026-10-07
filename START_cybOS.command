#!/bin/bash
set -e
cd "$(dirname "$0")"
APP="$PWD/cybOS.app"
BIN="$PWD/target/release/cybos"
if ! command -v cargo >/dev/null 2>&1; then
  echo "Rust/Cargo is not installed. Installing rustup..."
  if ! command -v curl >/dev/null 2>&1; then echo "curl is required."; exit 1; fi
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
  source "$HOME/.cargo/env"
fi
source "$HOME/.cargo/env" 2>/dev/null || true

need_build=0
if [ ! -x "$BIN" ]; then
  need_build=1
elif find src -type f -name '*.rs' -newer "$BIN" -print -quit | grep -q .; then
  need_build=1
elif [ "Cargo.toml" -nt "$BIN" ] || [ "Cargo.lock" -nt "$BIN" ]; then
  need_build=1
fi
if [ "$need_build" = "1" ]; then
  echo "Building cybOS..."
  cargo build --release
fi

echo "Creating cybOS.app..."
rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
cp "$BIN" "$APP/Contents/MacOS/cybOS"
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
</dict></plist>
PLIST
open "$APP"
echo "cybOS launched. You can close this Terminal window."
