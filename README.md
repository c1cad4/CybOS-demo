# cybOS 0.7 — CicadaFarm + RobotCYB + CybChat

Native macOS-first desktop MVP. The goal is a fast local-first application,
not a browser wrapper.

## Core implemented

- Native egui/eframe desktop UI
- CicadaFarm dashboard and farm event log
- RobotCYB local contextual assistant
- CybChat local conversation shell with persistent SQLite history
- Cybergraph with nodes, links, zoom/pan and inspector
- Persistent SQLite event, memory and graph storage
- macOS Keychain-backed Ed25519 node identity with one-time migration from legacy SQLite identity storage
- macOS CybChat history encrypted at rest with ChaCha20-Poly1305 and automatic migration of legacy plaintext chat rows
- Search/navigation shell with native page routing
- Local Qwen runtime shared by planner, learning, RobotCYB and web answers
- Real local LAN peer discovery with UDP broadcast and node/version reporting
- Directed LAN CybChat with explicit recipient identity, message IDs and bounded delivery acknowledgements
- Multi-hop onion transport core with layered ChaCha20-Poly1305 relay packets, route-scoped key derivation, replay cache and bounded UDP relay forwarding
- Background market and Solana balance refreshes that do not block UI rendering
- Public $CICADAFARM and $ROBOTCYB mint identifiers
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
and graph-oriented state. Direct LAN CybChat uses authenticated peer identity, X25519 key exchange, encrypted wire envelopes, signed ACKs, TOFU pinning and a per-message key ratchet. Routed CybChat adds authenticated per-hop sessions, signed route binding, nested relay layers, route/packet IDs, hop binding, replay suppression, reverse routed ACKs and a real UDP forwarding path. The CybChat UI exposes explicit relay selection and preserves the selected hop order.

## What is deliberately not faked

Bluetooth, P2P, Nostr and RTSP camera feeds are shown as transport/ready states,
but the MVP does not claim them as connected. LAN discovery is implemented as a
real local peer-discovery channel. Discovery is broadcast-only; CybChat messages
are directed to a selected peer and acknowledged within a bounded timeout. The
CybChat direct-LAN payloads are encrypted with the authenticated wire envelope
and are only reported as delivered after a verified signed ACK. CybChat history remains stored in SQLite as encrypted application data on macOS; other local datasets are not yet encrypted at rest.


## One-click macOS launch

Double-click START_cybOS.command in Finder.

The launcher:

1. installs Rust/Cargo if needed;
2. detects changes in any Rust source file, Cargo.toml or Cargo.lock;
3. builds the release binary when required;
4. creates cybOS.app;
5. launches the app.

The first build requires internet access because Cargo downloads Rust crates.
