# cybOS Mac Product Acceptance Test

This is the practical manual test for the native macOS product.

## A. Launch the real product app

On the Mac:

```bash
chmod +x START_cybOS.command
./START_cybOS.command
```

The script builds the normal release binary when needed, creates `cybOS.app`,
ad-hoc signs it locally and launches it.

Use the normal `cybOS.app` for product behavior. Use `cybOS-QA.app` only for
transport diagnostics.

## B. Direct LAN discovery

Use two Macs on the same LAN.

1. Launch `cybOS.app` on Mac A.
2. Launch `cybOS.app` on Mac B.
3. Open **CybChat** on both.
4. Run **SCAN LAN**.
5. Each Mac should discover the other node with:
   - node ID
   - LAN address
   - cybOS version
   - SHA-256 fingerprint
   - `NEW KEY` until trusted

Pass condition:

`authenticated discovery` is shown; no peer is invented by the UI.

## C. OOB fingerprint provisioning

From Mac A:

```bash
./cybOS.app/Contents/MacOS/cybOS --identity
```

Record:

```text
NODE_ID ...
FINGERPRINT ...
PUBLIC_KEY ...
```

Transfer the Node ID and fingerprint through an independent channel.

On Mac B:

```bash
./cybOS.app/Contents/MacOS/cybOS --provision-peer NODE_ID FINGERPRINT
```

Restart/scan the LAN from the UI.

The matching peer should become `TRUSTED` automatically.

Negative test: provision a deliberately wrong fingerprint and confirm the peer
stays untrusted / pinned-key conflict is surfaced.

## D. Direct encrypted Chat

1. Select the trusted peer as the direct target.
2. Send a short message.
3. Confirm the sender shows delivery only after ACK verification.
4. Confirm the receiver displays the message.
5. Send a second message.

Pass condition:

`DIRECT LAN` delivery is acknowledged and chat history persists after restart.

## E. Two-hop Onion on one Mac

The normal product app does not need to expose test-only relay processes.

Run:

```bash
chmod +x QA_cybOS.command
./QA_cybOS.command
```

The QA launcher builds an isolated release binary with the `qa` feature, creates
`cybOS-QA.app`, and runs:

```text
--self-test onion
--self-test onion-process
```

The first test creates:

```text
source → relay A → relay B → destination
```

and verifies:

- two real UDP relay hops
- per-hop X25519 sessions
- encrypted route binding
- layered onion packets
- destination CybChat delivery
- signed reverse ACK
- relays do not receive the destination chat event

Expected:

```text
ONION_TEST OK · 2 relays · ...
```

## F. Relay failure recovery

The same QA launcher executes the process-isolated failure test.

The test intentionally terminates one relay during the route attempt and verifies
that cybOS retries through the available route variant and still delivers the
message.

Expected:

```text
ONION_PROCESS_TEST OK · relay crash recovered · ...
```

## G. Real 2-hop GUI route

For an actual UI-driven two-hop route, use four distinct LAN nodes:

```text
source GUI → relay A → relay B → destination GUI
```

Each relay must be a trusted LAN peer.

On the source CybChat page:

1. Scan LAN.
2. Establish trust/OOB for all three other nodes.
3. Select the destination.
4. Add relay A as H1.
5. Add relay B as H2.
6. Send **SEND ONION · 2**.

The UI should show:

```text
H1 relay-A → H2 relay-B → DESTINATION
```

and the delivery status must become acknowledged only after the destination's
signed ACK returns through the route.

## Acceptance result

A Mac build is considered product-test-ready when:

- the normal `cybOS.app` launches successfully;
- LAN discovery works between two real Macs;
- OOB provisioning pins the expected fingerprint;
- direct encrypted Chat delivers and acknowledges;
- `cybOS-QA.app` passes the two-hop onion test;
- `cybOS-QA.app` passes relay-failure recovery;
- a four-node LAN can complete a real UI-driven 2-hop route.
