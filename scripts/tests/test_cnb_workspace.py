#!/usr/bin/env python3
"""Offline regression for the real CNB workspace entry; never contact CNB or SSH."""
import base64
import hashlib
import json
import os
from pathlib import Path
import subprocess
import tempfile
import unittest

SCRIPTS = Path(__file__).resolve().parents[1]
ENTRY = SCRIPTS / 'cnb-workspace.sh'
PUBLIC_KEY = Path(__file__).with_name('fixtures') / 'cnb-space-rsa.pub'
FINGERPRINT = 'SHA256:fnWZvpqd+VAIRJxaZdV1KVMFfDgcCjYrP2VSWQ68T/E'
SECRET = 'fixture-credential-must-not-be-logged'


class CnbWorkspaceHostKeyTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory(prefix='flamepanel cnb regression ')
        self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name)
        self.repo = self.root / 'repo with spaces'
        self.scripts = self.repo / 'scripts'
        self.scripts.mkdir(parents=True)
        self.entry = self.scripts / 'cnb-workspace.sh'
        self.entry.write_text(ENTRY.read_text())
        self.bin = self.root / 'mock bin'
        self.bin.mkdir()
        self.temporary = self.root / 'private temp with spaces'
        self.temporary.mkdir()
        self.scan = self.root / 'keyscan-output'
        self.valid_key = PUBLIC_KEY.read_text()
        self.scan.write_text(self.valid_key)
        self.ssh_log = self.root / 'ssh.jsonl'
        self.scan_log = self.root / 'keyscan.jsonl'
        self.env = {
            'PATH': str(self.bin) + os.pathsep + '/usr/local/bin:/usr/bin:/bin',
            'HOME': str(self.root), 'LANG': 'C.UTF-8',
            'TMPDIR': str(self.temporary), 'CNB_TOKEN': SECRET,
            'CNB_REPO': 'fixture/panel', 'FIXTURE_SCAN': str(self.scan),
            'FIXTURE_SSH_LOG': str(self.ssh_log),
            'FIXTURE_SCAN_LOG': str(self.scan_log),
            'FIXTURE_TARGET': 'cnb-test-001.fixture-session@cnb.space',
            'GIT_CONFIG_NOSYSTEM': '1', 'GIT_TERMINAL_PROMPT': '0',
        }
        self.mock('curl', '''
import json, os, sys
url = sys.argv[-1]
if '/workspace/detail/' in url:
    print(json.dumps({'remoteSsh': os.environ['FIXTURE_TARGET']}))
elif url.endswith('/workspace/start'):
    print(json.dumps({'sn': 'cnb-test'}))
elif url.endswith('/workspace/stop'):
    print(json.dumps({'message': 'fixture stopped'}))
else:
    raise SystemExit('Unexpected fixture API path')
''')
        self.mock('ssh-keyscan', '''
import json, os, pathlib, sys
with open(os.environ['FIXTURE_SCAN_LOG'], 'a') as log:
    log.write(json.dumps(sys.argv[1:]) + '\\n')
if os.environ.get('FIXTURE_SCAN_EXIT'):
    print('fixture keyscan unavailable', file=sys.stderr)
    raise SystemExit(int(os.environ['FIXTURE_SCAN_EXIT']))
sys.stdout.write(pathlib.Path(os.environ['FIXTURE_SCAN']).read_text())
''')
        self.mock('ssh', '''
import json, os, pathlib, stat, sys
args = sys.argv[1:]
known = next((x.split('=', 1)[1] for x in args if x.startswith('UserKnownHostsFile=')), None)
record = {'args': args, 'known_path': known}
if known:
    path = pathlib.Path(known)
    record.update(known=path.read_text(), mode=stat.S_IMODE(path.stat().st_mode),
                  parent_mode=stat.S_IMODE(path.parent.stat().st_mode))
record['stdin_bytes'] = len(sys.stdin.buffer.read())
with open(os.environ['FIXTURE_SSH_LOG'], 'a') as log:
    log.write(json.dumps(record) + '\\n')
print('FIXTURE_SSH_COMPLETE')
raise SystemExit(int(os.environ.get('FIXTURE_SSH_EXIT', '0')))
''')
        self.git('init', '-q')
        self.git('-c', 'user.name=Fixture', '-c', 'user.email=fixture@example.invalid',
                 'add', 'scripts/cnb-workspace.sh')
        self.git('-c', 'user.name=Fixture', '-c', 'user.email=fixture@example.invalid',
                 '-c', 'commit.gpgsign=false', 'commit', '-qm', 'fixture baseline')

    def mock(self, name, body):
        path = self.bin / name
        path.write_text('#!/usr/bin/env python3\n' + body)
        path.chmod(0o755)

    def git(self, *args):
        return subprocess.run(['git', *args], cwd=self.repo, env=self.env,
                              capture_output=True, text=True, check=True, timeout=10)

    def run_entry(self, command='ssh', *args):
        result = subprocess.run(['bash', str(self.entry), command, 'cnb-test', *args],
                                cwd=self.root, env=self.env, input='',
                                capture_output=True, text=True, timeout=10)
        self.assertNotIn(SECRET, result.stdout + result.stderr)
        return result

    def records(self):
        if not self.ssh_log.exists():
            return []
        return [json.loads(line) for line in self.ssh_log.read_text().splitlines()]

    def assert_rejected(self):
        result = self.run_entry('ssh', 'printf fixture')
        self.assertNotEqual(result.returncode, 0, result.stdout)
        self.assertEqual(self.records(), [], 'Unverified host reached SSH')
        self.assertEqual(list(self.temporary.iterdir()), [], 'Temporary trust material leaked')
        return result

    def assert_verified(self, record):
        args = record['args']
        for option in ['StrictHostKeyChecking=yes', 'GlobalKnownHostsFile=/dev/null',
                       'KnownHostsCommand=none', 'VerifyHostKeyDNS=no',
                       'UpdateHostKeys=no', 'BatchMode=yes',
                       'HostKeyAlgorithms=rsa-sha2-512,rsa-sha2-256']:
            self.assertIn(option, args)
        self.assertNotIn('StrictHostKeyChecking=accept-new', args)
        self.assertNotIn('StrictHostKeyChecking=no', args)
        self.assertEqual(record['known'], self.valid_key.strip() + '\n')
        self.assertEqual(record['mode'], 0o600)
        self.assertEqual(record['parent_mode'], 0o700)
        self.assertFalse(Path(record['known_path']).exists(), 'Trust file was not cleaned up')
        self.assertEqual(list(self.temporary.iterdir()), [])

    def test_fixture_is_the_documented_public_key(self):
        blob = base64.b64decode(self.valid_key.split()[2], validate=True)
        fingerprint = 'SHA256:' + base64.b64encode(hashlib.sha256(blob).digest()).decode().rstrip('=')
        self.assertEqual(fingerprint, FINGERPRINT)

    def test_shell_syntax(self):
        result = subprocess.run(['bash', '-n', str(self.entry)], capture_output=True, text=True)
        self.assertEqual(result.returncode, 0, result.stderr)

    def test_verified_host_with_spaces_in_paths_and_command(self):
        command = "printf '%s\\n' 'a command with spaces'"
        result = self.run_entry('ssh', command)
        self.assertEqual(result.returncode, 0, result.stderr)
        record, = self.records()
        self.assert_verified(record)
        self.assertEqual(record['args'][-2:], [self.env['FIXTURE_TARGET'], command])
        self.assertIn('FIXTURE_SSH_COMPLETE', result.stdout)

    def test_all_ssh_operations_pin_the_host(self):
        for command in ['sync', 'run', 'bg', 'hold', 'log', 'ssh']:
            with self.subTest(command=command):
                self.ssh_log.unlink(missing_ok=True)
                result = self.run_entry(command)
                self.assertEqual(result.returncode, 0, result.stderr)
                record, = self.records()
                self.assert_verified(record)
                if command == 'sync':
                    self.assertGreater(record['stdin_bytes'], 0)
                    self.assertFalse((self.repo / '.ci-rev').exists())
                    self.assertFalse((self.repo / '.sync-manifest').exists())

    def test_wrong_key_blocks_every_ssh_operation(self):
        self.scan.write_text('cnb.space ssh-rsa ' + base64.b64encode(b'wrong host key').decode() + '\n')
        for command in ['sync', 'run', 'bg', 'hold', 'log', 'ssh']:
            with self.subTest(command=command):
                self.ssh_log.unlink(missing_ok=True)
                result = self.run_entry(command)
                self.assertNotEqual(result.returncode, 0, result.stdout)
                self.assertEqual(self.records(), [])
                self.assertEqual(list(self.temporary.iterdir()), [])

    def test_keyscan_failure_is_closed(self):
        self.env['FIXTURE_SCAN_EXIT'] = '7'
        self.assert_rejected()

    def test_empty_keyscan_is_closed(self):
        self.scan.write_text('')
        self.assert_rejected()

    def test_comments_are_not_a_verified_key(self):
        self.scan.write_text('# cnb.space:22 SSH-2.0-fixture\n\n')
        self.assert_rejected()

    def test_comments_and_duplicate_verified_keys_are_safe(self):
        self.scan.write_text('# public key\n\n' + self.valid_key + self.valid_key)
        result = self.run_entry()
        self.assertEqual(result.returncode, 0, result.stderr)
        record, = self.records()
        self.assert_verified(record)

    def test_malformed_base64_is_closed(self):
        self.scan.write_text('cnb.space ssh-rsa !!!not-base64!!!\n')
        self.assert_rejected()

    def test_wrong_key_type_is_closed(self):
        self.scan.write_text(self.valid_key.replace('ssh-rsa', 'ssh-ed25519'))
        self.assert_rejected()

    def test_wrong_scanned_hostname_is_closed(self):
        self.scan.write_text(self.valid_key.replace('cnb.space', 'untrusted.example'))
        self.assert_rejected()

    def test_mixed_verified_and_unverified_keys_are_closed(self):
        self.scan.write_text(self.valid_key + 'cnb.space ssh-rsa d3Jvbmc=\n')
        self.assert_rejected()

    def test_oversized_scan_is_closed(self):
        self.scan.write_text('#' + 'x' * 65536 + '\n' + self.valid_key)
        self.assert_rejected()

    def test_unexpected_target_is_rejected_before_keyscan(self):
        for target in ['session@untrusted.example', '-oProxyCommand=bad@cnb.space',
                       'session@cnb.space extra', 'session@cnb.space:22',
                       'session@cnb.space\n-oStrictHostKeyChecking=no']:
            with self.subTest(target=target):
                self.env['FIXTURE_TARGET'] = target
                self.scan_log.unlink(missing_ok=True)
                self.assert_rejected()
                self.assertFalse(self.scan_log.exists())

    def test_existing_user_trust_is_not_modified_or_used(self):
        user_hosts = self.root / '.ssh' / 'known_hosts'
        user_hosts.parent.mkdir()
        original = 'cnb.space ssh-rsa d3Jvbmc=\n'
        user_hosts.write_text(original)
        result = self.run_entry()
        self.assertEqual(result.returncode, 0, result.stderr)
        record, = self.records()
        self.assert_verified(record)
        self.assertNotEqual(record['known_path'], str(user_hosts))
        self.assertEqual(user_hosts.read_text(), original)

    def test_ssh_exit_status_and_cleanup_are_preserved(self):
        self.env['FIXTURE_SSH_EXIT'] = '23'
        result = self.run_entry()
        self.assertEqual(result.returncode, 23)
        record, = self.records()
        self.assert_verified(record)

    def test_start_and_stop_do_not_open_ssh(self):
        for command in ['start', 'stop']:
            with self.subTest(command=command):
                result = self.run_entry(command)
                self.assertEqual(result.returncode, 0, result.stderr)
                self.assertEqual(self.records(), [])
                self.assertFalse(self.scan_log.exists())


if __name__ == '__main__':
    unittest.main(verbosity=2)
