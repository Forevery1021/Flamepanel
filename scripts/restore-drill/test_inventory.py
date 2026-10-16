import argparse
import contextlib
import hashlib
import json
import os
from pathlib import Path
import sqlite3
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

import inventory


class InventoryTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name)
        self.assets = self.root / 'assets'
        self.assets.mkdir(mode=0o700)
        self.source = self.root / 'backup.db'
        with contextlib.closing(sqlite3.connect(self.source)) as conn:
            conn.executescript('''
                CREATE TABLE users(id INTEGER PRIMARY KEY, username TEXT, password_hash TEXT, role TEXT);
                CREATE TABLE permissions(id INTEGER PRIMARY KEY, resource TEXT, action TEXT);
                CREATE TABLE panel_settings(key TEXT, value TEXT);
                CREATE TABLE backup_accounts(config_json TEXT);
                CREATE TABLE dns_provider_accounts(credentials_json TEXT);
                CREATE TABLE user_two_factor(secret TEXT, pending_secret TEXT);
                CREATE TABLE websites(id INTEGER); CREATE TABLE projects(id INTEGER);
                CREATE TABLE installed_apps(id INTEGER); CREATE TABLE databases(id INTEGER);
                CREATE TABLE nodes(id INTEGER); CREATE TABLE certificates(id INTEGER, source TEXT);
                INSERT INTO backup_accounts VALUES('enc:v1:not-a-real-secret');
            ''')
        self.original = self.source.read_bytes()
        self.secret = b'PRIVATE_KEY_MATERIAL_NEVER_READ'
        self.manifest = {'schema': inventory.SCHEMA, 'jwt_strategy': 'preserve', 'assets': []}
        for role in ['deployment_config', 'jwt_signing', 'encryption_master']:
            path = self.assets / role
            path.write_bytes(self.secret)
            path.chmod(0o600)
            self.manifest['assets'].append({'role': role, 'path': role})
        self.manifest_path = self.root / 'manifest.json'
        self.args = argparse.Namespace(backup=str(self.source), assets_root=str(self.assets),
                                      manifest=str(self.manifest_path), out=str(self.root / 'report'),
                                      timeout=30, max_bytes=1024**2)

    def write_manifest(self):
        self.manifest_path.write_text(json.dumps(self.manifest))

    def run_case(self, status='REVIEW_REQUIRED'):
        self.write_manifest()
        report = inventory.run(self.args)
        self.assertEqual(report['status'], status, report)
        self.assertFalse(report['restore_ready'])
        self.assertFalse(report['key_match_verified'])
        self.assertFalse(report['secret_contents_read'])
        self.assertTrue(report['copies_removed'])
        for p in Path(self.args.out).iterdir():
            self.assertEqual(p.stat().st_mode & 0o777, 0o600)
            contents = p.read_text()
            for sensitive in [self.secret.decode(), hashlib.sha256(self.secret).hexdigest(),
                              str(self.assets), str(self.source), 'not-a-real-secret']:
                self.assertNotIn(sensitive, contents)
        return report

    def asset_status(self, report, role):
        return next(a['status'] for a in report['assets'] if a['role'] == role)

    def test_present_wrong_key_is_only_metadata_not_crypto_success(self):
        report = self.run_case()
        self.assertEqual(self.asset_status(report, 'encryption_master'), 'PRESENT_UNVERIFIED')
        self.assertEqual(self.source.read_bytes(), self.original)
        self.assertEqual((self.assets / 'encryption_master').read_bytes(), self.secret)

    def test_secret_inspection_never_reads_or_hashes_contents(self):
        fd = os.open(self.assets, os.O_RDONLY | os.O_DIRECTORY)
        try:
            with patch.object(os, 'read', side_effect=AssertionError('secret read')), \
                    patch.object(os, 'fdopen', side_effect=AssertionError('secret stream')), \
                    patch.object(Path, 'read_bytes', side_effect=AssertionError('secret bytes')), \
                    patch.object(Path, 'read_text', side_effect=AssertionError('secret text')):
                with patch.object(hashlib, 'sha256', side_effect=AssertionError('secret hash')):
                    self.assertEqual(inventory.inspect_asset(fd, {'role': 'encryption_master',
                                                                   'path': 'encryption_master'}),
                                     'PRESENT_UNVERIFIED')
        finally:
            os.close(fd)

    def test_missing_master_with_ciphertext_blocks(self):
        (self.assets / 'encryption_master').unlink()
        report = self.run_case('BLOCKED')
        self.assertEqual(self.asset_status(report, 'encryption_master'), 'MISSING')

    def test_undeclared_master_with_ciphertext_blocks(self):
        self.manifest['assets'] = [a for a in self.manifest['assets'] if a['role'] != 'encryption_master']
        report = self.run_case('BLOCKED')
        self.assertEqual(self.asset_status(report, 'encryption_master'), 'NOT_DECLARED')

    def test_empty_master_blocks(self):
        (self.assets / 'encryption_master').write_bytes(b'')
        report = self.run_case('BLOCKED')
        self.assertEqual(self.asset_status(report, 'encryption_master'), 'EMPTY_FILE')

    def test_broad_secret_permissions_block(self):
        (self.assets / 'encryption_master').chmod(0o644)
        report = self.run_case('BLOCKED')
        self.assertEqual(self.asset_status(report, 'encryption_master'), 'UNSAFE_PERMISSIONS')

    def test_root_permissions_block(self):
        self.assets.chmod(0o755)
        report = self.run_case('BLOCKED')
        self.assertEqual(report['error']['code'], 'ASSET_ROOT_PERMISSIONS')

    def test_final_symlink_refused(self):
        path = self.assets / 'encryption_master'
        path.unlink()
        path.symlink_to(self.assets / 'jwt_signing')
        report = self.run_case('BLOCKED')
        self.assertEqual(self.asset_status(report, 'encryption_master'), 'UNSAFE_OR_UNREADABLE_PATH')

    def test_parent_symlink_refused(self):
        (self.assets / 'link').symlink_to(self.assets, target_is_directory=True)
        self.manifest['assets'][2]['path'] = 'link/encryption_master'
        report = self.run_case('BLOCKED')
        self.assertEqual(self.asset_status(report, 'encryption_master'), 'UNSAFE_OR_UNREADABLE_PATH')

    def test_hardlink_refused(self):
        os.link(self.assets / 'encryption_master', self.root / 'alias')
        report = self.run_case('BLOCKED')
        self.assertEqual(self.asset_status(report, 'encryption_master'), 'HARDLINK_REFUSED')

    def test_fifo_refused_without_blocking(self):
        path = self.assets / 'encryption_master'
        path.unlink()
        os.mkfifo(path, 0o600)
        report = self.run_case('BLOCKED')
        self.assertEqual(self.asset_status(report, 'encryption_master'), 'WRONG_TYPE')

    def test_traversal_and_absolute_path_rejected(self):
        for value in ['../key', '/etc/passwd', 'a/../key', 'a//key', './key', 'a\\key', 'a\x7fkey', 'a\x85key']:
            with self.subTest(value=value):
                with self.assertRaises(inventory.drill.DrillError):
                    inventory.relative_parts(value)

    def test_duplicate_role_and_inline_secret_rejected(self):
        self.manifest['assets'].append(dict(self.manifest['assets'][0]))
        report = self.run_case('BLOCKED')
        self.assertEqual(report['error']['code'], 'MANIFEST_INVALID')

    def test_unknown_manifest_fields_not_echoed(self):
        self.manifest['secret'] = self.secret.decode()
        report = self.run_case('BLOCKED')
        self.assertEqual(report['error']['code'], 'MANIFEST_INVALID')

    def test_duplicate_json_keys_rejected(self):
        self.manifest_path.write_text('{"schema":"a","schema":"b"}')
        with self.assertRaisesRegex(inventory.drill.DrillError, 'MANIFEST_INVALID'):
            inventory.load_manifest(self.manifest_path)

    def test_jwt_reissue_is_plan_only_not_session_action(self):
        self.manifest['jwt_strategy'] = 'reissue'
        self.manifest['assets'] = [a for a in self.manifest['assets'] if a['role'] != 'jwt_signing']
        report = self.run_case()
        self.assertIn('approve_session_reissue_no_action_performed', report['manual_review'])
        self.assertEqual(self.source.read_bytes(), self.original)

    def test_jwt_preserve_requires_file(self):
        self.manifest['assets'] = [a for a in self.manifest['assets'] if a['role'] != 'jwt_signing']
        self.run_case('BLOCKED')

    def test_resource_presence_requires_external_assets(self):
        with contextlib.closing(sqlite3.connect(self.source)) as conn:
            conn.executescript("INSERT INTO websites VALUES(1); INSERT INTO nodes VALUES(1); INSERT INTO certificates VALUES(1,'letsencrypt');")
        report = self.run_case('BLOCKED')
        required = {a['role'] for a in report['assets'] if a['required_by_observation']}
        self.assertTrue({'website_data', 'agent_material', 'bootstrap_token', 'certificate_bundle', 'acme_account'} <= required)

    def test_empty_directory_does_not_claim_complete_backup(self):
        with contextlib.closing(sqlite3.connect(self.source)) as conn:
            conn.execute('INSERT INTO websites VALUES(1)')
            conn.commit()
        (self.assets / 'sites').mkdir(mode=0o700)
        self.manifest['assets'].append({'role': 'website_data', 'path': 'sites'})
        report = self.run_case()
        self.assertEqual(self.asset_status(report, 'website_data'), 'PRESENT_UNVERIFIED')
        self.assertIn('directory_contents_and_external_snapshots', report['manual_review'])

    def test_legacy_missing_table_is_not_no_ciphertext_proof(self):
        with contextlib.closing(sqlite3.connect(self.source)) as conn:
            conn.executescript('DELETE FROM backup_accounts; DROP TABLE user_two_factor;')
        self.manifest['assets'] = [a for a in self.manifest['assets'] if a['role'] != 'encryption_master']
        report = self.run_case('BLOCKED')
        self.assertFalse(report['observations']['cipher_scan_complete'])

    def test_unknown_cipher_version_requires_master(self):
        with contextlib.closing(sqlite3.connect(self.source)) as conn:
            conn.execute("UPDATE backup_accounts SET config_json='enc:v9:unknown'")
            conn.commit()
        (self.assets / 'encryption_master').unlink()
        self.run_case('BLOCKED')

    def test_non_text_cipher_column_is_coverage_gap(self):
        with contextlib.closing(sqlite3.connect(self.source)) as conn:
            conn.execute('UPDATE backup_accounts SET config_json=?', (b'enc:v1:blob',))
            conn.commit()
        report = self.run_case()
        self.assertFalse(report['observations']['cipher_scan_complete'])
        self.assertIn('backup_accounts.config_json.non_text', report['observations']['coverage_gaps'])

    def test_normal_column_named_virtual_is_not_a_virtual_table(self):
        with contextlib.closing(sqlite3.connect(self.source)) as conn:
            conn.execute('ALTER TABLE backup_accounts ADD COLUMN virtual_note TEXT')
            conn.commit()
        report = self.run_case()
        self.assertEqual(report['observations']['encrypted_fields']['backup_accounts.config_json'], 1)

    def test_view_is_coverage_gap_not_queried(self):
        with contextlib.closing(sqlite3.connect(self.source)) as conn:
            conn.executescript('DROP TABLE backup_accounts; CREATE VIEW backup_accounts AS SELECT no_such_function() AS config_json;')
        report = self.run_case()
        self.assertIn('backup_accounts.config_json', report['observations']['coverage_gaps'])

    def test_corrupt_database_cleans_private_copy(self):
        self.source.write_bytes(b'not sqlite')
        report = self.run_case('BLOCKED')
        self.assertEqual(report['error']['code'], 'NOT_SQLITE')
        self.assertEqual(self.source.read_bytes(), b'not sqlite')

    def test_wal_sidecar_is_rejected(self):
        Path(str(self.source) + '-wal').write_bytes(b'active')
        report = self.run_case('BLOCKED')
        self.assertEqual(report['error']['code'], 'LIVE_DATABASE_REFUSED')
        self.assertEqual(self.source.read_bytes(), self.original)

    def test_manifest_symlink_and_fifo_refused(self):
        self.manifest_path.symlink_to(self.assets / 'encryption_master')
        with self.assertRaises(OSError):
            inventory.load_manifest(self.manifest_path)
        self.manifest_path.unlink()
        os.mkfifo(self.manifest_path, 0o600)
        with self.assertRaises(inventory.drill.DrillError):
            inventory.load_manifest(self.manifest_path)

    def test_manifest_size_cap(self):
        self.manifest_path.write_bytes(b' ' * 65537)
        with self.assertRaises(inventory.drill.DrillError):
            inventory.load_manifest(self.manifest_path)

    def test_duplicate_path_rejected(self):
        self.manifest['assets'][1]['path'] = self.manifest['assets'][0]['path']
        self.run_case('BLOCKED')

    def test_interrupted_snapshot_cleans_private_copy(self):
        def interrupt(args, staged, deadline):
            staged.write_bytes(b'incomplete')
            raise KeyboardInterrupt
        with patch.object(inventory.drill, 'acquire', side_effect=interrupt):
            report = self.run_case('BLOCKED')
        self.assertEqual(report['error']['code'], 'INTERRUPTED')

    def test_expired_inventory_deadline_does_not_open_assets(self):
        observations = {'encrypted_fields': {}, 'cipher_scan_complete': True,
                        'resource_counts': {}, 'acme_records': 0}
        with patch.object(inventory, 'inspect_asset') as inspect:
            with self.assertRaises(inventory.drill.DrillError):
                inventory.evaluate(self.manifest, observations, 0, 0)
            inspect.assert_not_called()

    def test_existing_report_is_not_overwritten(self):
        self.write_manifest()
        Path(self.args.out).mkdir()
        with self.assertRaises(FileExistsError):
            inventory.run(self.args)

    def test_cli_exit_and_input_content_unchanged(self):
        self.write_manifest()
        command = [sys.executable, str(Path(inventory.__file__)), '--backup', str(self.source),
                   '--manifest', str(self.manifest_path), '--assets-root', str(self.assets), '--out', self.args.out]
        result = subprocess.run(command, capture_output=True, text=True, timeout=10)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(result.stdout.strip(), 'RECOVERY_INVENTORY: REVIEW_REQUIRED')
        self.assertEqual(self.source.read_bytes(), self.original)


if __name__ == '__main__':
    unittest.main()
