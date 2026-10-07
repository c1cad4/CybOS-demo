# cybOS Secure CybChat Security Audit

Date: 2026-10-07
Scope: direct-LAN discovery, identity, handshake, wire envelope, ratchet, replay handling, ACK delivery, local storage, and documentation.

## Audit result

The direct LAN design has a sound primitive selection and an explicit security boundary, but the initial implementation contained two merge-blocking state/integration defects:

1. The runtime persisted a random node ID unrelated to the persistent Ed25519 identity. This could make authenticated discovery reject the local peer because the announced node ID did not equal the hash-derived identity ID.
2. A lost delivery ACK could leave the sender reusing a stale ratchet session while the receiver had already advanced its chain. Subsequent messages could then be rejected until the process was restarted.

Both defects were fixed on this branch.

## Fixed during audit

- Node ID is now always derived from the persistent Ed25519 public key.
- Sender-side ratchet session is dropped after an ACK timeout so the next send performs a fresh X25519 handshake.
- Receiver only advances its chain after successful decryption and successful next-chain derivation.
- Inbound session state is bounded to 128 concurrent peer sessions.
- Local Unix runtime directory is hardened to mode 0700 and the SQLite database to mode 0600 when permissions can be changed.
- README and CybChat UI no longer claim that wire-level E2E is disabled.
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
- Explicit non-claim of full multi-hop onion routing.

## Residual risks / next hardening

### MEDIUM — First contact still depends on operator trust
A new peer is now shown with its SHA-256 public-key fingerprint and requires explicit "TRUST KEY" confirmation. This makes first-contact trust visible, but it is still TOFU: the operator should compare the fingerprint through an independent channel when identity authenticity matters.

Recommended next step: support out-of-band key provisioning for deployments that need stronger initial trust.

### MEDIUM — Local data is not encrypted at rest
The Ed25519 private key and chat history are stored in the local SQLite database. Unix file permissions are hardened, but the implementation does not use the macOS Keychain or database-level encryption.

Recommended next step: move the private identity key to an OS-backed secure key store and separately define whether chat history should be encrypted at rest.

### MEDIUM — Adversarial integration coverage is partial
A deterministic two-node loopback UDP harness now exercises real handshake, signed delivery ACKs, sequential ratchet messages, replay injection, ciphertext tampering, and lost-ACK recovery through a fresh handshake. OS-process isolation and several remaining cases are still not covered.

Recommended next step: add an OS-process harness and cover stale timestamps, wrong recipient, TOFU replacement, forged ACKs, and lost-ACK recovery under controlled fault injection.

### LOW — Discovery address is not cryptographically bound
The signed discovery response authenticates node ID, version and public key, but not the observed source address. A valid discovery response can therefore be replayed from a different address. The subsequent signed handshake prevents identity impersonation, but stale/misrouted discovery can still cause connection failure or denial of service.

Recommended next step: include a challenge/nonce and observed endpoint in the discovery exchange, or treat discovery only as a locator and require the authenticated handshake before showing the peer as fully trusted.

### LOW — Traffic metadata remains visible
The direct-LAN channel still exposes packet timing, approximate size, node IDs, message IDs, counters and network endpoints to a passive LAN observer. Encryption protects message content, not traffic analysis.

### LOW — Ratchet is ordered and symmetric, not a Double Ratchet
The current design is a per-message HKDF chain with a monotonic counter. It is not the Signal Double Ratchet and does not provide asynchronous out-of-order message handling or a DH ratchet after every message.

## Merge gate

Do not claim "production-grade secure messenger" yet.

For this direct-LAN MVP, the implementation is suitable to merge after the current CI is green, with the residual risks above kept explicit in the documentation. The next security milestone should be the two-process adversarial integration harness, followed by explicit first-use fingerprint confirmation and secure at-rest key storage.


Current automated integration coverage also includes:
- live UDP rejection of stale envelopes
- live UDP rejection of wrong-recipient envelopes
- forged ACK signature rejection
- persistent TOFU key replacement rejection
