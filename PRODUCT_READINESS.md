# cybOS product readiness

The native macOS desktop core is at technical-beta readiness.

## Technical beta — READY

- Native Rust/egui desktop shell.
- SQLite persistence and node identity.
- RobotCYB with asynchronous bounded execution and a 90-second end-to-end deadline.
- Local Qwen runtime with internal HTTP timeout.
- CYB RADAR with opt-in LAN discovery and macOS BLE RSSI discovery.
- Unified LAN/BLE peer projection.
- Secure CYBChat over Noise XX with encrypted frames and delivery ACK.
- First-seen peer identity persisted as TOFU; changed identity is rejected.
- Secure peer relations persisted in Cybergraph.
- Stoppable long-lived LAN and secure listeners.
- Bounded HTTP workers for Qwen, web, Solana balances and market data.
- macOS app packaging path and GitHub release workflow.

## Public macOS beta — about 80–85%

The remaining work is mostly productization:
- signed/notarized distribution;
- first-run onboarding and permissions guidance;
- real two-Mac interoperability testing for RADAR + Secure CYBChat + TOFU;
- upgrade/reinstall and recovery tests.

## Production 1.0 — about 65–70%

Additional hardening remains around security review, cross-version interoperability,
diagnostics/recovery, configuration migration, signed updates and operational
release/support workflows.

Physical RobotCYB hardware, farm sensors/cameras and Cyblex are separate future
layers and should not block the first desktop beta.
