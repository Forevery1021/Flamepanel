import argparse
import contextlib
import datetime as dt
import errno
import io
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
import urllib.error

import drill


class RestoreDrillTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name)
        self.source = self.root / 'snapshot.db'
        with contextlib.closing(sqlite3.connect(self.source)) as conn:
            conn.executescript('''
                CREATE TABLE users(id INTEGER PRIMARY KEY, username TEXT, password_hash TEXT, role TEXT);
                CREATE TABLE permissions(id INTEGER PRIMARY KEY, resource TEXT, action TEXT);
                CREATE TABLE panel_settings(key TEXT PRIMARY KEY, value TEXT);
                INSERT INTO users VALUES(1, 'private-name', 'secret-hash', 'admin');
                INSERT INTO permissions VALUES(1, 'backup', 'read');
                INSERT INTO panel_settings VALUES('private-setting', 'secret-value');
            ''')
        self.before = self.source.read_bytes()
        self.args = argparse.Namespace(backup=str(self.source), url=None,
                                      out=str(self.root / 'report'), max_bytes=1024**2,
                                      timeout=30, backup_time=None, sha256=None)

    def run_case(self, expected='FAIL', code=None):
        report = drill.run(self.args)
        self.assertEqual(report['status'], expected)
        self.assertTrue(report['copies_removed'])
        self.assertFalse(list(Path(self.args.out).glob('.isolated-*')))
        if code:
            self.assertIn(code, [stage.get('code') for stage in report['stages'].values()])
        for file in Path(self.args.out).iterdir():
            self.assertEqual(file.stat().st_mode & 0o777, 0o600)
            text = file.read_text()
            for secret in ['private-name', 'secret-hash', 'secret-value', str(self.source), 'url-secret']:
                self.assertNotIn(secret, text)
        self.assertEqual(Path(self.args.out).stat().st_mode & 0o777, 0o700)
        return report

    def test_normal_restores_reopens_and_samples_without_source_changes(self):
        report = self.run_case('PASS')
        self.assertEqual(self.source.read_bytes(), self.before)
        self.assertEqual(report['sampled_rows'], {'users': 1, 'permissions': 1, 'panel_settings': 1})
        self.assertTrue(all(s['status'] == 'PASS' for s in report['stages'].values()))
        self.assertIsNone(report['rpo_seconds'])
        self.assertIsNone(report['production_rto_seconds'])
        self.assertFalse(report['kernel_started'])
        self.assertGreaterEqual(report['storage_rto_seconds'], 0)

    def test_trusted_checksum_match(self):
        self.args.sha256 = hashlib.sha256(self.before).hexdigest()
        report = self.run_case('PASS')
        self.assertTrue(report['checksum_verified'])

    def test_trusted_checksum_mismatch_stops_restore(self):
        self.args.sha256 = '0' * 64
        report = self.run_case(code='CHECKSUM_MISMATCH')
        self.assertEqual(report['stages']['restore']['status'], 'NOT_RUN')

    def test_remote_unknown_length_cannot_exceed_size_limit(self):
        self.args.backup = None
        self.args.url = 'https://example.invalid/snapshot'
        self.args.max_bytes = 100
        response = io.BytesIO(self.before)
        response.status, response.headers = 200, {}
        with patch.object(drill.urllib.request.OpenerDirector, 'open', return_value=response):
            self.run_case(code='SIZE_LIMIT')

    def test_copy_mismatch_stops_reopen(self):
        original = drill.copy_stream
        def wrong_digest(source, destination, *args):
            size, digest = original(source, destination, *args)
            return size, '0' * 64 if destination.name == 'restored.tmp' else digest
        with patch.object(drill, 'copy_stream', side_effect=wrong_digest):
            report = self.run_case(code='COPY_MISMATCH')
        self.assertEqual(report['stages']['reopen']['status'], 'NOT_RUN')

    def test_corrupt_header_stops_before_restore(self):
        self.source.write_bytes(b'not a database')
        report = self.run_case(code='NOT_SQLITE')
        self.assertEqual(report['stages']['restore']['status'], 'NOT_RUN')

    def test_corrupt_page_rejected(self):
        self.source.write_bytes(self.before[:100])
        self.run_case(code='SQLITE_CHECK_FAILED')

    def test_wrong_schema_rejected(self):
        with contextlib.closing(sqlite3.connect(self.source)) as conn:
            conn.execute('DROP TABLE permissions')
            conn.commit()
        self.run_case(code='SCHEMA_MISMATCH')

    def test_view_is_not_accepted_as_core_table(self):
        with contextlib.closing(sqlite3.connect(self.source)) as conn:
            conn.executescript('DROP TABLE permissions; CREATE VIEW permissions AS SELECT 1 AS id;')
        self.run_case(code='SCHEMA_MISMATCH')

    def test_capacity_preflight(self):
        with patch.object(drill.shutil, 'disk_usage', return_value=argparse.Namespace(free=0)):
            self.run_case(code='INSUFFICIENT_SPACE')
        self.assertEqual(self.source.read_bytes(), self.before)

    def test_enospc_during_restore_is_reported_and_copies_removed(self):
        original = drill.copy_stream
        def fail_restore(source, destination, *args):
            if destination.name == 'restored.tmp':
                destination.write_bytes(b'partial')
                raise OSError(errno.ENOSPC, 'secret path must not be logged')
            return original(source, destination, *args)
        with patch.object(drill, 'copy_stream', side_effect=fail_restore):
            report = self.run_case(code='INSUFFICIENT_SPACE')
        self.assertEqual(report['stages']['restore']['status'], 'FAIL')

    def test_remote_unavailable_does_not_leak_url(self):
        self.args.backup = None
        self.args.url = 'https://example.invalid/backup?token=url-secret'
        with patch.object(drill.urllib.request.OpenerDirector, 'open',
                          side_effect=urllib.error.URLError(self.args.url)):
            self.run_case(code='SOURCE_UNAVAILABLE')

    def test_remote_success_and_unknown_length_limit(self):
        self.args.backup = None
        self.args.url = 'https://example.invalid/snapshot'
        response = io.BytesIO(self.before)
        response.status, response.headers = 200, {}
        with patch.object(drill.urllib.request.OpenerDirector, 'open', return_value=response):
            self.run_case('PASS')

    def test_remote_truncated(self):
        self.args.backup = None
        self.args.url = 'https://example.invalid/snapshot'
        response = io.BytesIO(self.before)
        response.status, response.headers = 200, {'Content-Length': str(len(self.before) + 1)}
        with patch.object(drill.urllib.request.OpenerDirector, 'open', return_value=response):
            self.run_case(code='REMOTE_TRUNCATED')

    def test_redirect_policy(self):
        with self.assertRaisesRegex(drill.DrillError, 'REDIRECT_REFUSED'):
            drill.NoRedirect().redirect_request(None, None, 302, '', {}, 'https://other.invalid')

    def test_unsafe_scheme_and_embedded_credentials(self):
        self.args.backup = None
        for i, url in enumerate(['http://example.invalid', 'file:///etc/passwd',
                                 'https://user:pass@example.invalid/']):
            with self.subTest(url=url):
                self.args.out = str(self.root / f'report-{i}')
                self.args.url = url
                self.run_case(code='HTTPS_SOURCE_REQUIRED')

    def test_source_size_limit(self):
        self.args.max_bytes = 1
        self.run_case(code='SIZE_LIMIT')

    def test_wal_source_refused(self):
        Path(str(self.source) + '-wal').touch()
        self.run_case(code='LIVE_DATABASE_REFUSED')

    def test_symlink_refused_without_touching_target(self):
        link = self.root / 'link.db'
        link.symlink_to(self.source)
        self.args.backup = str(link)
        self.run_case(code='SOURCE_UNAVAILABLE')
        self.assertEqual(self.source.read_bytes(), self.before)

    def test_fifo_refused_without_blocking(self):
        fifo = self.root / 'pipe'
        os.mkfifo(fifo)
        self.args.backup = str(fifo)
        self.run_case(code='REGULAR_FILE_REQUIRED')

    def test_existing_output_never_overwritten(self):
        output = Path(self.args.out)
        output.mkdir()
        sentinel = output / 'report.json'
        sentinel.write_text('existing')
        with self.assertRaises(FileExistsError):
            drill.run(self.args)
        self.assertEqual(sentinel.read_text(), 'existing')

    def test_expired_deadline(self):
        self.args.timeout = -1
        self.run_case(code='TIMEOUT')

    def test_interruption_cleans_copies(self):
        with patch.object(drill, 'validate', side_effect=InterruptedError):
            self.run_case(code='INTERRUPTED')

    def test_declared_backup_time_not_reported_as_rpo(self):
        self.args.backup_time = dt.datetime.now(dt.timezone.utc) - dt.timedelta(minutes=10)
        report = self.run_case('PASS')
        self.assertGreaterEqual(report['backup_age_seconds'], 600)
        self.assertIsNone(report['rpo_seconds'])

    def test_cli_exit_codes_and_machine_report(self):
        entry = Path(drill.__file__)
        result = subprocess.run([sys.executable, str(entry), '--backup', str(self.source),
                                 '--out', self.args.out], capture_output=True, text=True, timeout=10)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(json.loads((Path(self.args.out) / 'report.json').read_text())['status'], 'PASS')
        result = subprocess.run([sys.executable, str(entry), '--backup', str(self.source),
                                 '--out', self.args.out], capture_output=True, text=True, timeout=10)
        self.assertEqual(result.returncode, 2)


if __name__ == '__main__':
    unittest.main()
