#!/usr/bin/env python3
"""Package the locally built browser as a macOS application bundle."""
import argparse
from pathlib import Path
import plistlib
import shutil


def package(binary, output):
    binary, output = Path(binary), Path(output)
    if not binary.is_file():
        raise ValueError('Browser executable is missing')
    executable = output / 'Contents/MacOS/cybos-browser'
    executable.parent.mkdir(parents=True, exist_ok=True)
    shutil.copyfile(binary, executable)
    executable.chmod(0o755)
    (output / 'Contents/Info.plist').write_bytes(plistlib.dumps({
        'CFBundleIdentifier': 'to.cicada.cybos.browser',
        'CFBundleName': 'CybBrowser',
        'CFBundleDisplayName': 'CybBrowser',
        'CFBundleExecutable': 'cybos-browser',
        'CFBundlePackageType': 'APPL',
        'LSMinimumSystemVersion': '11.0',
        'NSHighResolutionCapable': True,
    }))


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary', required=True)
    parser.add_argument('--output', required=True)
    args = parser.parse_args()
    package(args.binary, args.output)
