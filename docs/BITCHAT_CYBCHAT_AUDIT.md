# BitChat → CybChat engineering audit

Audit reference: [permissionlesstech/bitchat](https://github.com/permissionlesstech/bitchat), especially its Noise session implementation, security constants, protocol tests, and protocol whitepaper.

## Scope and compatibility

BitChat is a Swift application with iOS/macOS platform services, Bluetooth LE mesh transport, Nostr relay transport, Keychain identity storage, and a custom message-routing stack. cybOS is a Rust desktop application with a different UI/runtime and currently uses the Rust `snow` implementation of Noise XX over bounded TCP sessions. Copying BitChat's Swift files into cybOS would not produce a compatible or buildable implementation. Port protocol concepts and tests, not platform-specific code.

## Immediate fix in cybOS

- Peer fingerprints were previously generated with a 64-bit FNV-style non-cryptographic hash. That is not suitable for a security identity displayed for manual comparison.
- The fingerprint is now SHA-256 over the Noise static public key, rendered as 64 hexadecimal characters.
- A known SHA-256 test vector for `abc` and distinct-key checks prevent accidental regression.

## Existing protections to retain

- Noise XX with ChaChaPoly and BLAKE2s, through the established `snow` crate.
- Persistent local static key; refuse malformed identity material and fail if identity cannot be durably stored.
- Exact peer-key pin persistence; reject malformed pins, storage errors, and changed keys.
- Bounded frame lengths, session deadlines, atomic concurrent-session admission, message-ID replay rejection, and explicit delivery acknowledgements.
- CI on Linux and macOS, including universal macOS target builds.

## Gaps to close before calling CybChat production-ready

1. **First-contact authentication:** first-seen keys are still accepted through TOFU. This detects a changed key on later connections but does not prevent a first-contact man-in-the-middle. Add a pending-trust UI with full fingerprint, explicit user approval, and QR/out-of-band verification; do not deliver messages or send ACK until approved.
2. **Handshake/session lifecycle:** add tests for malformed and truncated handshake frames, timeout at every handshake step, concurrent/crossed attempts, reconnects, and resource release after failure.
3. **Cryptographic conformance:** add the official `snow` Noise XX test vectors and assert the exact protocol name, handshake transcript, and transport encryption/decryption. Do not claim wire compatibility with BitChat: its protocol details and transport envelope differ.
4. **Rate limiting:** apply per-peer and global handshake/message rate limits in addition to the existing concurrent-session cap.
5. **Identity binding:** validate node-ID length/format and ensure the claimed sender ID, destination ID, and pinned static key are consistently bound to the session and envelope.
6. **Replay and delivery semantics:** test duplicate IDs, wrong recipient, forged ACK/reject payloads, malformed JSON, and process restart behavior. Distinguish transport acceptance from user-visible delivery.
7. **Secret storage:** use the OS key store/keychain where feasible; the current SQLite-backed private-key persistence needs a documented local-at-rest threat model.
8. **Operational status:** surface listener bind/startup errors rather than reporting a ready state when the port could not be bound.
9. **Transport roadmap:** only after direct encrypted chat is reliable, consider BLE mesh and optional Nostr transport as separate adapters behind a common message-router interface.

## Release gate

Do not describe CybChat as fully authenticated or production-secure until first-contact verification, protocol-vector tests, lifecycle/fuzz tests, and a real macOS app smoke test pass. Noise encryption protects message contents in transit; it does not by itself authenticate a human identity, conceal metadata, or guarantee delivery.
