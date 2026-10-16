"""Real-process R-01B contracts, invoked by the Rust integration suite with its built binary."""
import argparse
import contextlib
import json
import os
from pathlib import Path
import secrets
import sqlite3
import subprocess
import sys
import tempfile
import time
import unittest

import drill
import probe_http

BINARY = None


class ProbeContracts(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name)
        self.source = self.root / 'legacy.db'
        self.sentinel = self.root / 'must-not-execute'
        with contextlib.closing(sqlite3.connect(self.source)) as conn:
            # Legacy core schema: missing newer user columns and most modern tables.
            conn.executescript('''
                CREATE TABLE users(id INTEGER PRIMARY KEY, username TEXT NOT NULL UNIQUE,
                    password_hash TEXT NOT NULL, role TEXT NOT NULL DEFAULT 'user',
                    created_at TEXT NOT NULL DEFAULT (datetime('now')));
                CREATE TABLE permissions(id INTEGER PRIMARY KEY, resource TEXT NOT NULL,
                    action TEXT NOT NULL, description TEXT NOT NULL DEFAULT '');
                CREATE TABLE panel_settings(key TEXT PRIMARY KEY, value TEXT NOT NULL DEFAULT '',
                    description TEXT NOT NULL DEFAULT '', updated_at TEXT NOT NULL DEFAULT (datetime('now')));
                CREATE TABLE scheduled_tasks(id INTEGER PRIMARY KEY, name TEXT NOT NULL,
                    command TEXT NOT NULL, schedule TEXT NOT NULL DEFAULT '* * * * *',
                    enabled INTEGER NOT NULL DEFAULT 1, last_status TEXT NOT NULL DEFAULT 'never',
                    last_output TEXT NOT NULL DEFAULT '', last_run_at TEXT, next_run_at TEXT,
                    created_at TEXT NOT NULL DEFAULT (datetime('now')),
                    updated_at TEXT NOT NULL DEFAULT (datetime('now')));
                INSERT INTO users(id,username,password_hash,role) VALUES(1,'private-user','secret-hash','admin');
                INSERT INTO panel_settings(key,value) VALUES('jwt_secret','secret-setting-must-not-leak');
            ''')
            conn.execute("INSERT INTO scheduled_tasks(name,command,next_run_at) VALUES(?,?,?)",
                         ('never run', 'touch ' + str(self.sentinel), '2000-01-01 00:00:00'))
            conn.commit()
        self.before = self.source.read_bytes()

    def args(self):
        return argparse.Namespace(backup=str(self.source), url=None,
                                  out=str(self.root / 'report'), max_bytes=1024**2,
                                  timeout=30, backup_time=None, sha256=None, probe_bin=BINARY)

    def launch(self, snapshot=None, ttl=20, token=None, extra_env=None):
        token = token or secrets.token_hex(32)
        env = dict(os.environ, OP_DATABASE_URL='sqlite:/must-not-create.db',
                   OP_HOST='0.0.0.0', OP_PORT='12345', OP_FILE_ROOT=str(self.sentinel))
        env.update(extra_env or {})
        p = subprocess.Popen([BINARY, '--snapshot', str(snapshot or self.source), '--ttl', str(ttl)],
                             cwd=self.root, env=env, stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                             stderr=subprocess.PIPE, start_new_session=True)
        p.stdin.write((token + '\n').encode())
        p.stdin.close()
        def cleanup():
            try:
                probe_http.stop(p)
            except probe_http.ProbeFailure:
                pass
            p.stdout.close()
            p.stderr.close()
        self.addCleanup(cleanup)
        return p, token

    def assert_source_intact(self):
        self.assertEqual(self.source.read_bytes(), self.before)
        self.assertFalse(self.sentinel.exists())
        self.assertFalse(list(self.root.glob('.probe-*')))
        with contextlib.closing(sqlite3.connect(self.source)) as conn:
            self.assertNotIn('must_change_password', [r[1] for r in conn.execute('PRAGMA table_info(users)')])
            self.assertEqual(conn.execute("SELECT value FROM panel_settings WHERE key='jwt_secret'").fetchone()[0],
                             'secret-setting-must-not-leak')

    def test_full_drill_migrates_copy_and_verifies_http_without_executing_tasks(self):
        report = drill.run(self.args())
        self.assertEqual(report['status'], 'PASS', report)
        self.assertTrue(report['migration_verified'])
        self.assertTrue(report['probe_http_verified'])
        self.assertFalse(report['kernel_started'])
        self.assertEqual(report['probe']['counts']['users'], 1)
        self.assertEqual(report['probe']['counts']['scheduled_tasks'], 1)
        self.assertTrue(report['probe']['stopped_cleanly'])
        self.assertTrue(report['copies_removed'])
        self.assertIsNone(report['production_rto_seconds'])
        self.assertIsNone(report['rpo_seconds'])
        text = json.dumps(report)
        for value in ['private-user', 'secret-hash', 'secret-setting-must-not-leak', str(self.source)]:
            self.assertNotIn(value, text)
        self.assert_source_intact()
        export = os.environ.get('FP_PROBE_EVIDENCE')
        if export:
            dest = Path(export)
            dest.mkdir(mode=0o700)
            for name in ['report.json', 'report.md']:
                (dest / name).write_bytes((Path(self.args().out) / name).read_bytes())

    def test_direct_binary_ignores_op_settings_and_exposes_no_write_or_production_routes(self):
        p, token = self.launch()
        port = probe_http.readiness(p, time.monotonic() + 20)
        result = probe_http.exercise(port, token, time.monotonic() + 10)
        self.assertTrue(result['http_auth_passed'])
        for path in ['/api/mcp', '/api/scheduled-tasks', '/ws/terminal', '/api/auth/login']:
            self.assertEqual(probe_http.request(port, path, token, time.monotonic() + 5)[0], 404)
        probe_http.stop(p)
        self.assert_source_intact()

    def test_ttl_stops_listener_and_removes_copy(self):
        p, token = self.launch(ttl=2)
        port = probe_http.readiness(p, time.monotonic() + 5)
        self.assertEqual(p.wait(timeout=5), 0)
        with self.assertRaises(OSError):
            probe_http.request(port, '/health', token, time.monotonic() + 2)
        self.assert_source_intact()

    def test_active_schema_fails_before_readiness(self):
        with contextlib.closing(sqlite3.connect(self.source)) as conn:
            conn.executescript("CREATE TRIGGER forbid_migration BEFORE UPDATE ON users BEGIN SELECT RAISE(ABORT,'secret-error'); END;")
        self.before = self.source.read_bytes()
        p, _ = self.launch()
        self.assertNotEqual(p.wait(timeout=10), 0)
        self.assertEqual(p.stdout.read(), b'')
        self.assertIn(b'ACTIVE_SCHEMA_REFUSED', p.stderr.read())
        self.assert_source_intact()

    def test_invalid_core_columns_fail_before_migration(self):
        with contextlib.closing(sqlite3.connect(self.source)) as conn:
            conn.execute('ALTER TABLE users RENAME COLUMN username TO incompatible_name')
            conn.commit()
        self.before = self.source.read_bytes()
        p, _ = self.launch()
        self.assertNotEqual(p.wait(timeout=5), 0)
        self.assertEqual(p.stdout.read(), b'')
        self.assertIn(b'CORE_COLUMNS_REQUIRED', p.stderr.read())
        self.assert_source_intact()

    def test_view_refused_before_migration(self):
        with contextlib.closing(sqlite3.connect(self.source)) as conn:
            conn.execute('CREATE VIEW dormant_view AS SELECT id FROM users')
            conn.commit()
        self.before = self.source.read_bytes()
        p, _ = self.launch()
        self.assertNotEqual(p.wait(timeout=5), 0)
        self.assertEqual(p.stdout.read(), b'')
        self.assertIn(b'ACTIVE_SCHEMA_REFUSED', p.stderr.read())
        self.assert_source_intact()

    def test_bad_token_fails_before_copy(self):
        p, _ = self.launch(token='short')
        self.assertNotEqual(p.wait(timeout=5), 0)
        self.assertEqual(p.stdout.read(), b'')
        self.assertIn(b'TOKEN_REQUIRED', p.stderr.read())
        self.assert_source_intact()

    def test_missing_source_failure_is_redacted(self):
        p, _ = self.launch(snapshot=self.root / 'private-secret-missing.db')
        self.assertNotEqual(p.wait(timeout=5), 0)
        self.assertEqual(p.stdout.read(), b'')
        self.assertNotIn(b'private-secret', p.stderr.read())
        self.assert_source_intact()

    def test_failed_migration_does_not_listen_or_change_source(self):
        # run_migrations creates an expires_at index, not a repair for arbitrary table shapes.
        with contextlib.closing(sqlite3.connect(self.source)) as conn:
            conn.execute('CREATE TABLE revoked_refresh_tokens (incompatible TEXT)')
            conn.commit()
        self.before = self.source.read_bytes()
        p, _ = self.launch()
        self.assertNotEqual(p.wait(timeout=10), 0)
        self.assertEqual(p.stdout.read(), b'')
        self.assertIn(b'MIGRATION_FAILED', p.stderr.read())
        self.assert_source_intact()

    def test_wrong_probe_filename_refused_without_execution(self):
        args = self.args()
        args.probe_bin = str(self.root / 'flame-kernel')
        Path(args.probe_bin).write_text('not an executable')
        report = drill.run(args)
        self.assertEqual(report['status'], 'FAIL')
        self.assertEqual(report['stages']['probe']['code'], 'DEDICATED_PROBE_REQUIRED')
        self.assertTrue(report['copies_removed'])
        self.assert_source_intact()


if __name__ == '__main__':
    BINARY = str(Path(sys.argv.pop(1)).resolve(strict=True))
    unittest.main(verbosity=2)
