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

## 3. Run the full routed acceptance check

Use one command to execute both the live multi-hop smoke test and the process-isolated relay-crash recovery test:

```bash
cargo run -- --self-test all
```

Expected output includes both:

```text
ONION_TEST OK · 2 relays · <source-id> → <destination-id>
ONION_PROCESS_TEST OK · relay crash recovered · <source-id> → <destination-id>
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


## Process-isolated onion recovery

The debug binary can also run an OS-process-level routed recovery test:

```bash
CYBOS_HEADLESS_ONION_PROCESS_TEST=1 target/debug/cybos
```

The harness launches separate cybOS processes for three relays and a destination.
One relay intentionally exits when the first onion packet arrives. The source then
uses the existing route fallback logic and must complete delivery with a signed
destination acknowledgement. A successful run prints:

```
ONION_PROCESS_TEST OK · relay crash recovered · <source-id> → <destination-id>
```

This covers a real process boundary rather than only in-process UDP threads.
