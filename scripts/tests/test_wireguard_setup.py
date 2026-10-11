import base64
import importlib.util
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

spec = importlib.util.spec_from_file_location('wg_setup', Path(__file__).resolve().parents[1] / 'prepare_wireguard_server.py')
setup = importlib.util.module_from_spec(spec)
spec.loader.exec_module(setup)
KEY = base64.b64encode(bytes([7]) * 32).decode()
SERVER = base64.b64encode(bytes([9]) * 32).decode()


class WireGuardSetupTests(unittest.TestCase):
    def test_invalid_addresses_keys_and_reserved_server_address(self):
        for address in ['10.77.0.1/32', '10.77.0.0/32', '10.77.0.255/32', '10.78.0.2/32', '10.77.0.2/24']:
            with self.assertRaises(ValueError):
                setup.validate('203.0.113.10:51820', address, KEY)
        for endpoint in ['0.0.0.0:51820', '203.0.113.10:0', 'hostname:51820', '203.0.113.10:51820\nPostUp=evil']:
            with self.assertRaises(ValueError):
                setup.validate(endpoint, '10.77.0.2/32', KEY)
        with self.assertRaises(ValueError):
            setup.validate('203.0.113.10:51820', '10.77.0.2/32', base64.b64encode(bytes(32)).decode())

    def test_private_permissions_public_invitation_and_no_overwrite(self):
        with tempfile.TemporaryDirectory() as tmp, patch.object(setup, 'wg_run', side_effect=[SERVER, KEY]) as run:
            output = Path(tmp) / 'private'
            # Use a different client public key so the fixture server key cannot match it.
            client = base64.b64encode(bytes([8]) * 32).decode()
            invitation = setup.prepare(output, '203.0.113.10:51820', '10.77.0.2/32', client)
            self.assertEqual(output.stat().st_mode & 0o777, 0o700)
            self.assertEqual((output / 'cybos.conf').stat().st_mode & 0o777, 0o600)
            self.assertNotIn('PrivateKey', (output / 'device-invitation.json').read_text())
            self.assertNotIn(SERVER, (output / 'device-invitation.json').read_text())
            self.assertEqual(invitation['server_key'], KEY)
            before = (output / 'cybos.conf').read_bytes()
            with self.assertRaises(ValueError):
                setup.prepare(output, '203.0.113.10:51820', '10.77.0.2/32', client)
            self.assertEqual((output / 'cybos.conf').read_bytes(), before)
            self.assertEqual(run.call_count, 2)

    def test_symlink_output_rejected(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / 'linked'
            path.symlink_to(Path(tmp) / 'missing')
            with self.assertRaises(ValueError):
                setup.prepare(path, '203.0.113.10:51820', '10.77.0.2/32', KEY)


if __name__ == '__main__':
    unittest.main()
