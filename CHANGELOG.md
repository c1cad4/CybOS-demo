# cybOS 0.7.0

Core release for the native local-first cybOS runtime.

## Security and communication

- LAN discovery is broadcast-only; CybChat delivery is directed to a selected peer.
- Direct LAN CybChat uses authenticated Ed25519 identity, ephemeral X25519 key exchange, HKDF-SHA256, ChaCha20-Poly1305 encrypted envelopes, signed acknowledgements, TOFU key pinning and a per-message ratchet.
- CybChat local history is encrypted at rest after the local storage key is configured, with migration support for legacy plaintext rows.
- Memories, events and graph node/link payloads are encrypted before SQLite persistence after the local storage key is configured.
- Routed CybChat uses anonymous per-hop X25519 sessions, encrypted route bindings, layered ChaCha20-Poly1305 packets, replay protection, bounded route state, reverse routed ACKs and live UDP integration tests.
- Route fallback preserves the message ID across attempts and records relay health for future route selection.
- Bluetooth, P2P, Nostr and RTSP camera paths remain adapter/ready states and are not presented as connected.

## Core application

- Native egui/eframe desktop UI
- CicadaFarm dashboard and farm event log
- RobotCYB local contextual assistant
- CybChat local conversation shell with persistent encrypted SQLite history
- Cybergraph with nodes, links, zoom/pan and inspector
- Search/navigation shell with native page routing
- Local Qwen runtime shared by planner, learning, RobotCYB and web answers
- Background market and Solana balance refreshes that do not block UI rendering
- Public $CICADAFARM and $ROBOTCYB mint identifiers
- Linux and macOS CI with locked dependencies, headless onion smoke tests, unit/integration tests and macOS app packaging
