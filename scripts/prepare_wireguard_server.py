#!/usr/bin/env python3
"""Prepare a new server profile without installing/activating WireGuard or changing routes."""
import argparse
import base64
import ipaddress
import json
import os
from pathlib import Path
import subprocess


def public_key(value):
    try:
        raw = base64.b64decode(value, validate=True)
    except (ValueError, TypeError):
        raise ValueError('Invalid device public key') from None
    if len(raw) != 32 or not any(raw) or base64.b64encode(raw).decode() != value:
        raise ValueError('Invalid device public key')
    return value


def validate(endpoint, device_address, device_key):
    host, sep, port = endpoint.rpartition(':')
    try:
        ip = ipaddress.IPv4Address(host)
        port = int(port)
        device = ipaddress.IPv4Interface(device_address)
    except ValueError:
        raise ValueError('Use server IPv4:port and a device IPv4/32') from None
    if not sep or not endpoint.rpartition(":")[2].isascii() or not endpoint.rpartition(":")[2].isdigit() or not 1 <= port <= 65535 or ip.is_unspecified or ip.is_multicast or str(ip) == '255.255.255.255':
        raise ValueError('Invalid server endpoint')
    subnet = ipaddress.IPv4Network('10.77.0.0/24')
    if device.network.prefixlen != 32 or device.ip not in subnet or device.ip in (subnet.network_address, subnet.broadcast_address, ipaddress.IPv4Address('10.77.0.1')):
        raise ValueError('Choose a unique client address from 10.77.0.2/32 through 10.77.0.254/32')
    return public_key(device_key), str(device)


def wg_run(tool, args, input_data=None):
    try:
        result = subprocess.run([tool, *args], input=input_data, text=True, capture_output=True, timeout=5, check=True)
    except (OSError, subprocess.SubprocessError):
        raise RuntimeError('WireGuard tool failed; install official wireguard-tools and retry') from None
    return result.stdout.strip()


def write_new(path, text):
    fd = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
    with os.fdopen(fd, 'w') as stream:
        stream.write(text)
        stream.flush()
        os.fsync(stream.fileno())


def prepare(output, endpoint, device_address, device_key, tool='wg'):
    device_key, device_address = validate(endpoint, device_address, device_key)
    output = Path(output)
    if output.exists() or output.is_symlink():
        raise ValueError('Output directory already exists; existing keys are never replaced')
    private = wg_run(tool, ['genkey'])
    # Validate tool output before writing; never print private material.
    public_key(private)
    server_key = public_key(wg_run(tool, ['pubkey'], private + '\n'))
    if server_key == device_key:
        raise ValueError('Server and client keys must differ')
    output.mkdir(mode=0o700)
    config = f'[Interface]\nPrivateKey = {private}\nAddress = 10.77.0.1/24\nListenPort = {endpoint.rpartition(":")[2]}\n\n[Peer]\nPublicKey = {device_key}\nAllowedIPs = {device_address}\n'
    write_new(output / 'cybos.conf', config)
    invitation = {'version': 1, 'endpoint': endpoint, 'server_key': server_key, 'address': device_address, 'network': '10.77.0.0/24'}
    write_new(output / 'device-invitation.json', json.dumps(invitation, indent=2) + '\n')
    print('Prepared server configuration; no tunnel was activated.')
    print('Public server key:', server_key)
    print('Invitation:', output / 'device-invitation.json')
    return invitation


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output-dir', required=True)
    parser.add_argument('--endpoint', required=True)
    parser.add_argument('--device-public-key', required=True)
    parser.add_argument('--device-address', default='10.77.0.2/32')
    parser.add_argument('--wg', default='wg', help='Path to official wg executable')
    args = parser.parse_args()
    try:
        prepare(args.output_dir, args.endpoint, args.device_address, args.device_public_key, args.wg)
    except (ValueError, RuntimeError, OSError) as error:
        parser.exit(1, str(error) + '\n')


if __name__ == '__main__':
    main()
