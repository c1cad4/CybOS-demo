# cybOS security review — v0.7.x

This review distinguishes transport security from local-data security. A green UI status or successful CI run is not a security audit.

## Fixed in PR #24

- A malformed persisted Noise private key is rejected instead of silently generating a replacement identity.
- Noise identity creation now fails if SQLite cannot persist and read back the key.
- Secure chat is disabled when identity initialization fails; it no longer starts a listener with an empty key or reports READY.
- First-contact peer trust must be persisted before the message is accepted.
- A malformed/corrupt stored peer-key pin is rejected rather than treated as a new TOFU peer.
- Inbound secure-session admission is atomically capped; accepted sockets are normalized before framed reads.
- Regression tests cover malformed keys, strict hex decoding, failed SQLite writes, and oversized frame rejection.
- Runtime cells begin in IDLE and become READY only after an explicit subsystem status update.

## Remaining release blockers

### P0 — Protect secrets and local conversations at rest

The PR now includes a macOS Keychain-backed Noise private-key path with a guarded one-time migration from legacy SQLite storage. It verifies read-back, checks identity consistency if both copies exist, and refuses to silently fall back or rotate the identity when Keychain access fails. CI and migration/recovery tests are still required before treating this path as verified. Peer-key pins and chat history remain in SQLite; Noise transport encryption does **not** encrypt these local records.

Before a public release:
- Validate the macOS Keychain implementation on clean install, legacy migration, restart, locked/denied Keychain access, mismatched copies and interrupted migration.
- Define and test an OS-backed trust-pin store or explicitly include SQLite trust pins in the accepted local threat model.
- Encrypt sensitive chat history at rest or clearly disclose the local threat model and provide an explicit history-protection setting.
- Set restrictive permissions on the app data directory, database, WAL, and shared-memory files; test creation and migration on a clean install. SQLite durability is configured to FULL in the PR branch.
- Treat corrupt trust records as a security error requiring explicit user recovery, never automatic re-pairing.

### P1 — Verify the network boundary

- Run two-device tests for first-contact trust, trusted reconnect, changed-key rejection, replay rejection, oversized frames, timeout, listener shutdown, and concurrent connection limits.
- Verify firewall/network-interface behavior. The current listener binds to 0.0.0.0:39394; document that it is reachable on all local interfaces and confirm this is intended.
- Test that all workers terminate or become observably stopped when the application closes.
- Confirm no UI state reports a service as READY unless that service has actually initialized.

### P1 — Verify CybLex with authorized content

- Test magnet and .torrent downloads, local seeding, pause/resume, persistence across restart, and two-node transfer.
- Verify that deleting a torrent never deletes user files unless the user explicitly chooses that action.
- Use only content the user owns or is authorized to distribute.

### P2 — Release engineering

- Require Linux and macOS CI to pass on the final PR head, including ARM64/Intel builds and universal packaging.
- Test clean install, upgrade, restart, corrupted-database recovery, and rollback.
- Sign and notarize macOS distribution before presenting it as a normal consumer installer.

## Current disposition

The fixes in PR #24 reduce identity and listener risks, but do not complete the items above. This review does not certify cybOS as production-secure. Public release should remain gated on OS-backed key storage, explicit local-data protection, and successful real-device validation.
