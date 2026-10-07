# cybOS architecture

cybOS 0.7 is a native desktop system with a thin application shell, domain services, local persistence, bounded cells and page-oriented UI.

## Core layers

- Application shell: src/shell.rs
- State: src/state.rs
- Persistence: src/store.rs
- Runtime: src/runtime/
- Cryptographic identity: src/identity.rs
- E2E message primitives: src/crypto.rs
- Bounded cell contract: src/cell.rs
- Brain: src/brain/
- Network: src/network/
- LAN: src/network/lan.rs
- UI: src/ui/

## Cell contract

A domain operation exposes input -> cell -> output and publishes status, heartbeat, deadline and error. A cell must not reach directly into another cell's internal state; cross-cell communication should use explicit messages or state services.

## Secure CybChat contract

LAN discovery -> peer identity -> X25519 ephemeral agreement -> HKDF-SHA256 -> ChaCha20-Poly1305 -> Ed25519 envelope authentication.

The current crypto primitives are ready for integration into the direct LAN message envelope. Until that integration is complete, the existing LAN transport remains plaintext and is explicitly labelled that way.
