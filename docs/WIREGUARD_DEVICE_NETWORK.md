# CybOS private device network — preparation stage

This version prepares interoperable WireGuard configurations. It does not install
WireGuard, create a tunnel, change system routes, forward traffic, monitor
App Store tunnels, or report a working VPN. A real server and second endpoint are still
required for the connectivity and recovery tests below.

## Device on macOS

1. Install the official [WireGuard app](https://www.wireguard.com/install/).
2. Open CybOS → Network → Create / load device identity.
3. Send only the displayed **public** key to your server administrator.
4. Obtain `device-invitation.json` from that administrator through a trusted
   channel. Import it in the Network panel, or enter the same public settings.
5. Compare the complete server public key with the administrator. Mark the
   verification checkbox only after the comparison.
6. Export the WireGuard profile. The `.conf` contains the device's private key:
   import it locally into the official WireGuard app and keep it private.
7. Activate the profile in WireGuard after the server is configured. CybOS's
   export-success message is not a connection-status indicator.
   For CLI-based tunnels, expand CLI tunnel diagnostics to query matching local
   and server public keys and the latest handshake. App Store tunnels are not
   visible to that CLI diagnostic. A recent handshake does not prove routing or
   CybChat delivery.
8. Add the other CybOS device's tunnel IP and node ID in Chat → Add device by
   address. CybChat Noise approval remains separate from WireGuard enrollment.

Device private keys are separate from CybChat Noise keys. They are stored under
`<CYBOS_DATA_DIR>/wireguard/device.key`, or the application's normal data
location. Directories require private Unix permissions; key/config files are
created with mode 0600. Existing identities are never replaced on load errors.
Public profile metadata is saved in SQLite; private WireGuard keys are not.
Windows secret storage is not implemented in this stage. Files are not encrypted
at rest or placed in macOS Keychain in this stage.

## Prepare a first server

Use a Linux server with a public IPv4 address and reachable UDP port 51820.
The script prepares files only; run it on the server after installing official
`wireguard-tools`. Assign each device a unique address. The first device defaults
to `10.77.0.2/32`; the server uses `10.77.0.1/24`.

```sh
python3 scripts/prepare_wireguard_server.py \
  --output-dir ./cybos-server \
  --endpoint SERVER_PUBLIC_IPV4:51820 \
  --device-public-key DEVICE_PUBLIC_KEY \
  --device-address 10.77.0.2/32
```

Replace the uppercase values before running. Output is a new private directory
containing `cybos.conf` and a public `device-invitation.json`. Existing output
folders are rejected to prevent replacing server keys. No private material is
printed. Transfer only the invitation to the device.

Review the server configuration, permit UDP 51820 in the host/provider firewall,
then install and activate it on Linux:

```sh
sudo install -d -m 700 /etc/wireguard
sudo install -m 600 cybos-server/cybos.conf /etc/wireguard/cybos.conf
sudo wg-quick up cybos
```

Before installing, ensure `/etc/wireguard/cybos.conf` does not already exist:
choose another interface name or review the existing setup instead of replacing
it. Interface names and network addresses must not conflict with existing VPNs
or LANs. Choose another private subnet in the GUI if 10.77.0.0/24 is in use;
the first-server helper currently fixes that subnet, so its configuration must
be reviewed and adjusted consistently in that case.

The first configuration contains one client. Prepare a second without changing
that server's keys or the original files:

```sh
python3 scripts/add_wireguard_peer.py \
  --server-dir ./cybos-server \
  --output-dir ./cybos-server-next \
  --endpoint SERVER_PUBLIC_IPV4:51820 \
  --device-public-key SECOND_DEVICE_PUBLIC_KEY \
  --device-address 10.77.0.3/32
```

The new folder contains a full server configuration retaining all existing peers
and an invitation for the new device only. Duplicate addresses and keys, the
server's own key, changed listen ports, permissive source files and unsupported
configuration settings are rejected. This helper accepts the minimal format
created by the first-server helper; it rejects customized configurations rather
than silently discarding policy. Review and deploy the new configuration through
your existing server maintenance process; nothing is applied automatically. For
later devices, use the latest folder as `--server-dir` and a new output folder.

Client-to-client routing through the server additionally requires
IPv4 forwarding and firewall rules allowing the intended device subnet.
Those policy changes are not performed by the helper. No internet NAT or
exit-node routing is configured.

For persistent restart support on a systemd Linux host, enable the reviewed
`wg-quick@cybos` service. This guide does not remotely execute these commands.

## Revoke a device

On the server, remove its peer from the persistent configuration and apply the
same removal to the active interface:

```sh
sudo wg set cybos peer DEVICE_PUBLIC_KEY remove
```

A runtime-only removal is insufficient: otherwise restarting can restore the
peer. Do not rotate the server key when removing one client. Keep a local console
or another management connection available while editing network configuration.
The GUI does not yet manage server enrollment or revocation remotely.

## What must pass before calling the network operational

- Client can reach 10.77.0.1, and the server reports a recent handshake for that
  client's public key (`sudo wg show cybos latest-handshakes`).
- Two enrolled devices exchange actual CybChat messages over tunnel addresses,
  with explicit Noise approvals and delivery ACKs.
- Reboot, macOS sleep/wake, Wi-Fi change and temporary server loss recover after
  activation; record reconnect time rather than assuming it works.
- Removing a peer blocks access immediately and remains effective after reboot.
- Ordinary internet traffic and DNS retain their previous path. Check IPv4 and
  IPv6 independently; this stage tunnels the selected private IPv4 subnet only.
- Duplicate addresses, subnet conflicts, incorrect keys and unreachable UDP
  produce clear diagnostics in the WireGuard client/server.

## Validation performed in development

Rust tests cover route/key/input validation, public-invitation format, private
file permissions, corrupt-key behavior, persistence and refusal to export before
server-key verification. Python tests cover server configuration preparation,
reserved device addresses, invitation secrecy and refusal to overwrite files.

Read-only CLI diagnostics use only `wg show all public-key` and
`wg show all latest-handshakes`, with bounded subprocesses. They never use
`wg show dump`, which can expose private keys. Tests distinguish unrelated
interfaces, wrong peers, absent/stale/recent handshakes and invalid clocks.

The public key derived by CybOS was checked against official `wg pubkey`,
including the identity created through the GUI. Client/server WireGuard peer
settings were accepted by the official `wg` parser after removing the
`wg-quick`-specific Address line; application then failed against a deliberately
nonexistent interface. No tunnel was created by this syntax test.
The official macOS `wg-quick strip` script was not run successfully because the
system Bash is version 3 and that script requires Bash 4+. This does not verify
import or activation in the official macOS app.

Protocol/configuration references: [WireGuard quick start](https://www.wireguard.com/quickstart/),
[official tools source](https://github.com/WireGuard/wireguard-tools).
