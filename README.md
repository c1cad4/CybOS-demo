# cybOS 0.7 — CicadaFarm + RobotCYB + CybChat

Native macOS-first desktop MVP. The goal is a fast local-first application,
not a browser wrapper.

## Core implemented

- Native egui/eframe desktop UI
- CicadaFarm dashboard and farm event log
- RobotCYB local contextual assistant
- CybChat local conversation shell with persistent SQLite history
- Cybergraph with nodes, links, zoom/pan and inspector
- Persistent SQLite event, memory, graph and node-identity storage
- Search/navigation shell with native page routing
- Local Qwen runtime shared by planner, learning, RobotCYB and web answers
- Real local LAN peer discovery with UDP broadcast and node/version reporting
- Directed LAN CybChat with explicit recipient identity, message IDs and bounded delivery acknowledgements
- Background market and Solana balance refreshes that do not block UI rendering
- Public $CICADAFARM and $ROBOTCYB mint identifiers
- No private keys
- Rust CI checks for push and pull requests

## Architecture

The eframe application trait is kept in src/shell.rs.
The complete native shell composition lives in src/ui/shell.rs.
Brain planning, learning, tools, web intent and Qwen transport are separated
into dedicated modules.

The application is local-first: UI and storage do not require a web browser,
and infrastructure is not presented as live until a real connection exists.

The communication design borrows the useful protocol ideas from the Cyberia CybOS
concept: persistent node identity, explicit message addressing, bounded delivery
and graph-oriented state. Direct LAN CybChat now uses authenticated peer identity, X25519 key exchange, encrypted wire envelopes, signed ACKs, TOFU pinning and a per-message key ratchet.

## What is deliberately not faked

Bluetooth, P2P, Nostr and RTSP camera feeds are shown as transport/ready states,
but the MVP does not claim them as connected. LAN discovery is implemented as a
real local peer-discovery channel. Discovery is broadcast-only; CybChat messages
are directed to a selected peer and acknowledged within a bounded timeout. The
payload is still plaintext and must not carry secrets. End-to-end encryption is
not implemented yet.


## One-click macOS launch

Double-click START_cybOS.command in Finder.

The launcher:

1. installs Rust/Cargo if needed;
2. detects changes in any Rust source file, Cargo.toml or Cargo.lock;
3. builds the release binary when required;
4. creates cybOS.app;
5. launches the app.

The first build requires internet access because Cargo downloads Rust crates.
