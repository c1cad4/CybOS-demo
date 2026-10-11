# cybOS product readiness

## Verified development baseline

The native macOS-first desktop compiles and its 28 current Rust tests pass on
macOS ARM64 (11 October 2026, Rust 1.99.0). Ten pinned sibling libraries pass
24 unit tests. Setup/launcher regression tests verify recovery from failed
fetches, preserved local work, pinned HEAD checks, fresh-clone builds, sibling
source rebuilds and bundle-version consistency.

These checks validate the local development baseline. They do not establish
public-beta or production readiness percentages.

## Release blockers

- Secure-chat identity writes ignore SQLite failures in current main.
- Secure-listener bind/setup failures are not surfaced to the caller.
- Peer fingerprints still use FNV; cryptographic fingerprints and explicit
  first-contact approval are proposed in existing PRs.
- Two-Mac LAN/BLE/Noise interoperability, restart/recovery and upgrade tests
  remain necessary.
- Developer ID signing/notarization requires the owner's configured Apple
  credentials; it was not exercised in this verification.
- CybLex download/seed needs an actual two-node interoperability test.

[Development order](docs/DESKTOP_DEVELOPMENT.md) maps the existing PR chains
onto the current desktop. Passing unit tests must not be reported as evidence
that these outstanding failures have been fixed.

## Product boundaries

The native desktop and SQLite state are local-first. RobotCYB requires a
separately configured local Qwen server for model answers. CybDEX is read-only;
no swap is signed or submitted. Farm sensors, cameras and physical actuators
require real adapters and device validation. The browser prototype remains a
visual experiment. The optional Python CybCore has its own API and test suite;
it is not a mandatory service for opening the desktop.
