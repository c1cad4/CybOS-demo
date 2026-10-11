import base64
from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import patch

SCRIPTS = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPTS))
import add_wireguard_peer as enrollment


def key(byte):
    return base64.b64encode(bytes([byte]) * 32).decode()


class EnrollmentTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.source = Path(self.tmp.name) / 'source'
        self.source.mkdir(mode=0o700)
        self.config = f'[Interface]\nPrivateKey = {key(1)}\nAddress = 10.77.0.1/24\nListenPort = 51820\n\n[Peer]\nPublicKey = {key(2)}\nAllowedIPs = 10.77.0.2/32\n'
        (self.source / 'cybos.conf').write_text(self.config)
        (self.source / 'cybos.conf').chmod(0o600)
        self.output = Path(self.tmp.name) / 'next'

    def enroll(self, address='10.77.0.3/32', device_key=None):
        return enrollment.enroll(self.source, self.output, '203.0.113.10:51820', address, device_key or key(3))

    @patch.object(enrollment, 'wg_run', return_value=key(4))
    def test_second_and_third_device_preserve_server_and_previous_peers(self, run):
        invite = self.enroll()
        self.assertEqual((self.source / 'cybos.conf').read_text(), self.config)
        updated = (self.output / 'cybos.conf').read_text()
        self.assertIn(f'PrivateKey = {key(1)}', updated)
        self.assertEqual(updated.count('[Peer]'), 2)
        self.assertEqual(invite['server_key'], key(4))
        self.assertNotIn(key(1), (self.output / 'device-invitation.json').read_text())
        self.assertEqual(self.output.stat().st_mode & 0o777, 0o700)
        self.assertEqual((self.output / 'cybos.conf').stat().st_mode & 0o777, 0o600)
        third = Path(self.tmp.name) / 'third'
        enrollment.enroll(self.output, third, '203.0.113.10:51820', '10.77.0.4/32', key(5))
        self.assertEqual((third / 'cybos.conf').read_text().count('[Peer]'), 3)
        run.assert_called_with('wg', ['pubkey'], key(1) + '\n')

    @patch.object(enrollment, 'wg_run', return_value=key(4))
    def test_duplicates_server_key_and_existing_output_rejected(self, run):
        for address, device in [('10.77.0.2/32', key(3)), ('10.77.0.3/32', key(2)), ('10.77.0.3/32', key(4))]:
            with self.assertRaises(ValueError):
                self.enroll(address, device)
            self.assertFalse(self.output.exists())
        self.enroll()
        before = (self.output / 'cybos.conf').read_bytes()
        with self.assertRaises(ValueError):
            self.enroll('10.77.0.4/32', key(5))
        self.assertEqual((self.output / 'cybos.conf').read_bytes(), before)

    def test_permissive_secret_storage_rejected(self):
        (self.source / 'cybos.conf').chmod(0o644)
        with self.assertRaises(ValueError):
            self.enroll()
        (self.source / 'cybos.conf').chmod(0o600)
        self.source.chmod(0o755)
        with self.assertRaises(ValueError):
            self.enroll()
        self.assertFalse(self.output.exists())

    def test_unknown_settings_malformed_duplicate_source_and_symlink_rejected(self):
        for text in [self.config + 'PostUp = unsafe\n', self.config + '[Peer]\nPublicKey = ' + key(2) + '\nAllowedIPs = 10.77.0.3/32\n', self.config.replace('ListenPort = 51820', 'ListenPort = 51821'), '[Interface]\nPrivateKey = invalid\n']:
            (self.source / 'cybos.conf').write_text(text)
            with self.assertRaises(ValueError):
                self.enroll()
            self.assertFalse(self.output.exists())
        (self.source / 'cybos.conf').unlink()
        (self.source / 'cybos.conf').symlink_to(Path(self.tmp.name) / 'missing')
        with self.assertRaises(ValueError):
            self.enroll()


if __name__ == '__main__':
    unittest.main()
