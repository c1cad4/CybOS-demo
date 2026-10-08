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

The routed CybChat path uses a real UDP onion transport with a separate anonymous
X25519 session for each relay hop. Before a relay allocates that session state,
the initiator must answer a signed stateless cookie challenge bound to its UDP
endpoint and ephemeral key; this prevents unauthenticated UDP traffic from
consuming relay session slots.

The persistent node identity remains Ed25519. It is used to authenticate a relay's
ephemeral onion session reply, but the source node identity is deliberately not
included in the onion session-init message or the route-binding payload sent to
that relay.

For each relay hop:

1. The source generates a fresh random onion session ID and an ephemeral X25519 key.
2. The relay authenticates its signed session reply with its known Ed25519 public key.
3. Both sides derive a per-hop session key with X25519 + HKDF-SHA256.
4. The source sends the route binding encrypted with that hop key. The bind contains only route ID, hop index, expiry, the previous/next transport addresses and the next node ID needed by that relay.
5. The relay stores the route binding against the anonymous session ID and accepts data packets only from the same source endpoint observed during the session setup.
6. The actual CybChat end-to-end payload remains inside the existing authenticated destination envelope; each relay only peels its own ChaCha20-Poly1305 onion layer.
7. Each nested layer carries its own hop session ID, route ID, packet ID and hop index, so a relay cannot reuse one hop's key as another hop's layer key.
8. Reverse delivery acknowledgements travel back through the established route. The destination's signed ACK is verified end-to-end by the source.
9. A routed send keeps one message ID across recovery attempts. If the selected route cannot establish or complete delivery, cybOS can retry through shorter variants of the selected relay list rather than inventing unrelated relays.
10. The destination keeps a bounded cache of signed ACKs for recently delivered message IDs. A valid duplicate envelope can receive the cached ACK without delivering the chat payload again; when a fresh destination session is used for recovery, the destination advances that session's ratchet counter to keep the sender and receiver synchronized.

The onion packet therefore exposes only the metadata necessary for the current hop.
A relay does not receive the source Ed25519 public key as part of route setup, and
it does not decrypt the end-to-end CybChat payload.

The relay listener is a real UDP forwarder. Live loopback tests exercise multi-hop
forwarding, replay rejection, ciphertext tampering, route-binding authentication and
routed ACK delivery. The debug binary also exposes a standalone headless onion
smoke test that exercises the same live listener path.

This is routed encrypted transport, not a complete anonymity network. Network
endpoints, timing, packet size and route participation remain visible to relevant
network observers. The design also does not introduce a persistent X25519 private
relay identity; onion hop material is ephemeral and scoped to the route/session.

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

The current harness also exercises a full live source → relay-a → relay-b →
destination route with reverse ACK delivery, route rebuild after an unavailable
relay, and duplicate-delivery ACK recovery, plus a standalone headless onion
smoke test. Process-isolated active-relay crash, controlled mid-route packet
drop, route-bind crash/expiry validation, and malformed packet fault injection
are also exercised. Onion session admission now uses a signed stateless cookie
challenge so the relay does not allocate session state until the initiator proves
control of the same UDP endpoint.

## 12. Operational status

The direct LAN channel is real transport, not a UI simulation.

The UI should report actual states: authenticated peer, key exchange, encrypted send, acknowledged delivery, timeout, authentication failure, and pinned-key conflict.

It must never claim E2E delivery before the cryptographic ACK is verified.


## 12. Headless onion smoke test

The debug binary includes a standalone live onion-path smoke test. It creates one
source node, two relay nodes and one destination in the same process, establishes
anonymous per-hop sessions, binds the route, sends an end-to-end CybChat envelope,
and verifies the reverse signed ACK and destination delivery.

Run it with:

```bash
CYBOS_HEADLESS_ONION_TEST=1 cargo run
```

A successful run prints:

```text
ONION_TEST OK · 2 relays · <source-id> → <destination-id>
```

The GitHub CI runs the same smoke test against the debug `cybos` binary on Linux
and macOS.
