#!/usr/bin/env python3
"""Package the AVKit playback helper, including its own application icon."""
import argparse
from pathlib import Path
import plistlib
import shutil
from build_app_icon import build as build_icon


def package(binary, output):
    binary, output = Path(binary), Path(output)
    if not binary.is_file():
        raise ValueError('Player executable is missing')
    executable = output / 'Contents/MacOS/cybos-player'
    executable.parent.mkdir(parents=True, exist_ok=True)
    shutil.copyfile(binary, executable)
    executable.chmod(0o755)
    build_icon(output / 'Contents/Resources/CybOS.icns')
    (output / 'Contents/Info.plist').write_bytes(plistlib.dumps({
        'CFBundleIdentifier': 'to.cicada.cybos.player',
        'CFBundleIconFile': 'CybOS.icns',
        'CFBundleName': 'CybPlayer',
        'CFBundleExecutable': 'cybos-player',
        'CFBundlePackageType': 'APPL',
        'LSMinimumSystemVersion': '11.0',
        'NSHighResolutionCapable': True,
        'NSAppTransportSecurity': {'NSAllowsLocalNetworking': True},
    }))


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary', required=True)
    parser.add_argument('--output', required=True)
    args = parser.parse_args()
    package(args.binary, args.output)
