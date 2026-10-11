#!/usr/bin/env python3
"""Convert the supplied CybOS artwork into a macOS launcher icon."""
import argparse
import base64
from pathlib import Path
import subprocess
import tempfile
import struct


def build(output):
    artwork = Path(__file__).resolve().parents[1] / 'assets/cybos-logo.png.base64'
    output = Path(output).resolve()
    output.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix='cybos-icon-') as scratch:
        root = Path(scratch)
        source = root / 'logo.png'
        source.write_bytes(base64.b64decode(artwork.read_text().strip(), validate=True))
        iconset = root / 'CybOS.iconset'
        iconset.mkdir()
        for size in [16, 32, 128, 256, 512]:
            for scale in [1, 2]:
                name = f'icon_{size}x{size}' + ('@2x' if scale == 2 else '') + '.png'
                subprocess.run(['sips', '-z', str(size * scale), str(size * scale), str(source), '--out', str(iconset / name)], check=True, stdout=subprocess.DEVNULL)
        chunks = []
        for tag, name in [('icp4','icon_16x16.png'), ('icp5','icon_32x32.png'), ('ic07','icon_128x128.png'), ('ic08','icon_256x256.png'), ('ic09','icon_512x512.png'), ('ic10','icon_512x512@2x.png')]:
            image = (iconset / name).read_bytes()
            chunks.append(tag.encode('ascii') + struct.pack('>I',len(image)+8) + image)
        payload = b''.join(chunks)
        output.write_bytes(b'icns' + struct.pack('>I',len(payload)+8) + payload)



if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', required=True)
    build(parser.parse_args().output)
