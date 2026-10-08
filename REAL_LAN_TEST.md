# cybOS Real-LAN Onion Test

This is the manual field test for the current cybOS routed CybChat implementation.

## Goal
Run 2–3 real macOS cybOS nodes on the same LAN and verify authenticated LAN discovery, explicit peer-key trust, destination selection, relay selection, SEND ONION end-to-end delivery, and signed destination ACK verification.

## Topology
Minimum 3-node test: Mac A → Mac B → Mac C.
Mac A = sender; Mac B = relay; Mac C = destination.
4-node test: Mac A → Mac B → Mac C → Mac D, with B and C as relays and D as destination.

## Before starting
All Macs must be on the same IPv4 LAN and able to receive UDP traffic.
Allow incoming connections for cybOS if the macOS firewall asks.
Do not use a VPN interface for the first test.
Use the same cybOS build on every Mac.

## Obtain the macOS test build
The GitHub Actions macOS job publishes cybOS-macos-test and cybOS-macos-qa artifacts.
For the real GUI field test use cybOS-macos-test.
Unzip the artifact and launch cybOS.app.
The CI build is ad-hoc signed for testing, not Apple-notarized, so a Gatekeeper warning can be expected on first launch.

## Node A — sender
1. Open CybChat.
2. Wait for LAN peers to appear.
3. Inspect the SHA-256 fingerprint for every participating node.
4. Click TRUST KEY only after confirming the displayed fingerprint.
5. Select Mac C as TARGET.
6. Select Mac B as ADD under ONION ROUTE, producing H1.
7. Enter a unique message such as: ONION FIELD TEST A→B→C <timestamp>.
8. Click SEND ONION · 1.
Expected sender state: ONION ROUTE · 1 RELAY(S) → destination, then E2E DELIVERED.

## Node B — relay
Mac B does not need to send a message.
Verify that Mac B remains running and trusted and does not receive the end-to-end plaintext as a local CybChat message.

## Node C — destination
Mac C should receive the test message in CybChat.
The destination accepts it only after E2E WireEnvelope authentication and decryption succeed.

## 4-node test
On Mac A: TARGET = Mac D; H1 = Mac B; H2 = Mac C; click SEND ONION · 2.
Expected: E2E DELIVERED on Mac A and plaintext only at Mac D.

## Route recovery test
Start with A → B → C. Make B unavailable after discovery/trust.
On Mac A keep C as destination and another trusted peer available where possible. Use AUTO to rebuild a healthy relay selection and send the test again.
The sender must never claim E2E DELIVERED without a verified destination ACK.

## What to record
For each Mac record macOS version, cybOS version, node ID, first 24 fingerprint characters, LAN IPv4 address, discovery result, trust result, relay order, exact sender status, and whether the destination received the message.
Do not post private identity keys.

## Failure diagnosis
No peers: verify same LAN, UDP/39393 allowed through macOS firewall, no VPN interception, and cybOS running on every node.
Trust failure: compare the full fingerprint out-of-band. Do not accept an unexpected key replacement.
Direct chat works but onion fails: verify trusted destination, at least one trusted relay, reachable relay/destination addresses, and relay uptime.
Sender says delivered but destination shows nothing: preserve sender node ID, destination node ID, relay node IDs, message ID, and exact delivery status for debugging.