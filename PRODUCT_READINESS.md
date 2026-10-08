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
- macOS app packaging and release workflow;
- native CybLex P2P archive powered by embedded librqbit 9.0.1.

## Public macOS beta
Estimated readiness: **90–95%**.

Remaining blockers are mostly productization and real-device validation:
- signed/notarized distribution;
- two-Mac interoperability test of RADAR + Secure CYBChat + TOFU;
- upgrade/reinstall and recovery testing;
- complete real-device interoperability validation;
- verify CybLex two-node download/seed interoperability with authorized test content.

## CybDEX
The first market-data phase is implemented as a read-only native terminal. Swap routing, wallet signing and an own on-chain AMM remain separate future layers.

## Production 1.0
Estimated readiness: **75–80%**.

Remaining work includes security review, cross-version interoperability,
configuration migration, diagnostics/recovery, signed updates and operational
release/support workflows.

The v0.7.1 macOS ZIP is published through the companion `c1cad4/cybOS` release repository. Main contains replay protection, recovery diagnostics, truthful listener errors, cached database diagnostics, WAL persistence hardening, and universal ARM64 + Intel packaging for v0.7.2.
Physical RobotCYB hardware and farm sensor/camera integrations remain future product layers. CybLex is now part of the desktop beta; two-node interoperability still needs real-device validation.


Verification note: current main contains the runtime hardening, replay cache, recovery diagnostics and universal packaging used for the v0.7.2 release line.
