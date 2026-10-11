import importlib.util
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import patch

spec = importlib.util.spec_from_file_location('bootstrap', Path(__file__).parents[1] / 'bootstrap_components.py')
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)


class BootstrapTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.rows = [{'name': 'component', 'commit': 'a' * 40}]

    def checkout(self):
        path = self.root / 'component'
        path.mkdir()
        (path / '.git').mkdir()
        (path / 'local.rs').write_text('local work')
        return path

    def test_invalid_later_row_does_not_start_download(self):
        for name in ('../escape', '-option', '.', '..'):
            with self.subTest(name=name), patch.object(module.subprocess, 'run') as run:
                with self.assertRaises(ValueError):
                    module.bootstrap(self.root, self.rows + [{'name': name, 'commit': 'b' * 40}])
                run.assert_not_called()

    def test_duplicate_names_rejected_before_download(self):
        with patch.object(module.subprocess, 'run') as run:
            with self.assertRaises(ValueError):
                module.bootstrap(self.root, self.rows * 2)
            run.assert_not_called()

    def test_case_collisions_and_non_string_pins_are_rejected(self):
        for rows in (self.rows + [{'name': 'COMPONENT', 'commit': 'b' * 40}],
                     [{'name': None, 'commit': 'a' * 40}],
                     [{'name': 'component', 'commit': None}]):
            with self.subTest(rows=rows), self.assertRaises(ValueError):
                module.bootstrap(self.root, rows)

    def test_existing_wrong_revision_preserves_work_and_fails(self):
        path = self.checkout()
        with patch.object(module.subprocess, 'check_output', return_value='b' * 40), patch.object(module.subprocess, 'run') as run:
            with self.assertRaisesRegex(ValueError, 'differs from lock'):
                module.bootstrap(self.root, self.rows)
            run.assert_not_called()
        self.assertEqual((path / 'local.rs').read_text(), 'local work')

    def test_matching_revision_preserves_dirty_work(self):
        path = self.checkout()
        with patch.object(module.subprocess, 'check_output', return_value='a' * 40), patch.object(module.subprocess, 'run') as run:
            module.bootstrap(self.root, self.rows, verify=True)
            run.assert_not_called()
        self.assertEqual((path / 'local.rs').read_text(), 'local work')

    def test_verify_is_offline_and_does_not_create_missing_root(self):
        missing = self.root / 'missing'
        with patch.object(module.subprocess, 'run') as run:
            with self.assertRaisesRegex(ValueError, 'missing'):
                module.bootstrap(missing, self.rows, verify=True)
            run.assert_not_called()
        self.assertFalse(missing.exists())

    def test_failed_fetch_cleans_staging_and_can_be_retried(self):
        def fail_fetch(args, **kwargs):
            if args[1] == 'init':
                Path(args[2]).mkdir()
            if 'fetch' in args:
                raise subprocess.CalledProcessError(1, args)
        for _ in range(2):
            with patch.object(module.subprocess, 'run', side_effect=fail_fetch):
                with self.assertRaises(subprocess.CalledProcessError):
                    module.bootstrap(self.root, self.rows)
            self.assertEqual(list(self.root.iterdir()), [])

    def test_successful_fetch_publishes_only_verified_checkout(self):
        def init(args, **kwargs):
            if args[1] == 'init':
                path = Path(args[2])
                path.mkdir()
                (path / '.git').mkdir()
        with patch.object(module.subprocess, 'run', side_effect=init), patch.object(module.subprocess, 'check_output', return_value='a' * 40):
            module.bootstrap(self.root, self.rows)
        self.assertTrue((self.root / 'component' / '.git').is_dir())
        self.assertEqual([p.name for p in self.root.iterdir()], ['component'])

    def test_symlink_destination_is_rejected(self):
        destination = self.root / 'component'
        destination.symlink_to(self.root / 'not-created')
        with patch.object(module.subprocess, 'run') as run:
            with self.assertRaisesRegex(ValueError, 'not a Git checkout'):
                module.bootstrap(self.root, self.rows)
            run.assert_not_called()
        self.assertTrue(destination.is_symlink())


if __name__ == '__main__':
    unittest.main()
