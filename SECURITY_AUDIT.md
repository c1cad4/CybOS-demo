# cybOS Secure CybChat Security Audit

Date: 2026-10-07
Scope: direct-LAN discovery, identity, handshake, wire envelope, ratchet, replay handling, ACK delivery, local storage, routed onion transport, and documentation.

## Audit result

The direct LAN design has a sound primitive selection and an explicit security boundary, but the initial implementation contained two merge-blocking state/integration defects:

The branch now contains the routed CybChat path: each relay establishes a fresh anonymous X25519 session, authenticates its own session reply with its Ed25519 identity, and receives an AEAD-encrypted route binding. Onion packets use per-hop session IDs and scoped AEAD keys, enforce TTL and hop/route lineage, suppress replay, forward over bounded UDP, and carry reverse routed ACKs. The UI exposes explicit relay selection and ordered hop construction. The route setup does not send the source Ed25519 identity to the relay.

1. The runtime persisted a random node ID unrelated to the persistent Ed25519 identity. This could make authenticated discovery reject the local peer because the announced node ID did not equal the hash-derived identity ID.
2. A lost delivery ACK could leave the sender reusing a stale ratchet session while the receiver had already advanced its chain. Subsequent messages could then be rejected until the process was restarted.

Both defects were fixed on this branch.

## Fixed during audit

- Node ID is now always derived from the persistent Ed25519 public key.
- Sender-side ratchet session is dropped after an ACK timeout so the next send performs a fresh X25519 handshake.
- Receiver only advances its chain after successful decryption and successful next-chain derivation.
- Inbound session state is bounded to 128 concurrent peer sessions.
- Local Unix runtime directory is hardened to mode 0700 and the SQLite database to mode 0600 when permissions can be changed.
- On macOS, the Ed25519 private identity is stored in the system Keychain instead of SQLite.
- Existing macOS SQLite-backed identities are migrated to Keychain once and removed from the database.
- On macOS, CybChat history is encrypted with ChaCha20-Poly1305 before being written to SQLite.
- Existing plaintext chat rows are migrated to encrypted rows on first read after the storage key is configured.
- The chat table carries an explicit encrypted-state column so plaintext content cannot be confused with an encrypted payload.
- README, GUIDE and CybChat UI describe the implemented wire-level E2E and routed transport boundary accurately.
- Security guide now distinguishes live transport behavior from integration-test targets.
- AEAD AAD/ciphertext tampering tests and wrong-recipient envelope checks were added.
- The unused envelope signing helper now includes the ratchet counter so it cannot drift from the live transcript format.

## Security properties present

- Persistent Ed25519 identity.
- Node ID derived from SHA-256(public key).
- TOFU peer-key pinning.
- Ephemeral X25519 handshake.
- HKDF-SHA256 session derivation.
- ChaCha20-Poly1305 confidentiality and integrity.
- Ed25519 authentication of key-exchange and message transcripts.
- AAD binds message ID, sender, recipient, timestamp and counter.
- Monotonic ratchet counter enforcement.
- UUID replay suppression plus a five-minute timestamp window.
- Signed delivery ACK before reporting E2E delivery.
- Direct forward secrecy from one-shot X25519 session keys.
- Routed CybChat is implemented as a bounded UDP onion transport; it is not an
  anonymity network and does not hide traffic metadata, route membership or
  network endpoints from every observer.

## Residual risks / next hardening

### MEDIUM — First contact still depends on operator trust
A new peer is now shown with its SHA-256 public-key fingerprint and requires explicit "TRUST KEY" confirmation. This makes first-contact trust visible, but it is still TOFU: the operator should compare the fingerprint through an independent channel when identity authenticity matters.

Recommended next step: support out-of-band key provisioning for deployments that need stronger initial trust.

### MEDIUM — Other local application data is not encrypted at rest
On macOS, the Ed25519 private identity is protected by the system Keychain and CybChat history is now AEAD-encrypted in SQLite. Other persisted data such as memories, events and graph state remains unencrypted application data. Unix file permissions are hardened, but a process or account-level local compromise could expose those records.

Recommended next step: extend the same encrypted-storage policy to any additional sensitive persisted datasets that require it.

### MEDIUM — Adversarial integration coverage is still bounded
The harness now covers live routed delivery, replay injection and ciphertext tampering across three real cybOS child processes, in addition to the existing in-process direct-LAN and routed tests. Some broader process-level fault cases are still not covered.

Recommended next step: extend the process harness with controlled relay crash, route-expiry, malformed-binding and mid-route packet-drop scenarios.

### LOW — Discovery address is not cryptographically bound
The signed discovery response authenticates node ID, version and public key, but not the observed source address. A valid discovery response can therefore be replayed from a different address. The subsequent signed handshake prevents identity impersonation, but stale/misrouted discovery can still cause connection failure or denial of service.

Recommended next step: include a challenge/nonce and observed endpoint in the discovery exchange, or treat discovery only as a locator and require the authenticated handshake before showing the peer as fully trusted.

### LOW — Traffic metadata remains visible
The direct-LAN channel still exposes packet timing, approximate size, node IDs, message IDs, counters and network endpoints to a passive LAN observer. Encryption protects message content, not traffic analysis.

### LOW — Ratchet is ordered and symmetric, not a Double Ratchet
The current design is a per-message HKDF chain with a monotonic counter. It is not the Signal Double Ratchet and does not provide asynchronous out-of-order message handling or a DH ratchet after every message.

## Merge gate

Do not claim "production-grade secure messenger" yet.

For this branch, the direct-LAN transport and the onion transport core are suitable to merge only while the residual risks above remain explicit. The current onion path is a complete routed CybChat send/ack path for the implemented UDP transport. The next security milestone is broader fault injection at process boundaries and route lifecycle hardening.


Current automated integration coverage also includes:
- live UDP rejection of stale envelopes
- live UDP rejection of wrong-recipient envelopes
- forged ACK signature rejection
- persistent TOFU key replacement rejection


### Multi-hop onion transport

Implemented:
- layered ChaCha20-Poly1305 onion packets;
- unique per-hop anonymous X25519 session IDs and keys;
- Ed25519 authentication of relay session replies;
- AEAD-encrypted route binding and encrypted route acknowledgement;
- source-identity non-disclosure in route setup;
- route ID, packet ID, hop-index and nested-packet lineage validation;
- bounded 120-second route expiry;
- per-relay replay cache;
- route binding table capped at 256 entries;
- onion session state capped at 256 entries with endpoint binding;
- live UDP relay forwarding;
- live three-relay loopback forwarding test;
- process-isolated routed delivery, replay and ciphertext-tampering tests;
- forged encrypted route-ack rejection;
- regression test proving route-bind payload does not expose source node ID/public key.

Completed:
- automatic anonymous per-hop X25519 session establishment;
- encrypted route binding;
- reverse routed signed destination ACK delivery;
- explicit route selection and ordered relay hops in CybChat UI.

Still to harden:
- relay crash/failure recovery during an active route;
- route-expiry refresh under long-lived sessions;
- malformed route-binding and packet-drop fault injection across process boundaries;
- broader traffic-analysis and endpoint privacy protections.
