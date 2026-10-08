# cybOS 0.7.1

Core release for the native local-first cybOS runtime.

## Runtime hardening

- explicit runtime cell contracts expose inputs, outputs, status, heartbeat and execution budgets;
- RobotCYB has one 90-second end-to-end deadline across planner steps and local model calls;
- web search/fetch, learning, LAN discovery, secure chat and Assets refreshes use bounded worker lifecycles;
- long-lived LAN and secure listeners own cooperative stop handles and terminate on application shutdown.

## Communication layer

- LAN discovery is broadcast-only and opt-in visibility is persisted locally;
- directed LAN chat uses message identity and bounded delivery acknowledgement;
- secure CYBChat uses Noise XX with encrypted transport frames and delivery ACK;
- first-seen peer identity keys are persisted locally as TOFU;
- changed peer identity keys are rejected at the application trust boundary;
- secure peers and relationships are persisted into Cybergraph;
- macOS BLE advertising is opt-in and exposes only a short node identifier;
- BLE proximity uses measured RSSI and never fabricates GPS coordinates or meter distances;
- LAN and BLE observations are correlated into one unified proximity projection.

## Native desktop

- Rust/egui macOS-first application;
- persistent SQLite chat, event, memory and graph state;
- local Qwen integration at `127.0.0.1:8080`;
- background Solana balance and GeckoTerminal market refresh;
- macOS .app packaging and CoreBluetooth advertiser compilation;
- Linux and macOS CI with release-build verification.

## Current release status

The repository is suitable for a technical/private beta and has a published v0.7.1 macOS ZIP through `c1cad4/cybOS`.

A normal public macOS installation still needs:
- Developer ID signing and Apple notarization for a smooth Gatekeeper install path;
- two-Mac interoperability and upgrade/recovery validation;
- first-run Qwen onboarding remains local-model dependent.

Hardware/camera integrations and P2P/Nostr adapters remain outside the current desktop beta.

## Post-release hardening on main

After publication of v0.7.1, main gained an additional secure-message replay cache retaining the latest 1024 message IDs. This hardening is intentionally not retroactively described as part of the already-published v0.7.1 binary.
