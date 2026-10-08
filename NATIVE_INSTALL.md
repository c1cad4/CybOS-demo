# cybOS native macOS installation

## Current status

cybOS is a working native macOS technical beta. Repository CI verifies:

- Linux `cargo check` + `cargo test`;
- macOS `cargo check` + `cargo test`;
- macOS release build;
- CoreBluetooth advertiser compilation;
- macOS packaging-script validation;
- whitespace validation.

The current native runtime includes local SQLite state, RobotCYB/Qwen integration, CYB RADAR, unified LAN+BLE proximity, Noise XX secure CYBChat, TOFU peer identity and Cybergraph persistence.

## Build locally

From the repository root:

~~~bash
./scripts/package_macos.sh
~~~

The result is:

~~~text
dist/cybOS.app
dist/cybOS-<version>-macOS.zip
~~~

Open the app by double-clicking `dist/cybOS.app`.

## RobotCYB / local Qwen

The desktop shell works without Qwen. RobotCYB becomes fully functional when a local OpenAI-compatible Qwen server is available at:

~~~text
127.0.0.1:8080
~~~

The current macOS launcher looks for:

~~~text
~/cybAI/.venv/bin/mlx_lm.server
~~~

and starts the configured local Qwen model automatically when available.

The model weights are intentionally not bundled into the repository. This keeps the application package small, but it means the current beta is not yet a zero-setup AI distribution.

## Network permissions

CYB RADAR is opt-in.

LAN discovery uses the local network. macOS BLE discovery uses CoreBluetooth. Hidden mode disables CYB visibility and BLE advertising.

Secure CYBChat uses Noise XX encrypted transport and stores first-seen peer public keys locally as TOFU.

## What is not bundled yet

- Qwen model weights;
- farm hardware integration;
- live camera hardware;
- P2P and Nostr fallback transports.

These are separate product layers, not prerequisites for the current desktop core.

## Public release path

Tagging the repository with a `v*` tag triggers `.github/workflows/release-macos.yml`, which builds and publishes the macOS ZIP as a GitHub Release asset.

For a smooth end-user macOS installation experience, the remaining distribution step is Developer ID signing + Apple notarization. The repository does not contain Apple signing credentials, so that step cannot be honestly marked complete from CI alone.
