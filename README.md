# cybOS 0.7 — CicadaFarm · RobotCYB · CybChat

This repository contains both the browser prototype and the native macOS-first Rust/egui application. The browser layer is the visual prototype; the native layer is the real desktop runtime.

The native implementation lives under `src/`, with the macOS launcher and BLE helper under `START_cybOS.command` and `macos/`.

For the native desktop installation path, see [NATIVE_INSTALL.md](NATIVE_INSTALL.md).

## What is in this repository

### Browser prototype

- Cyberpunk neon-green cybOS interface
- RADAR / contact visualization
- CYBCORE energy visualization and throttle interaction
- SIGNAL GRID with moving data packets
- CYBCHAT visual conversation layer
- CYBERGRAPH / knowledge graph visualization
- Hologram panel
- Interactive terminal
- WARP / SPHERE / ORACLE / SENATE controls
- Runtime status inspector

### Native desktop

- Rust/egui macOS-first application
- Local SQLite persistence
- RobotCYB local Qwen runtime
- CYB RADAR LAN + macOS BLE proximity discovery
- Noise XX secure CYBChat with delivery ACK
- TOFU peer identity persistence
- Cybergraph secure-peer relationships

See [NATIVE_INSTALL.md](NATIVE_INSTALL.md) for build and packaging instructions.

## Browser runtime architecture

runtime.js introduces the first explicit cell contract for the browser prototype.

Each logical cell declares:
- inputs — what the cell is allowed to consume
- outputs — what it produces
- status — BOOT, RUNNING, ONLINE, DEGRADED, ERROR, OFFLINE
- heartbeat — freshness of the cell's liveness signal
- budget — cooperative execution budget in milliseconds
- run / overrun / error counters

Current cells:

| Cell | Inputs | Outputs | Budget |
|---|---|---|---:|
| SYSTEM | clock, keyboard, pointer | global state, events | 4 ms |
| RADAR | system state, scan command | contacts, scan history | 4 ms |
| CYBCORE | throttle, warp, power | energy, resonance, status | 5 ms |
| SIGNAL_GRID | core state, throttle | packets, activity | 5 ms |
| CYBCHAT | messages, local events | conversation state | 5 ms |
| CYBERGRAPH | entities, relations, events | nodes, connections, signals | 6 ms |
| HOLOGRAM | system state, target | projection | 4 ms |
| TERMINAL | commands | events, state changes | 8 ms |

### Bounded execution

A browser JavaScript function cannot be safely hard-killed in the middle of synchronous execution. Therefore the demo uses a cooperative bound:

1. execution is measured;
2. overruns are recorded;
3. the cell becomes DEGRADED when it exceeds its budget;
4. errors transition the cell to ERROR;
5. heartbeat freshness detects stalled cells.

runtime-bridge.js currently provides a frame-level heartbeat for the legacy p5 renderer. The next architectural step is to move each real cell into an isolated task/worker boundary so its budget becomes enforceable rather than observational.

## Runtime inspector

Click the RUNTIME indicator in the lower-right corner or press R.

The inspector exposes:
- cell state
- heartbeat freshness
- execution duration vs budget
- run count
- overrun count
- error count
- declared inputs and outputs

This is deliberately visible: cybOS should never claim that a subsystem is connected or alive when the runtime has not observed it.

## Browser prototype limitations

The browser layer is still a prototype, not the final cybOS native runtime.

- The p5 renderer remains a legacy monolithic frame.
- The runtime currently observes that frame through runtime-bridge.js.
- P2P, Bluetooth, Nostr, cameras and external network services are not represented as connected unless a real implementation is added.
- There is no cryptographic private-messaging layer in this demo.
- Persistent graph/database state is not part of this browser repository.
- The terminal is a local simulation and does not execute operating-system commands.

## Direction

The intended architecture is:

CicadaFarm = physical world
RobotCYB = mind
CybChat = communication
Cybergraph = memory/relations
cybOS = connection + runtime

The long-term target is a local-first cybernetic operating environment where components communicate through explicit inputs/outputs and bounded execution rather than directly calling arbitrary subsystems.

## Run

Open index.html in a modern browser.

Because the demo loads p5.js from jsDelivr, the first load requires network access. The application itself does not claim external connectivity merely because the renderer loaded.

## Repository structure

CybOS-demo/
├── index.html
├── sketch.js
├── runtime.js
├── runtime-bridge.js
├── style.css
└── README.md

## License

See the repository license configuration.

## Runtime stack status

The current native stack is organized as bounded local-first cells:

- **CYB RADAR:** opt-in LAN discovery + macOS BLE RSSI discovery; no GPS and no fabricated meter distances.
- **Unified proximity:** LAN and BLE observations are correlated into one peer projection.
- **CYBChat:** direct TCP transport protected by Noise XX with encrypted frames and delivery ACKs.
- **Identity:** the first authenticated Noise public key for a peer is persisted as TOFU; a changed key is rejected.
- **Cybergraph:** secure peers and secure-chat relationships are persisted as graph nodes/links.
- **Bounded execution:** discovery and secure transport use explicit time windows, I/O deadlines and connection limits.

A peer is not considered geographically located merely because BLE RSSI is available.
