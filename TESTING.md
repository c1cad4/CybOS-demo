# cybOS Testing

## 1. Start the native app

On macOS:

```bash
./START_cybOS.command
```

This builds the release binary when needed, creates `cybOS.app`, and launches it.

## 2. Test the real onion transport

Build and run the debug self-test:

```bash
cargo run -- --self-test onion
```

The test creates four ephemeral nodes:

`source → relay A → relay B → destination`

It exercises:

- per-relay X25519 session establishment;
- encrypted route binding;
- layered ChaCha20-Poly1305 onion packets;
- live UDP forwarding;
- end-to-end CybChat WireEnvelope delivery;
- signed reverse ACK;
- relay isolation from the destination chat event.

Successful output is:

```text
ONION_TEST OK · 2 relays · cyb-... → cyb-...
```

## 3. Run the complete Rust suite

```bash
cargo test --locked -- --test-threads=1
```

## 4. Test the GUI path

Discover peers in CybChat, inspect the SHA-256 fingerprint, press `TRUST KEY`, select a destination, select one or more trusted relays, and use `SEND ONION`.

The UI must report a verified delivery rather than only a local send event.

## 5. What this test does not prove

The onion path is routed encrypted transport, not a complete anonymity network. Traffic metadata, timing, packet sizes, participating endpoints, and local process state are still observable to relevant observers.

Bluetooth, P2P, Nostr, and RTSP are not claimed as connected transports in this MVP.
