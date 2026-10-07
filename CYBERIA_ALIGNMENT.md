# cybOS × Cyberia design adoption

The current native macOS cybOS now adopts the useful architectural principles without pretending to be a replacement kernel.

## Adopted

### Identity
Each node has a persistent Ed25519 identity stored locally in SQLite. The cyb-* node ID is derived from SHA-256(public key), binding the logical identity to the cryptographic key.

### Cells
src/cell.rs defines explicit cell boundaries with input/output, Idle/Running/Completed/TimedOut/Failed status, heartbeat metadata and a declared deadline. Existing Brain, Network, Assets, Graph and RobotCYB paths can migrate incrementally.

### Bounded runtime
Network work already runs outside the UI thread. The cell contract makes deadline and liveness explicit.

### Directed communication
LAN discovery may broadcast, but directed communication addresses logical nodes with from/to/message_id fields.

### E2E CybChat primitives
src/crypto.rs provides persistent Ed25519 authentication, per-message ephemeral X25519 agreement, HKDF-SHA256 derivation and ChaCha20-Poly1305 authenticated encryption. The protocol is experimental: ratcheting, replay windows, key rotation and full conversation state remain future work.

### Delivery state
LAN delivery has an explicit ACK with bounded timeout. This is not a cryptographic proof of route integrity.

## Deferred

Onion routing comes only after authenticated encrypted transport is stable. The full no-filesystem/new-kernel Cyberia model is also deferred because the current target is a native macOS application.

## Next protocol work

1. Integrate crypto.rs into the LAN CybChat wire envelope.
2. Advertise and authenticate peer public keys.
3. Add replay protection with sequence/timestamp windows.
4. Add session key rotation and a stateful ratchet.
5. Move Brain / RobotCYB / Assets / Graph operations behind CellSpec contracts.
6. Evaluate relay and onion-routing layers only after the above.
