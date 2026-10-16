import argparse
import base64
import contextlib
import hashlib
import json
import os
from pathlib import Path
import signal
import sqlite3
import subprocess
import sys
import tempfile
import time
import unittest
from unittest.mock import patch

import keycheck


class KeycheckTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name)
        self.assets = self.root / 'assets'
        self.assets.mkdir(mode=0o700)
        self.master = self.assets / 'master'
        self.secret = b'SYNTHETIC_PRIVATE_MASTER_CANARY'
        self.master.write_bytes(self.secret)
        self.master.chmod(0o600)
        self.source = self.root / 'snapshot.db'
        with contextlib.closing(sqlite3.connect(self.source)) as conn:
            conn.executescript('''
                CREATE TABLE users(id INTEGER, username TEXT, password_hash TEXT, role TEXT);
                CREATE TABLE permissions(id INTEGER, resource TEXT, action TEXT);
                CREATE TABLE panel_settings(key TEXT, value TEXT);
                CREATE TABLE backup_accounts(config_json TEXT);
                CREATE TABLE dns_provider_accounts(credentials_json TEXT);
                CREATE TABLE user_two_factor(secret TEXT, pending_secret TEXT);
                INSERT INTO backup_accounts VALUES('enc:v1:PRIVATE_CIPHERTEXT_CANARY');
            ''')
        self.binary = self.make_worker('import sys\nsys.exit(0)\n')
        self.args = argparse.Namespace(backup=str(self.source), assets_root=str(self.assets), key_path='master',
            keycheck_bin=str(self.binary), allow_secret_read=True, out=str(self.root / 'report'),
            max_bytes=4 * 1024**2, timeout=10, sha256=None)

    def make_worker(self, body):
        binary = self.root / 'fp-restore-keycheck'
        binary.write_text('#!' + sys.executable + '\n' + body)
        binary.chmod(0o700)
        return binary

    def execute(self, sql, params=()):
        with contextlib.closing(sqlite3.connect(self.source)) as conn:
            conn.execute(sql, params)
            conn.commit()

    def run_case(self, expected='MATCHED_WITHIN_SCOPE', rejected=0):
        original = self.source.read_bytes()
        def fake_worker(binary, master, values, work, deadline):
            self.assertEqual(master, self.secret)
            self.assertTrue(values)
            return {'schema': keycheck.WORKER_SCHEMA, 'checked': len(values),
                    'authenticated': len(values) - rejected, 'rejected': rejected}
        with patch.object(keycheck, 'invoke_worker', side_effect=fake_worker) as worker:
            report = keycheck.run(self.args)
        self.assertEqual(report['status'], expected, report)
        self.assertFalse(report['restore_ready'])
        self.assertFalse(report['production_auth_verified'])
        self.assertFalse(report['kernel_started'])
        self.assertTrue(report['copies_removed'])
        self.assertEqual(self.source.read_bytes(), original)
        self.assertEqual(Path(self.args.out).stat().st_mode & 0o777, 0o700)
        for file in Path(self.args.out).iterdir():
            self.assertEqual(file.stat().st_mode & 0o777, 0o600)
            text = file.read_text()
            for forbidden in [self.secret.decode(), base64.b64encode(self.secret).decode(),
                              hashlib.sha256(self.secret).hexdigest(), str(self.assets), str(self.source),
                              'PRIVATE_CIPHERTEXT_CANARY']:
                self.assertNotIn(forbidden, text)
        return report, worker.call_count

    def test_authorized_success_does_not_emit_values_or_modify_key(self):
        report, calls = self.run_case()
        self.assertTrue(report['secret_contents_read'])
        self.assertTrue(report['key_match_verified'])
        self.assertEqual(calls, 1)
        self.assertEqual(self.master.read_bytes(), self.secret)

    def test_missing_explicit_authorization_never_reads_inputs_or_calls_worker(self):
        self.args.allow_secret_read = False
        with patch.object(keycheck, 'read_master', side_effect=AssertionError('secret read')), \
                patch.object(keycheck.drill, 'acquire', side_effect=AssertionError('input read')):
            report, calls = self.run_case('BLOCKED')
        self.assertEqual(report['error']['code'], 'SECRET_READ_NOT_AUTHORIZED')
        self.assertFalse(report['secret_contents_read'])
        self.assertEqual(calls, 0)

    def test_non_boolean_authorization_is_not_explicit_consent(self):
        self.args.allow_secret_read = 'yes'
        report, calls = self.run_case('BLOCKED')
        self.assertFalse(report['secret_read_authorized'])
        self.assertFalse(report['secret_contents_read'])
        self.assertEqual(calls, 0)

    def test_missing_key_is_not_generated(self):
        self.master.unlink()
        report, calls = self.run_case('BLOCKED')
        self.assertFalse(self.master.exists())
        self.assertFalse(report['secret_contents_read'])
        self.assertEqual(calls, 0)

    def test_empty_key_is_not_replaced_or_read(self):
        self.master.write_bytes(b'')
        report, calls = self.run_case('BLOCKED')
        self.assertEqual(self.master.read_bytes(), b'')
        self.assertFalse(report['secret_contents_read'])
        self.assertEqual(calls, 0)

    def test_active_trigger_is_refused(self):
        self.execute('CREATE TRIGGER trigger_canary AFTER INSERT ON backup_accounts BEGIN DELETE FROM backup_accounts; END')
        report, _ = self.run_case('BLOCKED')
        self.assertEqual(report['error']['code'], 'ACTIVE_SCHEMA_REFUSED')
        self.assertFalse(report['secret_contents_read'])

    def test_virtual_schema_is_refused(self):
        self.execute('CREATE VIRTUAL TABLE virtual_canary USING fts5(contents)')
        report, _ = self.run_case('BLOCKED')
        self.assertEqual(report['error']['code'], 'ACTIVE_SCHEMA_REFUSED')
        self.assertFalse(report['secret_contents_read'])

    def test_rejected_ciphertext_is_not_a_verified_key(self):
        report, _ = self.run_case('NOT_VERIFIED', rejected=1)
        self.assertFalse(report['key_match_verified'])
        self.assertEqual(report['reason'], 'KEY_MISMATCH_OR_CORRUPT_CIPHERTEXT')

    def test_plaintext_only_never_proves_a_match_or_reads_the_key(self):
        self.execute("UPDATE backup_accounts SET config_json='PRIVATE_LEGACY_PLAINTEXT'")
        with patch.object(keycheck, 'read_master', side_effect=AssertionError('unnecessary key read')):
            report, calls = self.run_case('INCONCLUSIVE')
        self.assertFalse(report['secret_contents_read'])
        self.assertEqual(calls, 0)
        self.assertEqual(report['reason'], 'NO_SUPPORTED_CIPHERTEXT')
        self.assertNotIn('PRIVATE_LEGACY_PLAINTEXT', json.dumps(report))

    def test_unknown_version_and_valid_ciphertext_is_inconclusive_without_key_read(self):
        self.execute("INSERT INTO backup_accounts VALUES('enc:v9:UNKNOWN_CANARY')")
        report, calls = self.run_case('INCONCLUSIVE')
        self.assertFalse(report['secret_contents_read'])
        self.assertEqual(calls, 0)
        self.assertEqual(report['observations']['unsupported_ciphertexts'], 1)

    def test_missing_cipher_column_is_a_coverage_gap(self):
        self.execute('DROP TABLE user_two_factor')
        report, calls = self.run_case('INCONCLUSIVE')
        self.assertIn('user_two_factor.secret', report['observations']['coverage_gaps'])
        self.assertFalse(report['secret_contents_read'])
        self.assertEqual(calls, 0)

    def test_non_text_value_is_a_gap_not_a_silent_skip(self):
        self.execute('INSERT INTO backup_accounts VALUES(?)', (b'enc:v1:BLOB_CANARY',))
        report, _ = self.run_case('INCONCLUSIVE')
        self.assertIn('backup_accounts.config_json.non_text', report['observations']['coverage_gaps'])
        self.assertFalse(report['secret_contents_read'])

    def test_active_view_is_refused(self):
        self.execute('CREATE VIEW private_view AS SELECT 1')
        report, calls = self.run_case('BLOCKED')
        self.assertEqual(report['error']['code'], 'ACTIVE_SCHEMA_REFUSED')
        self.assertFalse(report['secret_contents_read'])
        self.assertEqual(calls, 0)

    def test_generated_cipher_column_is_not_executed_for_key_matching(self):
        self.execute('DROP TABLE dns_provider_accounts')
        self.execute("CREATE TABLE dns_provider_accounts(dummy TEXT, credentials_json TEXT GENERATED ALWAYS AS ('enc:v1:generated') VIRTUAL)")
        report, _ = self.run_case('INCONCLUSIVE')
        self.assertIn('dns_provider_accounts.credentials_json', report['observations']['coverage_gaps'])
        self.assertFalse(report['secret_contents_read'])

    def test_record_budget_is_inconclusive_before_key_read(self):
        with contextlib.closing(sqlite3.connect(self.source)) as conn:
            conn.executemany('INSERT INTO backup_accounts VALUES(?)', [('enc:v1:x',)] * keycheck.RECORD_LIMIT)
            conn.commit()
        report, _ = self.run_case('INCONCLUSIVE')
        self.assertIn('ciphertext_budget', report['observations']['coverage_gaps'])
        self.assertFalse(report['secret_contents_read'])

    def test_large_single_value_is_not_fetched_into_a_worker(self):
        self.execute('UPDATE backup_accounts SET config_json=?', ('enc:v1:' + 'x' * keycheck.VALUE_LIMIT,))
        report, _ = self.run_case('INCONCLUSIVE')
        self.assertIn('backup_accounts.config_json.value_limit', report['observations']['coverage_gaps'])
        self.assertFalse(report['secret_contents_read'])

    def test_aggregate_budget_is_not_partial_success(self):
        with contextlib.closing(sqlite3.connect(self.source)) as conn:
            conn.executemany('INSERT INTO backup_accounts VALUES(?)', [('enc:v1:' + 'x' * 65000,)] * 17)
            conn.commit()
        report, _ = self.run_case('INCONCLUSIVE')
        self.assertIn('ciphertext_budget', report['observations']['coverage_gaps'])
        self.assertFalse(report['secret_contents_read'])

    def test_key_and_root_permissions_rejected_before_read(self):
        self.master.chmod(0o644)
        report, _ = self.run_case('BLOCKED')
        self.assertEqual(report['error']['code'], 'KEY_PERMISSIONS')
        self.assertFalse(report['secret_contents_read'])

    def test_root_permissions_rejected(self):
        self.assets.chmod(0o755)
        report, _ = self.run_case('BLOCKED')
        self.assertEqual(report['error']['code'], 'ASSET_ROOT_PERMISSIONS')
        self.assertFalse(report['secret_contents_read'])

    def test_oversized_key_rejected(self):
        self.master.write_bytes(b'x' * (keycheck.MASTER_LIMIT + 1))
        report, _ = self.run_case('BLOCKED')
        self.assertEqual(report['error']['code'], 'KEY_SIZE_LIMIT')
        self.assertFalse(report['secret_contents_read'])

    def test_hardlinked_key_refused(self):
        os.link(self.master, self.root / 'key-alias')
        report, _ = self.run_case('BLOCKED')
        self.assertEqual(report['error']['code'], 'KEY_REGULAR_PRIVATE_FILE_REQUIRED')
        self.assertFalse(report['secret_contents_read'])

    def test_symlinked_key_refused(self):
        self.master.unlink()
        self.master.symlink_to(self.root / 'not-an-asset')
        report, calls = self.run_case('BLOCKED')
        self.assertFalse(report['secret_contents_read'])
        self.assertEqual(calls, 0)

    def test_parent_symlink_refused(self):
        (self.assets / 'link').symlink_to(self.assets, target_is_directory=True)
        self.args.key_path = 'link/master'
        report, calls = self.run_case('BLOCKED')
        self.assertFalse(report['secret_contents_read'])
        self.assertEqual(calls, 0)

    def test_fifo_key_refused_without_blocking(self):
        self.master.unlink()
        os.mkfifo(self.master, 0o600)
        report, _ = self.run_case('BLOCKED')
        self.assertFalse(report['secret_contents_read'])

    def test_traversal_key_path_refused(self):
        self.args.key_path = '../master'
        report, _ = self.run_case('BLOCKED')
        self.assertEqual(report['error']['code'], 'MANIFEST_INVALID')
        self.assertFalse(report['secret_contents_read'])

    def test_live_database_and_checksum_mismatch_do_not_read_the_key(self):
        Path(str(self.source) + '-wal').write_bytes(b'live')
        report, _ = self.run_case('BLOCKED')
        self.assertEqual(report['error']['code'], 'LIVE_DATABASE_REFUSED')
        self.assertFalse(report['secret_contents_read'])

    def test_trusted_checksum_mismatch_refused(self):
        self.args.sha256 = '0' * 64
        report, _ = self.run_case('BLOCKED')
        self.assertEqual(report['error']['code'], 'CHECKSUM_MISMATCH')
        self.assertFalse(report['secret_contents_read'])

    def test_wrong_worker_name_is_rejected_before_key_read(self):
        path = self.root / 'flame-kernel'
        path.write_text('not an authorized worker')
        path.chmod(0o700)
        self.args.keycheck_bin = str(path)
        report, _ = self.run_case('BLOCKED')
        self.assertEqual(report['error']['code'], 'TRUSTED_WORKER_REQUIRED')
        self.assertFalse(report['secret_contents_read'])

    def test_partial_copy_interrupt_is_cleaned(self):
        def interrupted(args, staged, deadline):
            staged.write_bytes(b'partial')
            raise KeyboardInterrupt
        with patch.object(keycheck.drill, 'acquire', side_effect=interrupted):
            report, _ = self.run_case('BLOCKED')
        self.assertEqual(report['error']['code'], 'INTERRUPTED')
        self.assertFalse(report['secret_contents_read'])

    def test_key_permission_change_during_read_is_rejected(self):
        fd = os.open(self.assets, os.O_RDONLY | os.O_DIRECTORY)
        try:
            with keycheck.inventory.open_beneath(fd, 'master') as key_fd:
                original_read = os.read
                def changed(descriptor, count):
                    data = original_read(descriptor, count)
                    self.master.chmod(0o400)
                    return data
                report = {'secret_contents_read': False}
                with patch.object(keycheck.os, 'read', side_effect=changed):
                    with self.assertRaisesRegex(keycheck.drill.DrillError, 'KEY_CHANGED'):
                        keycheck.read_master(key_fd, report, time.monotonic() + 10)
                self.assertTrue(report['secret_contents_read'])
        finally:
            os.close(fd)

    def test_existing_report_cannot_be_overwritten(self):
        Path(self.args.out).mkdir()
        with self.assertRaises(FileExistsError):
            keycheck.run(self.args)

    def test_worker_protocol_rejects_extra_duplicate_and_inconsistent_fields(self):
        valid = {'schema': keycheck.WORKER_SCHEMA, 'checked': 1, 'authenticated': 1, 'rejected': 0}
        for changes in [{'plaintext': self.secret.decode()}, {'checked': True}, {'authenticated': 2}, {'schema': 'unknown'}]:
            with self.subTest(changes=changes):
                with self.assertRaises(keycheck.drill.DrillError):
                    keycheck.parse_worker_response(json.dumps(valid | changes).encode(), 1)
        with self.assertRaises(keycheck.drill.DrillError):
            keycheck.parse_worker_response(b'{"schema":"x","schema":"y"}', 1)

    def test_pipe_transport_has_no_secret_argv_or_inherited_environment(self):
        body = '''import os,sys,json
assert sys.argv[1:] == ['--stdin-authorized']
assert 'OP_DATABASE_URL' not in os.environ
assert 'PRIVATE_TEST_ENV' not in os.environ
assert os.environ['TMPDIR'] == os.getcwd()
data=json.load(sys.stdin)
n=len(data['ciphertexts'])
print(json.dumps({'schema':data['schema'],'checked':n,'authenticated':n,'rejected':0}))
'''
        binary = self.make_worker(body)
        with patch.dict(os.environ, {'OP_DATABASE_URL': 'not-allowed', 'PRIVATE_TEST_ENV': self.secret.decode()}):
            result = keycheck.invoke_worker(binary, self.secret, ['enc:v1:' + 'x' * 60000] * 4, self.root, time.monotonic() + 10)
        self.assertEqual(result['authenticated'], 4)

    def test_all_allowlisted_columns_and_no_unrelated_setting_values(self):
        self.execute("INSERT INTO dns_provider_accounts VALUES('enc:v1:dns-canary')")
        self.execute("INSERT INTO user_two_factor VALUES('enc:v1:totp-canary','enc:v1:pending-canary')")
        self.execute("INSERT INTO panel_settings VALUES('assistant.api_key','enc:v1:assistant-canary')")
        self.execute("INSERT INTO panel_settings VALUES('unrelated','PRIVATE_UNRELATED_CANARY')")
        report, _ = self.run_case()
        self.assertEqual(report['authenticated_records'], 5)
        self.assertEqual(len(report['observations']['encrypted_fields']), 5)
        self.assertNotIn('PRIVATE_UNRELATED_CANARY', json.dumps(report))

    def test_cli_sigterm_cleans_copy_and_stops_worker(self):
        marker = self.root / 'worker-ready'
        self.make_worker('import os,sys,time\nfrom pathlib import Path\nsys.stdin.read()\nPath(' + repr(str(marker)) + ').write_text(str(os.getpid()))\ntime.sleep(60)\n')
        command = [sys.executable, str(Path(keycheck.__file__)), '--backup', str(self.source),
            '--assets-root', str(self.assets), '--key-path', 'master', '--keycheck-bin', str(self.binary),
            '--allow-secret-read', '--out', self.args.out]
        original = self.source.read_bytes()
        process = subprocess.Popen(command, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
        try:
            deadline = time.monotonic() + 10
            while not marker.exists() and process.poll() is None and time.monotonic() < deadline:
                time.sleep(0.02)
            self.assertTrue(marker.exists())
            process.send_signal(signal.SIGTERM)
            stdout, stderr = process.communicate(timeout=10)
            self.assertEqual(process.returncode, 1, stderr)
            self.assertNotIn(self.secret, stdout + stderr)
            report = json.loads((Path(self.args.out) / 'keycheck.json').read_text())
            self.assertEqual(report['status'], 'BLOCKED')
            self.assertEqual(report['error']['code'], 'INTERRUPTED')
            self.assertTrue(report['copies_removed'])
            self.assertFalse(report['key_match_verified'])
            self.assertEqual(self.source.read_bytes(), original)
            with self.assertRaises(ProcessLookupError):
                os.kill(int(marker.read_text()), 0)
        finally:
            if process.poll() is None:
                process.kill()
                process.wait()
            # Even a future driver regression must not leave the synthetic helper alive.
            if marker.exists():
                pid = int(marker.read_text())
                try:
                    command_line = Path(f'/proc/{pid}/cmdline').read_bytes()
                    if str(self.binary).encode() in command_line and os.getpgid(pid) == pid:
                        os.killpg(pid, signal.SIGKILL)
                except (FileNotFoundError, ProcessLookupError):
                    pass
            for stream in (process.stdout, process.stderr):
                if stream and not stream.closed:
                    stream.close()

    def test_worker_output_limit_is_bounded_and_rejected(self):
        binary = self.make_worker('import sys\nsys.stdin.read()\nsys.stdout.write("x"*5000)\n')
        with self.assertRaisesRegex(keycheck.drill.DrillError, 'WORKER_PROTOCOL_INVALID'):
            keycheck.invoke_worker(binary, self.secret, ['enc:v1:x'], self.root, time.monotonic() + 10)

    def test_timeout_stops_and_reaps_owned_worker(self):
        binary = self.make_worker('import sys,time\nsys.stdin.read()\ntime.sleep(60)\n')
        original_popen = subprocess.Popen
        processes = []
        def capture(*args, **kwargs):
            process = original_popen(*args, **kwargs)
            processes.append(process)
            return process
        with patch.object(keycheck.subprocess, 'Popen', side_effect=capture):
            with self.assertRaisesRegex(keycheck.drill.DrillError, 'TIMEOUT'):
                keycheck.invoke_worker(binary, self.secret, ['enc:v1:x'], self.root, time.monotonic() + 0.2)
        self.assertIsNotNone(processes[0].poll())

    def test_cli_missing_authorization_and_unknown_arguments_do_not_echo_values(self):
        command = [sys.executable, str(Path(keycheck.__file__)), '--backup', str(self.source),
            '--assets-root', str(self.assets), '--key-path', 'master', '--keycheck-bin', str(self.binary), '--out', self.args.out]
        result = subprocess.run(command, capture_output=True, text=True, timeout=10)
        self.assertEqual(result.returncode, 1)
        self.assertEqual(result.stdout.strip(), 'RECOVERY_KEYCHECK: BLOCKED')
        invalid = subprocess.run(command + ['--accidental-secret', self.secret.decode()], capture_output=True, text=True, timeout=10)
        self.assertEqual(invalid.returncode, 2)
        self.assertNotIn(self.secret.decode(), invalid.stdout + invalid.stderr)


if __name__ == '__main__':
    unittest.main()
