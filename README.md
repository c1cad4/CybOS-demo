# cybOS 0.6 — CicadaFarm + RobotCYB + CybChat

Native macOS-first desktop MVP. The goal is a fast local-first application, not a browser wrapper.

## What works now
- Native egui/eframe desktop UI
- CicadaFarm dashboard and farm event log
- RobotCYB local contextual assistant
- CybChat local conversation shell
- Cybergraph with nodes, links, zoom/pan and inspector
- Persistent SQLite event store and node identity
- Search field / navigation shell
- Public $CICADAFARM and $ROBOTCYB mint identifiers
- No private keys

## What is deliberately not faked
Bluetooth, LAN, P2P and Nostr are shown as transport adapters, but the MVP does not claim a live connection. Real transport integration is the next engineering layer.

## One-click macOS launch
Double-click `START_cybOS.command` in Finder. On the first run it installs Rust/Cargo if needed, builds the release binary, creates `cybOS.app`, and launches it. Later launches reuse the built app.

The script requires internet access on the first build because Cargo downloads Rust crates.
