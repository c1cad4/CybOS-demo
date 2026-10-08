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
Estimated readiness: **85–90%**.

Remaining blockers are mostly productization and real-device validation:
- signed/notarized distribution;
- two-Mac interoperability test of RADAR + Secure CYBChat + TOFU;
- upgrade/reinstall and recovery testing.

## Production 1.0
Estimated readiness: **70–75%**.

Remaining work includes security review, cross-version interoperability,
configuration migration, diagnostics/recovery, signed updates and operational
release/support workflows.

The v0.7.1 macOS ZIP is now published through the companion `c1cad4/cybOS` release repository.
Physical RobotCYB hardware, farm sensor/camera integrations and Cyblex are
future product layers and do not block the first desktop beta.


Verification note: this document is intentionally the only branch-specific file; the code under test is the current main runtime at branch base.
