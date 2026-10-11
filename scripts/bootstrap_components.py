#!/usr/bin/env python3
"""Restore pinned ecosystem siblings without modifying existing checkouts."""
import argparse
import json
from pathlib import Path
import re
import subprocess
import tempfile


def validate_rows(rows):
    names = set()
    for row in rows:
        name, commit = row.get('name'), row.get('commit')
        if (not isinstance(name, str) or not isinstance(commit, str)
                or not re.fullmatch(r'[A-Za-z0-9_][A-Za-z0-9_.-]*', name)
                or not re.fullmatch(r'[0-9a-f]{40}', commit)
                or name.casefold() in names):
            raise ValueError('invalid or duplicate pinned repository')
        names.add(name.casefold())


def verify_checkout(target, commit):
    if target.is_symlink() or not (target / '.git').exists():
        raise ValueError(f'{target.name}: existing directory is not a Git checkout')
    actual = subprocess.check_output(
        ['git', '-C', str(target), 'rev-parse', 'HEAD'], text=True).strip()
    if actual != commit:
        raise ValueError(f'{target.name}: checkout differs from lock; local changes preserved')


def bootstrap(root, rows, verify=False):
    rows = list(rows)
    validate_rows(rows)
    root = Path(root).resolve()
    # Validate all existing siblings before downloading any missing component.
    for row in rows:
        target = root / row['name']
        if target.exists() or target.is_symlink():
            verify_checkout(target, row['commit'])
        elif verify:
            raise ValueError(f"{row['name']}: checkout is missing")
    if not verify:
        root.mkdir(parents=True, exist_ok=True)
    for row in rows:
        name, commit = row['name'], row['commit']
        target = root / name
        if target.exists():
            print(f'VERIFIED {name} {commit}', flush=True)
            continue
        # A failed fetch/checkout leaves no partial sibling; reruns can retry.
        with tempfile.TemporaryDirectory(prefix=f'.{name}-', dir=root) as temporary:
            staging = Path(temporary) / 'checkout'
            subprocess.run(['git', 'init', str(staging)], check=True)
            subprocess.run(['git', '-C', str(staging), 'remote', 'add', 'origin',
                            f'https://github.com/c1cad4/{name}.git'], check=True)
            subprocess.run(['git', '-C', str(staging), 'fetch', '--depth=1',
                            'origin', commit], check=True)
            subprocess.run(['git', '-C', str(staging), 'checkout', '--detach',
                            commit], check=True)
            verify_checkout(staging, commit)
            # Refuse to replace a directory another setup process created.
            if target.exists() or target.is_symlink():
                raise ValueError(f'{name}: destination appeared during setup; preserved')
            staging.rename(target)
        print(f'CLONED {name} {commit}', flush=True)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--root', type=Path, default=Path(__file__).resolve().parents[2])
    parser.add_argument('--verify', action='store_true')
    args = parser.parse_args()
    rows = json.loads((Path(__file__).resolve().parents[1] / 'components.lock.json').read_text())['repositories']
    bootstrap(args.root.resolve(), rows, args.verify)


if __name__ == '__main__':
    main()
