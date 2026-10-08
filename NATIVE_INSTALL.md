# cybOS native macOS installation

## Current status

cybOS is a working native macOS technical beta. Repository CI verifies:

- Linux `cargo check` + `cargo test`;
- macOS `cargo check` + `cargo test`;
- macOS release build;
- CoreBluetooth advertiser compilation;
- macOS packaging-script validation;
- whitespace validation.

The current native runtime includes local SQLite state with WAL/busy-timeout hardening, RobotCYB/Qwen integration, CYB RADAR, unified LAN+BLE proximity, Noise XX secure CYBChat, replay protection, TOFU peer identity and Cybergraph persistence.

## Download the macOS beta

The published v0.7.1 package is available from the companion release repository:

https://github.com/c1cad4/cybOS/releases/tag/v0.7.1

Download the ZIP, unpack it, and open `cybOS.app`.

On a first launch, macOS may show a Gatekeeper warning because the current public binary is not Developer ID signed/notarized. Use Finder's **Open** action on the app, or in **System Settings → Privacy & Security** use the **Open Anyway** action for cybOS.

The current `main` branch already contains universal ARM64 + Intel packaging for the next public release.

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

The current public beta is **v0.7.1**:

[Download cybOS v0.7.1 for macOS](https://github.com/c1cad4/cybOS/releases/download/v0.7.1/cybOS-0.7.1-macOS.zip)

Tagging the repository with a `v*` tag triggers `.github/workflows/release-macos.yml`, which builds and publishes the macOS ZIP as a GitHub Release asset.

For a smooth end-user macOS installation experience, the remaining distribution step is Developer ID signing + Apple notarization. The repository does not contain Apple signing credentials, so that step cannot be honestly marked complete from CI alone.
