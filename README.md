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
- Multi-hop onion transport with anonymous per-hop X25519 sessions, encrypted route binding, layered ChaCha20-Poly1305 relay packets, replay cache, bounded UDP relay forwarding, route fallback through shorter trusted paths, idempotent signed-ACK recovery, live end-to-end relay tests and a standalone headless smoke test
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
and graph-oriented state. Direct LAN CybChat uses authenticated peer identity, X25519 key exchange, encrypted wire envelopes, signed ACKs, TOFU pinning and a per-message key ratchet. Routed CybChat establishes a fresh anonymous X25519 session per relay, authenticates each relay through its Ed25519-signed session reply, then sends the route binding encrypted inside that hop session. Onion layers carry per-hop session IDs, route/packet IDs, hop lineage and AEAD-protected next-hop metadata; replay suppression and reverse routed ACKs complete the implemented UDP path. A routed send keeps one message ID across route attempts, can rebuild through shorter selected-relay paths after a failure, and the destination caches signed ACKs so a recovered route does not duplicate the chat event. The CybChat UI exposes explicit relay selection and preserves the selected hop order.

## What is deliberately not faked

Bluetooth, P2P, Nostr and RTSP camera feeds are shown as transport/ready states,
but the MVP does not claim them as connected. LAN discovery is implemented as a
real local peer-discovery channel. Discovery is broadcast-only; CybChat messages
are directed to a selected peer and acknowledged within a bounded timeout. The
CybChat direct-LAN payloads are encrypted with the authenticated wire envelope
and are only reported as delivered after a verified signed ACK. CybChat history plus memories, events and graph node/link payloads are encrypted before SQLite persistence after the local storage key is configured. On macOS, the key is derived from the Keychain-backed identity; non-macOS builds still retain the legacy SQLite-backed identity storage and therefore need a platform-specific secure keystore before at-rest encryption can be considered theft-resistant. Structural identifiers needed to maintain graph relationships remain visible.


## One-click macOS launch

Double-click START_cybOS.command in Finder.

The launcher:

1. installs Rust/Cargo if needed;
2. detects changes in any Rust source file, Cargo.toml or Cargo.lock;
3. builds the release binary when required;
4. creates cybOS.app;
5. launches the app.

The first build requires internet access because Cargo downloads Rust crates.


## Local testing

For the normal native UI on macOS:

```bash
./START_cybOS.command
```

For the full routed transport acceptance test:

```bash
cargo run -- --self-test all
```

For the basic in-process routed smoke test only:

```bash
cargo run -- --self-test onion
```

Expected result:

```text
ONION_TEST OK · 2 relays · cyb-... → cyb-...
```

The self-test creates a source node, two relay nodes and a destination,
performs real per-hop X25519 session establishment and encrypted route binding,
sends the end-to-end CybChat envelope through both relays, verifies the signed
reverse ACK, and confirms that the relays do not receive a chat event.

For the full Rust test suite:

```bash
cargo test --locked -- --test-threads=1
```

The CI pipeline runs the same checks on Linux and macOS and also builds a
testable macOS `cybOS.app` artifact.

## Out-of-band peer provisioning

For deployments that need stronger first-contact trust than TOFU:

```bash
cargo run -- --identity
```

Share the printed `NODE_ID` and `FINGERPRINT` through an independent channel. On the other machine, provision that expected fingerprint:

```bash
cargo run -- --provision-peer NODE_ID FINGERPRINT
```

Future LAN scans automatically trust a peer only when its authenticated public-key fingerprint matches the provisioned value. A mismatch is rejected and logged as a security event.

## Headless onion smoke test

For a local debug verification of the full routed path:

```bash
CYBOS_HEADLESS_ONION_TEST=1 cargo run
```

This starts a source, two relays and a destination, performs the real anonymous
per-hop session/bind flow, sends an encrypted CybChat payload, verifies the signed
reverse ACK, and confirms that relay nodes do not receive the destination chat event.
