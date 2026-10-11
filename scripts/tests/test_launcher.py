import os
from pathlib import Path
import plistlib
import shutil
import subprocess
import tempfile
import unittest


class LauncherTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.app = self.root / 'desktop'
        self.app.mkdir()
        for name in ('src', 'macos', 'scripts'):
            (self.app / name).mkdir()
        shutil.copyfile(Path(__file__).parents[2] / 'START_cybOS.command', self.app / 'START_cybOS.command')
        (self.app / 'Cargo.toml').write_text('[package]\nversion = "9.8.7"\n')
        (self.app / 'Cargo.lock').write_text('lock')
        (self.app / 'components.lock.json').write_text('{"repositories":[{"name":"library"}]}')
        (self.app / 'scripts/bootstrap_components.py').write_text(
            'from pathlib import Path\nPath("setup-ran").touch()\n')
        library = self.root / 'library'
        library.mkdir()
        (library / 'lib.rs').write_text('// source')
        self.commands = self.root / 'commands'
        self.commands.mkdir()
        cargo = self.commands / 'cargo'
        cargo.write_text('#!/bin/bash\necho "$*" >> cargo-ran\nmkdir -p target/release\nprintf "binary" > target/release/cybos\nchmod +x target/release/cybos\n')
        cargo.chmod(0o755)
        for name in ('open', 'xattr'):
            stub = self.commands / name
            stub.write_text('#!/bin/bash\nexit 0\n')
            stub.chmod(0o755)
        swift = self.commands / 'swiftc'
        swift.write_text('#!/bin/bash\nwhile [ "$#" -gt 0 ]; do\n  if [ "$1" = "-o" ]; then shift; touch "$1"; exit 0; fi\n  shift\ndone\nexit 1\n')
        swift.chmod(0o755)
        self.env = dict(os.environ, PATH=f'{self.commands}:{os.environ["PATH"]}')

    def launch(self):
        return subprocess.run(['bash', str(self.app / 'START_cybOS.command')], env=self.env, capture_output=True, text=True)

    def existing_binary(self):
        binary = self.app / 'target/release/cybos'
        binary.parent.mkdir(parents=True)
        binary.write_text('old binary')
        binary.chmod(0o755)
        for root in (self.app, self.root / 'library'):
            for path in root.rglob('*'):
                if path.is_file():
                    os.utime(path, (100, 100))
        os.utime(binary, (200, 200))

    def test_fresh_clone_restores_components_builds_locked_and_uses_package_version(self):
        result = self.launch()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertTrue((self.app / 'setup-ran').exists())
        self.assertEqual((self.app / 'cargo-ran').read_text().strip(), 'build --locked --release')
        with (self.app / 'cybOS.app/Contents/Info.plist').open('rb') as file:
            info = plistlib.load(file)
        self.assertEqual(info['CFBundleVersion'], '9.8.7')
        self.assertEqual(info['CFBundleShortVersionString'], '9.8.7')

    def test_unchanged_binary_skips_build(self):
        self.existing_binary()
        result = self.launch()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertFalse((self.app / 'cargo-ran').exists())

    def test_changed_sibling_rebuilds_binary(self):
        self.existing_binary()
        os.utime(self.root / 'library/lib.rs', (300, 300))
        result = self.launch()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertTrue((self.app / 'cargo-ran').exists())

    def test_setup_failure_stops_before_build_or_bundle_replacement(self):
        (self.app / 'scripts/bootstrap_components.py').write_text('raise SystemExit(1)\n')
        bundle = self.app / 'cybOS.app'
        bundle.mkdir()
        (bundle / 'existing').write_text('preserve')
        result = self.launch()
        self.assertNotEqual(result.returncode, 0)
        self.assertFalse((self.app / 'cargo-ran').exists())
        self.assertEqual((bundle / 'existing').read_text(), 'preserve')


if __name__ == '__main__':
    unittest.main()
