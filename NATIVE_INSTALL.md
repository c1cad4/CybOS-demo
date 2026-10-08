# cybOS native macOS installation

## Current status

The native Rust/egui application builds as a real macOS .app bundle. Repository CI verifies:

- Linux cargo check + cargo test
- macOS cargo check + cargo test
- macOS release build
- CoreBluetooth advertiser compilation
- whitespace validation

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

Open the app by double-clicking dist/cybOS.app.

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

## Network permissions

CYB RADAR is opt-in. LAN discovery uses the local network; macOS BLE discovery uses CoreBluetooth. Hidden mode disables CYB visibility/advertising.

Secure CYBChat uses a separate Noise XX encrypted transport and persists first-seen peer keys as TOFU.

## What is not bundled yet

The Qwen model weights are not bundled into the application package. Farm hardware, cameras and RobotCYB physical hardware integrations are also outside the desktop package.

## First public release path

Tagging the repository with a v* tag triggers .github/workflows/release-macos.yml, which builds and publishes the macOS ZIP as a GitHub Release asset.
