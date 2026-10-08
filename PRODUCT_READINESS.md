# cybOS product readiness

This document separates the first usable desktop product from the longer-term
CybOS vision.

## Current usable scope

The native macOS application already provides:

- native Rust/egui desktop shell;
- local SQLite state and persistent node identity;
- asynchronous bounded RobotCYB execution;
- local Qwen integration with an end-to-end 90 second RobotCYB deadline;
- CYB RADAR with opt-in LAN discovery and macOS BLE RSSI proximity;
- unified LAN/BLE peer projection;
- Secure CYBChat over Noise XX with encrypted frames and delivery ACK;
- persistent first-seen peer identity (TOFU) and changed-key rejection;
- secure peer relationships in Cybergraph;
- stoppable long-lived LAN and secure listeners;
- bounded HTTP workers for Qwen, web search/fetch, Solana balances and market data;
- macOS application packaging and a release workflow.

## Release levels

### Technical beta — READY

A technical user can build or package the native application and use the local
desktop, RobotCYB, local persistence, discovery and secure chat features.

### Public macOS beta — approximately 80–85%

The remaining work is product packaging and validation rather than core
architecture:

1. signed/notarized distribution for a normal double-click installation path;
2. first-run onboarding for Qwen and permissions;
3. real two-Mac interoperability test for RADAR + Secure CYBChat + TOFU;
4. upgrade/reinstall path and a small crash/recovery test matrix.

The percentage is a planning estimate, not a test metric.

### Production 1.0 — approximately 65–70%

Additional work is still required for production-grade operations:

- release signing/notarization and update strategy;
- broader cross-version interoperability and security testing;
- recovery/diagnostics and clearer user-facing failure states;
- persistent configuration/settings migration;
- privacy/security review of discovery and trust flows;
- release documentation and support workflows.

### Long-term CybOS vision — not the same milestone

Physical RobotCYB hardware, robot firmware/OS integration, farm sensor/camera
integrations, broader peer protocols and Cyblex are future layers. They should
not block the first usable desktop cybOS release.

## Practical target

The next milestone is a **public macOS beta**, not a rewrite.

The core runtime and secure communication stack are already in place; the focus
should now shift from architectural refactoring to packaging, onboarding,
two-node validation and release hardening.
