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
- Routed sends can preserve one message ID across bounded fallback route attempts.
- Recently delivered message IDs retain signed ACKs in a bounded cache so a recovered route can acknowledge a duplicate without re-delivering the chat payload.
- A recovered destination session advances its ratchet counter when accepting a valid duplicate envelope, keeping both sides synchronized.
- Receiver only advances its chain after successful decryption and successful next-chain derivation.
- Inbound session state is bounded to 128 concurrent peer sessions.
- Local Unix runtime directory is hardened to mode 0700 and the SQLite database to mode 0600 when permissions can be changed.
- On macOS, the Ed25519 private identity is stored in the system Keychain instead of SQLite.
- Existing macOS SQLite-backed identities are migrated to Keychain once and removed from the database.
- On macOS, CybChat history is encrypted with ChaCha20-Poly1305 before being written to SQLite.
- Memories, events and graph node/link payloads are also encrypted with the same ChaCha20-Poly1305 storage policy after the storage key is configured. On macOS this key is derived from the Keychain-backed identity; non-macOS builds still store the legacy identity in SQLite.
- Existing plaintext rows in those datasets are migrated to encrypted rows on first read after the storage key is configured.
- The chat table carries an explicit encrypted-state column so plaintext content cannot be confused with an encrypted payload.
- README, GUIDE and CybChat UI describe the implemented wire-level E2E and routed transport boundary accurately.
- Security guide now distinguishes live transport behavior from integration-test targets.
- AEAD AAD/ciphertext tampering tests and wrong-recipient envelope checks were added.
- The unused envelope signing helper now includes the ratchet counter so it cannot drift from the live transcript format.

## Security properties present

- Persistent Ed25519 identity.
- Node ID derived from SHA-256(public key).
- Discovery responses are bound to a fresh per-scan challenge nonce, preventing stale response replay across scans.\n- TOFU peer-key pinning.
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
- Onion session admission uses a relay-signed stateless cookie bound to the
  observed UDP endpoint, session ID and initiator ephemeral key before relay
  session state is allocated.

## Residual risks / next hardening

### MEDIUM — First contact can use TOFU or out-of-band provisioning
A new peer is shown with its SHA-256 public-key fingerprint and normally requires explicit "TRUST KEY" confirmation. For deployments that need stronger initial trust, cybOS now supports out-of-band fingerprint provisioning through the CLI:

`cybOS --identity` exports the local node ID and fingerprint, and
`cybOS --provision-peer NODE_ID FINGERPRINT` stores the expected fingerprint.
A subsequent LAN scan automatically pins the peer's authenticated public key only when the observed fingerprint matches the provisioned value; mismatches are rejected and logged.

TOFU remains available for interactive local deployments where independent fingerprint verification is not required.

### MEDIUM — Local storage still exposes structural metadata
On macOS, the Ed25519 private identity is protected by the system Keychain. CybChat history, memories, events and graph node/link payloads are AEAD-encrypted before SQLite persistence using a key derived from that Keychain-backed identity. Legacy plaintext rows are migrated on first access. Non-macOS builds still keep the legacy identity in SQLite, so their local-storage encryption is not yet a theft-resistant keystore boundary. Primary/relationship identifiers needed for graph integrity, row counts and SQLite file metadata remain observable.

A process with access to the running application can still request decrypted records through the live process, and the current design does not encrypt the SQLite schema itself.

### MEDIUM — Adversarial integration coverage is still bounded
The harness covers live routed delivery, replay injection and ciphertext tampering
in the real UDP listener path, plus process-isolated relay crash, controlled
mid-route packet-drop recovery, route-binding expiry rejection, and malformed
route-binding rejection without relay process failure.

### LOW — Discovery address is not cryptographically bound
The signed discovery response now includes a fresh per-scan challenge nonce, so an old authenticated `CYBOS_PEER` response cannot be replayed into a new scan. The observed response source address is still treated as a locator rather than a cryptographic identity binding. The subsequent signed handshake remains the authoritative identity check.

Recommended next step: optionally advertise an authenticated listen endpoint/capability record for deployments where discovery needs stronger address semantics.

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
- routed fallback after an unavailable relay
- process-isolated recovery after relay crash during packet forwarding
- process-isolated recovery after relay crash during route binding
- process-isolated recovery after a controlled mid-route packet drop
- process-isolated rejection of malformed route bindings without relay crash
- process-isolated rejection of expired route bindings
- idempotent duplicate-delivery ACK recovery


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
- live full-path source → relay-a → relay-b → destination delivery with reverse ACK;
- standalone headless onion smoke test executed from the debug cybOS binary;
- live routed delivery, replay and ciphertext-tampering tests through the actual UDP listener path;
- forged encrypted route-ack rejection;
- regression test proving route-bind payload does not expose source node ID/public key.

Completed:
- automatic anonymous per-hop X25519 session establishment;
- encrypted route binding;
- reverse routed signed destination ACK delivery;
- bounded fallback through shorter selected relay paths;
- idempotent duplicate-delivery ACK recovery with ratchet resynchronization;
- explicit route selection and ordered relay hops in CybChat UI.

Still to harden:
- route-expiry refresh/renewal semantics for long-lived sessions;
- broader traffic-analysis and endpoint privacy protections.
