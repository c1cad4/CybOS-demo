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
**Not yet cleared for public release.** The previous 90–95% estimate should not be treated as a security sign-off.

In addition to productization and real-device validation, local secret/data protection is still incomplete. The PR branch now includes a macOS Keychain-backed Noise private-key path and guarded migration from the legacy SQLite key, but this still needs CI and migration/recovery validation. Peer trust pins and chat history remain in SQLite. Noise protects transport; it does not by itself encrypt the local database. See [Security Review](docs/SECURITY_REVIEW.md).

Release blockers:
- CI plus clean-install, restart, legacy-migration and Keychain-denial validation for the new macOS Keychain private-key path;
- an explicit local chat-history protection policy;
- two-Mac interoperability tests for RADAR + Secure CYBChat + TOFU;
- upgrade/reinstall and recovery testing;
- complete real-device interoperability validation;
- CybLex two-node download/seed testing with authorized content;
- signed/notarized distribution.

## CybDEX
The first market-data phase is implemented as a read-only native terminal. Swap routing, wallet signing and an own on-chain AMM remain separate future layers.

## Production 1.0
Estimated readiness: **75–80%**.

Remaining work includes security review, cross-version interoperability,
configuration migration, diagnostics/recovery, signed updates and operational
release/support workflows.

The v0.7.2 macOS ZIP is published through the companion `c1cad4/cybOS` release repository. Main contains replay protection, recovery diagnostics, truthful listener errors, cached database diagnostics, WAL persistence hardening, and universal ARM64 + Intel packaging for v0.7.2.
Physical RobotCYB hardware and farm sensor/camera integrations remain future product layers. CybLex is now part of the desktop beta; two-node interoperability still needs real-device validation.


Verification note: current main contains the runtime hardening, replay cache, recovery diagnostics and universal packaging used for the v0.7.2 release line.
