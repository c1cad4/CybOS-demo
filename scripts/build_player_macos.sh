#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
mkdir -p target/debug/swift-module-cache
swiftc -module-cache-path target/debug/swift-module-cache macos/CybOSPlayer.swift -o target/debug/cybos-player -framework AppKit -framework AVKit -framework AVFoundation
python3 scripts/package_player_macos.py --binary target/debug/cybos-player --output target/debug/CybPlayer.app
