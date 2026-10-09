# cybOS execution roadmap — 0.7.x to 1.0

This roadmap separates implemented code from validated behavior. A UI control or successful compile is not proof of end-to-end operation.

## Product invariants
1. **Local-first:** each network, AI, P2P, Bluetooth, camera, sensor and API integration reports its real state.
2. **Bounded cells:** workers have explicit inputs/outputs, budgets, heartbeats, status and shutdown/error paths.
3. **Identity continuity:** corrupt or unavailable identity storage must never silently rotate a peer identity.
4. **Truthful telemetry:** unavailable sensors are shown as unavailable, never replaced with demo values.
5. **Explicit authority:** no arbitrary shell execution, automatic wallet signing/payments or physical actuation without a reviewed capability and user approval.
6. **Safe file lifecycle:** forgetting a torrent is distinct from deleting its downloaded files.
7. **Lawful P2P:** CybLex transports content users own or are authorized to distribute; it is not an index of unauthorized copyrighted material.

## P0 — Local data and identity
- [ ] Move the Noise static private key to macOS Keychain and define an OS-backed provider for any other supported OS.
- [ ] Implement a one-time migration from the legacy SQLite key; verify the public identity is unchanged before removing the legacy copy.
- [ ] Fail closed with recovery instructions if secure storage is unavailable or migration is ambiguous.
- [ ] Choose and implement a clear chat-history protection policy: authenticated encryption at rest or an explicit, documented local threat model and setting.
- [ ] Restrict permissions on the database, WAL and shared-memory files; test clean install, upgrade, backup/restore and migration.
- [ ] Exclude secrets, private messages and credentials from logs and diagnostic exports.

**Acceptance:** restart preserves identity; migration preserves the same public key; secure-store denial never rotates identity silently; corruption yields actionable recovery.

## P0 — CI and release gates
- [ ] Latest PR head passes Linux and macOS cargo check and cargo test.
- [ ] Build Apple Silicon and Intel targets, compile CoreBluetooth paths, validate universal packaging and whitespace.
- [ ] Add dependency auditing and a Rust dependency update policy.
- [ ] Distinguish build success, automated tests, two-device validation, notarization and public-release approval.
- [ ] Keep public release blocked until security review and device tests are complete.

**Acceptance:** release workflow builds the exact tagged commit and publishes a checksum for the exact ZIP asset.

## P1 — CYBChat and CYB RADAR
- [ ] Test first contact, trusted reconnect, changed-key rejection, replay rejection, oversized frames, timeouts, connection caps and listener shutdown on two Macs.
- [ ] Make bind interface/port visible; review whether listening on all interfaces at port 39394 is intended.
- [ ] Verify hidden mode stops discovery and advertising; report workers that do not stop.
- [ ] Separate peer trust from reachability; never infer geographic distance from BLE RSSI.
- [ ] Add deliberate peer re-pair/recovery with confirmation and an audit event.

## P1 — CybLex P2P archive
- [ ] Test magnet and torrent-URL downloads, local file/folder seeding, pause/resume, restart recovery and two-node transfer.
- [ ] Compare displayed progress/state/upload counters against the engine's actual state.
- [ ] Keep “forget task” separate from “delete payload”; never remove user files as a side effect of forgetting.
- [ ] Bound metadata and path handling; report disk-full, permission and tracker errors.
- [ ] Add upload/download bandwidth limits and clear NAT/firewall status.
- [ ] Publish magnet URI and torrent sidecar only after successful creation.
- [ ] Verify clean engine shutdown.

**Acceptance:** two independent nodes exchange an authorized test file with a verified hash; pause/resume/restart/forget never unexpectedly delete payload data.

## P1 — RobotCYB, local AI and Brain
- [ ] Make Qwen endpoint/model configuration visible and validate it before dispatch.
- [ ] Bound prompt/response sizes, time, concurrency and cancellation.
- [ ] Distinguish server reachable, model loaded and request succeeded.
- [ ] Treat model output as untrusted; it cannot directly execute commands, spend funds, change trust records or actuate hardware.
- [ ] Add bounded task traces with correlation IDs, deadlines and cancellation outcomes.
- [ ] Track memory provenance: user input, retrieved content, model summary or tool observation.

## P1 — Runtime cells and workflow engine
- [ ] Every cell declares versioned inputs/outputs, budget, owner and failure policy.
- [ ] Heartbeats originate from the actual worker, not the UI/scheduler.
- [ ] Add cancellation propagation, queue backpressure, shutdown deadlines and orphan-worker diagnostics.
- [ ] Enforce capability allow-lists at execution time.
- [ ] Test all workflow transitions, terminal states, timeout, cancellation, late worker return and queue saturation.
- [ ] Bound workflow artifacts and redact secrets from exported traces.

## P1 — CybBrowser and external networking
- [ ] Keep remote JavaScript disabled until sandboxing and permissions are implemented.
- [ ] Test response-size limits, deadlines, redirects, malformed URLs, relative links and gateway fallback.
- [ ] Make external gateway fallback visible; do not label a gateway response as a local-node response.
- [ ] Test malformed/adversarial HTML and local/private-network access policy.
- [ ] Keep UI responsive under slow and failed requests.

## P2 — UI, accessibility and diagnostics
- [ ] Test narrow windows, display scaling, keyboard navigation and contrast.
- [ ] Give every workspace purpose, first step, empty/busy/error states and real connectivity state.
- [ ] Label future features; remove controls that imply functionality that is not implemented.
- [ ] Provide recovery diagnostics and import/export without credentials or private keys.
- [ ] Use consistent notifications with actionable errors.
- [ ] Ensure dashboard telemetry comes from real services, never fixed demo values.

## P2 — Assets, CybDEX and economy
- [ ] Keep market data read-only until transaction signing receives a separate threat model and approval flow.
- [ ] Reject non-finite and inconsistent external market values; label stale/cached data.
- [ ] Do not store signing keys or enable autonomous swaps/payments in this beta.
- [ ] Keep local ledger records separate from actual payment execution.

## P2 — CicadaFarm, cameras and robot hardware
- [ ] Define a versioned sensor protocol with timestamp, unit, source, calibration and stale-data status.
- [ ] Treat configured devices as offline until real telemetry arrives.
- [ ] Implement read-only sensor ingestion before any control path.
- [ ] Require explicit approval, physical emergency stop and safe-state defaults before actuation.
- [ ] Test disconnected, stale, out-of-range and malformed readings.

## P2 — Packaging, updates and operations
- [ ] Test clean install, upgrade, restart, corrupted-database recovery and rollback.
- [ ] Sign and notarize macOS distribution; keep signing secrets out of repository and logs.
- [ ] Publish SHA-256 checksums for every release asset.
- [ ] Document backup, uninstall and remaining user data.
- [ ] Keep crash logs local by default; diagnostic upload must be opt-in.

## Execution order
1. Restore a green build on the latest PR head.
2. Protect local identity/history and test migration/recovery.
3. Complete two-device CYBChat/RADAR validation.
4. Complete CybLex lifecycle and two-node transfer tests.
5. Verify runtime cancellation, deadlines and capability enforcement.
6. Finish clean install/upgrade/recovery and signed distribution.
7. Revisit hardware adapters and transaction-capable features only after safety gates pass.

This is a checklist, not a claim that every item is implemented.
