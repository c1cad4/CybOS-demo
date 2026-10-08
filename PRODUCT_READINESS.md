# cybOS product readiness

## Technical beta
READY for technical macOS users.

Core desktop scope currently implemented:
- native Rust/egui shell and SQLite persistence;
- asynchronous RobotCYB worker with one 90-second end-to-end deadline;
- local Qwen runtime with internal HTTP timeout;
- opt-in LAN + macOS BLE CYB RADAR and unified proximity;
- Noise XX encrypted CYBChat with delivery ACK;
- TOFU peer identity persistence and changed-key rejection;
- Cybergraph secure-peer persistence;
- stoppable LAN and secure listeners;
- bounded HTTP workers for Qwen, web, Solana balances and market data;
- macOS app packaging and release workflow.

## Public macOS beta
Estimated readiness: **90–95%**.

Remaining blockers are mostly productization and real-device validation:
- signed/notarized distribution;
- two-Mac interoperability test of RADAR + Secure CYBChat + TOFU;
- upgrade/reinstall and recovery testing;
- publish the hardened v0.7.2 macOS build and complete real-device interoperability validation.

## Production 1.0
Estimated readiness: **75–80%**.

Remaining work includes security review, cross-version interoperability,
configuration migration, diagnostics/recovery, signed updates and operational
release/support workflows.

The v0.7.1 macOS ZIP is published through the companion `c1cad4/cybOS` release repository. Main contains replay protection, recovery diagnostics, truthful listener errors, cached database diagnostics, WAL persistence hardening, and universal ARM64 + Intel packaging for v0.7.2.
Physical RobotCYB hardware, farm sensor/camera integrations and Cyblex are
future product layers and do not block the first desktop beta.


Verification note: current main contains the runtime hardening, replay cache, recovery diagnostics and universal packaging used for the v0.7.2 release line.
