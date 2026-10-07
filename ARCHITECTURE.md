# cybOS architecture

cybOS 0.7 is a native desktop system composed of a small application shell,
domain services, local persistence and page-oriented UI. Its communication and
runtime design also adopts selected protocol principles from the Cyberia CybOS
concept without pretending the macOS app is a new kernel.

## Core layers

- Application shell — src/shell.rs owns the eframe application trait only.
- Shell UI composition — src/ui/shell.rs owns the native window layout,
  navigation rails, header, central page router and bottom status bus.
- State — src/state.rs owns the central runtime state and initialization.
- Persistence — src/store.rs owns SQLite events, memories and graph data.
- Runtime — src/runtime/ owns node bootstrap and persistent runtime setup.
- Configuration — src/config.rs centralizes version, public identifiers and default local settings.
- Brain — src/brain/ owns context, learning, planning, tools, web intent
  and the shared local Qwen HTTP runtime.
- Network — src/network/ owns web discovery, fetch, parsing and source
  handling.
- Assets — src/assets/ owns Solana balances and market data; network refreshes run in background workers.
- LAN — src/network/lan.rs provides broadcast-only discovery plus directed peer-to-peer chat.
- UI pages — src/ui/ contains dashboard, graph, brain, farm, robot,
  chat, network, cameras, assets and system views.
- CybChat history is persisted in SQLite and restored at startup.
- LAN chat uses an explicit envelope (`from`, `to`, `message_id`, `message`, `version`) and a bounded delivery ACK; this is a delivery acknowledgement, not a cryptographic proof.
- Theme/navigation — src/theme.rs and src/navigation.rs own visual
  primitives and page semantics.

## Local-first contract

The application does not pretend that unavailable infrastructure is live.
BLE, P2P, Nostr and camera feeds are represented as ready/adaptor states until a
real transport is connected. LAN node discovery is a real UDP broadcast service;
it does not claim application-level chat connectivity. LAN chat is explicitly user-triggered and unencrypted.

SQLite persists the local node identity, events, memories and graph state.

## Qwen runtime contract

All local Qwen chat requests use the shared runtime in
src/brain/qwen_runtime.rs.

The runtime owns:

- local address and chat endpoint;
- model identifier;
- JSON chat request construction;
- temperature/max-token handling;
- HTTP transport;
- JSON decoding.

Callers such as the planner, learning flow, RobotCYB answer path and web-source
answer path do not implement their own Qwen HTTP transport.

## Refactoring boundary

The root application shell is intentionally thin. Page rendering lives under
src/ui/, while domain-specific computation remains in the relevant service
modules. Empty placeholder domain module shells were removed once the page
layers became the active implementation boundary.
