# cybOS Secure CybChat Guide

## 1. Security model

cybOS uses a local persistent Ed25519 node identity.

A node ID is derived from SHA-256(public_key):

cyb-<first-12-bytes-of-sha256(public-key)>

The private Ed25519 key remains local to the node. On macOS it is stored in the
system Keychain as a generic password item and is not persisted in the SQLite
runtime store. Existing SQLite-backed identities are migrated to Keychain on
first launch and then removed from SQLite. Non-macOS builds retain the existing
local SQLite identity storage until a platform-specific secure keystore is added.

Peer discovery is authenticated. A peer announces its node ID, application version, Ed25519 public key and a signature binding those values.

The receiver verifies that the public key derives the announced node ID and verifies the signature.

## 2. Peer key exchange

Direct CybChat uses an authenticated X25519 handshake.

1. Initiator creates an ephemeral X25519 key.
2. Initiator signs the key-init transcript with its persistent Ed25519 identity.
3. Responder verifies the identity and creates its own ephemeral X25519 key.
4. Responder signs the key-reply transcript.
5. Both sides derive the same session root with HKDF-SHA256.
6. Ephemeral X25519 private keys are consumed by the agreement operation.

The protocol separates identity authentication (Ed25519), key agreement (X25519), key derivation (HKDF-SHA256), and message confidentiality/integrity (ChaCha20-Poly1305).

## 3. Persistent peer-key pinning

After authenticated discovery, cybOS uses explicit TOFU (Trust On First Use).

A newly discovered peer is shown with its SHA-256 public-key fingerprint and is
not eligible for direct send until the operator explicitly selects "TRUST KEY".

After confirmation, the key is stored as peer_pin:<node_id> = public_key in the
local SQLite key/value store.

On subsequent discovery:
- same node ID + same public key = trusted automatically
- same node ID + different public key = rejected and surfaced as a pinned-key conflict

This prevents silent identity-key replacement and makes the first-contact decision visible.

## 4. Encrypted wire envelope

A CybChat message is never sent as plaintext.

The wire envelope contains:
- message ID
- sender node ID
- recipient node ID
- timestamp
- ratchet counter
- AEAD nonce
- ciphertext
- Ed25519 signature

The signed transcript covers the encrypted material and routing identity. AEAD associated data covers message identity, endpoints, timestamp and counter.

## 5. Message-key ratchet

After the initial X25519 handshake, cybOS derives a message key from the current chain key and monotonically increasing counter.

message_key_n = HKDF(chain_key_n, "message-key:n")

After successful delivery:

chain_key_(n+1) = HKDF(chain_key_n, "chain-key:n")

The receiver requires the expected next counter and advances its chain only after successful authentication and decryption.

The current UDP design is ordered. If an ACK is lost after the receiver
successfully accepts a message, the sender drops its cached session and the next
send performs a fresh X25519 handshake instead of continuing a stale ratchet.
Packets that arrive out of order are rejected rather than silently skipped.

## 6. Replay protection

Each message has a unique UUID message ID, timestamp freshness window, monotonic session counter and an in-memory seen-message cache.

An already accepted message ID is not delivered again.

The timestamp window is approximately five minutes.

## 7. Authenticated delivery

The recipient sends an Ed25519-signed ACK after successful decryption.

The sender reports E2E DELIVERED only after verifying the message ID, identities and ACK signature.

## 8. Direct-LAN flow

LAN discovery
 -> Ed25519 identity verification
 -> TOFU public-key pinning
 -> authenticated X25519 handshake
 -> HKDF-SHA256 session root
 -> per-message HKDF ratchet
 -> ChaCha20-Poly1305
 -> signed encrypted wire envelope
 -> decrypt + authenticate + replay check
 -> signed ACK

## 9. Multi-hop onion transport

The branch now contains the first real onion-routing transport layer.

The design keeps the persistent node identity on Ed25519. Each adjacent relay
link is represented by an authenticated X25519-derived session root, and the
onion layer derives a separate ChaCha20-Poly1305 key from that root using a
route ID, packet ID, hop index and direction context.

For each packet:

1. The source creates a unique route ID and packet ID.
2. The end-to-end payload is wrapped in one encrypted layer per relay.
3. The outer layer exposes only the current route metadata and the current hop index.
4. After decryption, a relay learns only its immediate next node/address and an opaque inner packet.
5. The next packet remains encrypted under the next relay's distinct layer key.
6. Each relay enforces route ID, hop index, packet lineage and a bounded expiration window.
7. A per-relay replay cache rejects reuse of the same route/packet/hop tuple.
8. Unknown route bindings are rejected by the UDP relay listener.

The relay listener is a real UDP forwarder. A live three-relay loopback test
now exercises relay-a → relay-b → relay-c → destination forwarding.

The current milestone intentionally does not claim the full routed CybChat feature
set yet. Automatic route establishment between arbitrary peers, reverse onion
ACK delivery, and UI route selection still need to be connected to the existing
authenticated CybChat session layer.

The current relay-layer choice also avoids introducing a persistent X25519 private
identity. Ed25519 remains the long-term node identity; X25519 session material is
ephemeral and the onion layer derives fresh per-route/per-packet keys from it.

## 10. Cryptographic boundaries

| Purpose | Primitive |
|---|---|
| Persistent identity | Ed25519 |
| Node ID | SHA-256 |
| Handshake | X25519 ephemeral |
| KDF | HKDF-SHA256 |
| Message encryption | ChaCha20-Poly1305 |
| Authentication | Ed25519 signatures |
| Replay | UUID + timestamp + counter |

The implementation uses ring for the existing Ed25519, X25519, HKDF and AEAD primitives.

## 11. Security test matrix

Automated unit coverage currently includes:
- X25519 agreement symmetry
- HKDF session-key symmetry
- AEAD round trip
- ciphertext does not contain plaintext
- Ed25519 signature verification
- signature tampering failure
- ratchet key/counter derivation

The live loopback harness currently exercises:
- ciphertext tampering failure on the live wire
- stale timestamp rejection
- duplicate/replay rejection
- ratchet counter ordering
- TOFU key replacement rejection
- forged ACK signature rejection
- wrong recipient rejection
- lost-ACK recovery through a fresh handshake

The production-hardening harness now also launches three real cybOS child processes
for relay-a, relay-b and destination, performs a routed E2E delivery through those
process boundaries, and injects replay and ciphertext-tampering packets against the
first relay.

## 12. Operational status

The direct LAN channel is real transport, not a UI simulation.

The UI should report actual states: authenticated peer, key exchange, encrypted send, acknowledged delivery, timeout, authentication failure, and pinned-key conflict.

It must never claim E2E delivery before the cryptographic ACK is verified.
