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

The published macOS beta is v0.7.2:

https://github.com/c1cad4/cybOS/releases/tag/v0.7.2

Download the ZIP, unpack it, and open `cybOS.app`.

On a first launch, macOS may show a Gatekeeper warning because the current public binary is not Developer ID signed/notarized. Use Finder's **Open** action on the app, or in **System Settings → Privacy & Security** use the **Open Anyway** action for cybOS.

The current `main` branch contains universal macOS ARM64 + Intel packaging, replay protection, WAL persistence hardening, cached database diagnostics, bounded Qwen input, and optional Developer ID/notarization support.

### Platform support status

| Platform | Status |
|---|---|
| macOS Apple Silicon + Intel | Published beta; signing/notarization depends on configured Apple credentials |
| Linux | CI compile/test target; no supported end-user package documented here |
| Windows | Experimental CI compile/test target; no supported end-user installer yet |

A successful cross-platform compile is not the same as feature parity or a supported release. Hardware access, key storage, packaging, upgrade behavior and native interaction still require platform-specific validation.

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

The current published macOS beta is **v0.7.2**:

[Open cybOS v0.7.2 release](https://github.com/c1cad4/cybOS/releases/tag/v0.7.2)

The release contains `cybOS-0.7.2-macOS.zip` and `cybOS-0.7.2-macOS.zip.sha256`.

Tagging the repository with a `v*` tag triggers `.github/workflows/release-macos.yml`, which builds and publishes the macOS ZIP as a GitHub Release asset.

For a smooth end-user macOS installation experience, Developer ID signing + Apple notarization can be enabled without changing the application code.

Set:
~~~bash
export CODESIGN_IDENTITY="Developer ID Application: Your Name (TEAMID)"
export NOTARY_PROFILE="cybOS-notary"
./scripts/package_macos.sh
~~~

When those variables are present, packaging signs the app with the hardened runtime, verifies the signature, submits the ZIP with xcrun notarytool, staples the notarization ticket to the app, validates it, and rebuilds the final ZIP.

The repository does not contain Apple signing certificates, credentials or a notary keychain profile, so the actual Apple trust step must be performed in the owner's macOS/keychain or configured CI secrets.


## CybLex

CybLex embeds **librqbit 9.0.1** directly into the native app. No separate rqbit installation is required.

The native CybLex page supports:
- magnet links and HTTP(S) .torrent URLs for downloads;
- a configurable local output directory;
- creating and seeding a torrent from a local file or folder;
- generating a .torrent sidecar and magnet URI;
- pause, resume and forget operations;
- live progress and upload counters.

The first version intentionally keeps the torrent engine inside a dedicated background Tokio runtime instead of blocking the egui/UI thread.

Use CybLex only for content you own or are authorized to distribute, including public-domain, open-license and creator-authorized material.
