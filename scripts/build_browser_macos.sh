#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
mkdir -p target/debug/swift-module-cache
swiftc -module-cache-path target/debug/swift-module-cache macos/CybOSBrowser.swift -o target/debug/cybos-browser -framework AppKit -framework WebKit
python3 scripts/package_browser_macos.py --binary target/debug/cybos-browser --output target/debug/CybBrowser.app
