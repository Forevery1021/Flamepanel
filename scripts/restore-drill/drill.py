#!/usr/bin/env python3
"""R-01: bounded, offline SQLite restore drill. Never starts the panel or writes the source."""
import argparse
import contextlib
import datetime as dt
import errno
import hashlib
import json
import os
import re
from pathlib import Path
import shutil
import signal
import sqlite3
import stat
import tempfile
import time
import urllib.parse
import urllib.request

SCHEMA = {
    'users': ('id', 'username', 'password_hash', 'role'),
    'permissions': ('id', 'resource', 'action'),
    'panel_settings': ('key', 'value'),
}
STAGES = ('acquire', 'validate', 'capacity', 'restore', 'reopen', 'sample')
RESERVE = 8 * 1024 * 1024
CHUNK = 64 * 1024


class DrillError(Exception):
    """Only fixed codes, never source paths, URLs, rows or driver errors in reports."""


class NoRedirect(urllib.request.HTTPRedirectHandler):
    def redirect_request(self, req, fp, code, msg, headers, newurl):
        raise DrillError('REDIRECT_REFUSED')


def check_deadline(deadline):
    if time.monotonic() >= deadline:
        raise DrillError('TIMEOUT')


def capacity(directory, required):
    if shutil.disk_usage(directory).free < required:
        raise DrillError('INSUFFICIENT_SPACE')


def copy_stream(source, destination, max_bytes, deadline):
    size = 0
    digest = hashlib.sha256()
    with destination.open('xb') as output:
        os.chmod(destination, 0o600)
        while True:
            check_deadline(deadline)
            chunk = getattr(source, 'read1', source.read)(CHUNK)
            if not chunk:
                break
            size += len(chunk)
            if size > max_bytes:
                raise DrillError('SIZE_LIMIT')
            capacity(destination.parent, len(chunk) + RESERVE)
            output.write(chunk)
            digest.update(chunk)
        output.flush()
        os.fsync(output.fileno())
    if not size:
        raise DrillError('EMPTY_SOURCE')
    return size, digest.hexdigest()


def acquire(args, staged, deadline):
    if args.backup is not None:
        path = Path(args.backup)
        # Only standalone exported backups, not a live WAL database or symlink/FIFO.
        if any(Path(str(path) + suffix).exists() for suffix in ('-wal', '-shm', '-journal')):
            raise DrillError('LIVE_DATABASE_REFUSED')
        fd = os.open(path, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK)
        with os.fdopen(fd, 'rb') as source:
            before = os.fstat(source.fileno())
            if not stat.S_ISREG(before.st_mode):
                raise DrillError('REGULAR_FILE_REQUIRED')
            if before.st_size > args.max_bytes:
                raise DrillError('SIZE_LIMIT')
            capacity(staged.parent, 3 * before.st_size + RESERVE)
            result = copy_stream(source, staged, args.max_bytes, deadline)
            after = os.fstat(source.fileno())
            if (before.st_size, before.st_mtime_ns, before.st_ctime_ns) != (
                    after.st_size, after.st_mtime_ns, after.st_ctime_ns):
                raise DrillError('SOURCE_CHANGED')
            if any(Path(str(path) + suffix).exists() for suffix in ('-wal', '-shm', '-journal')):
                raise DrillError('LIVE_DATABASE_REFUSED')
            return result
    url = urllib.parse.urlsplit(args.url)
    if url.scheme != 'https' or not url.hostname or url.username or url.password or url.fragment:
        raise DrillError('HTTPS_SOURCE_REQUIRED')
    # Avoid environment proxy credentials. No redirects, auth, shell or URL logging.
    opener = urllib.request.build_opener(urllib.request.ProxyHandler({}), NoRedirect())
    check_deadline(deadline)
    with opener.open(args.url, timeout=min(5, max(0.1, deadline - time.monotonic()))) as response:
        if response.status != 200:
            raise DrillError('REMOTE_FETCH_FAILED')
        length = response.headers.get('Content-Length')
        if length is not None:
            try:
                length = int(length)
            except ValueError as error:
                raise DrillError('REMOTE_LENGTH_INVALID') from error
            if length < 0 or length > args.max_bytes:
                raise DrillError('SIZE_LIMIT')
            capacity(staged.parent, 3 * length + RESERVE)
        result = copy_stream(response, staged, args.max_bytes, deadline)
        if length is not None and result[0] != length:
            raise DrillError('REMOTE_TRUNCATED')
        return result


@contextlib.contextmanager
def reader(path, deadline):
    # Open only our immutable staged/restored files. Never open the original in SQLite.
    conn = sqlite3.connect(path.as_uri() + '?mode=ro&immutable=1', uri=True, timeout=1)
    try:
        conn.execute('PRAGMA trusted_schema=OFF')
        conn.execute('PRAGMA query_only=ON')
        conn.set_progress_handler(lambda: int(time.monotonic() >= deadline), 1000)
        yield conn
    finally:
        conn.close()


def validate(path, deadline):
    with path.open('rb') as source:
        if source.read(16) != b'SQLite format 3\x00':
            raise DrillError('NOT_SQLITE')
    with reader(path, deadline) as conn:
        if conn.execute('PRAGMA integrity_check').fetchmany(2) != [('ok',)]:
            raise DrillError('INTEGRITY_FAILED')
        for table, columns in SCHEMA.items():
            row = conn.execute(
                'SELECT type, sql FROM sqlite_schema WHERE name = ?', (table,)).fetchone()
            if not row or row[0] != 'table' or not row[1] or 'virtual' in row[1].lower():
                raise DrillError('SCHEMA_MISMATCH')
            actual = {r[1] for r in conn.execute(f'PRAGMA table_info("{table}")')}
            if not set(columns).issubset(actual):
                raise DrillError('SCHEMA_MISMATCH')


def sampled_fingerprint(path, deadline):
    # Read at most 20 rows from a fixed table allowlist. Never persist values or their hashes.
    result = {}
    with reader(path, deadline) as conn:
        for table, columns in SCHEMA.items():
            check_deadline(deadline)
            names = ', '.join(f'"{c}"' for c in columns)
            rows = conn.execute(f'SELECT {names} FROM "{table}" ORDER BY "{columns[0]}" LIMIT 20').fetchall()
            result[table] = rows
    return result


def error_code(error, stage, deadline):
    if isinstance(error, DrillError):
        return str(error)
    if isinstance(error, (KeyboardInterrupt, InterruptedError)):
        return 'INTERRUPTED'
    if time.monotonic() >= deadline or isinstance(error, TimeoutError):
        return 'TIMEOUT'
    if isinstance(error, OSError) and error.errno == errno.ENOSPC:
        return 'INSUFFICIENT_SPACE'
    if isinstance(error, sqlite3.Error):
        return 'SQLITE_CHECK_FAILED'
    return 'SOURCE_UNAVAILABLE' if stage == 'acquire' else 'IO_FAILED'


def run(args):
    """Output must not exist. A private subdirectory holds all DB copies and is always removed."""
    output = Path(args.out).absolute()
    output.mkdir(mode=0o700)  # Fail closed: no overwrite, no automatic parent creation.
    start = time.monotonic()
    deadline = start + args.timeout
    probe_bin = getattr(args, 'probe_bin', None)
    stages = STAGES + (('probe',) if probe_bin else ())
    report = {
        'schema': 'flamepanel/restore-drill/v1', 'status': 'FAIL',
        'scope': 'sqlite-with-dedicated-probe' if probe_bin else 'offline-sqlite-only',
        'kernel_started': False, 'migration_verified': False, 'probe_http_verified': False,
        'source_kind': 'local' if args.backup is not None else 'https',
        'checksum_verified': False,
        'started_at': dt.datetime.now(dt.timezone.utc).isoformat(),
        'stages': {name: {'status': 'NOT_RUN'} for name in stages},
        'production_rto_seconds': None, 'rpo_seconds': None,
        'backup_age_seconds': None, 'storage_rto_seconds': None,
        'limitations': ['No production kernel startup or production authentication', 'No key/decryption validation',
                        'No external service, database or volume recovery',
                        'RPO cannot be inferred from a database file or mtime'],
    }
    if args.backup_time:
        report['backup_age_seconds'] = (dt.datetime.now(dt.timezone.utc) - args.backup_time).total_seconds()
        report['backup_time_basis'] = 'operator-supplied-not-verified'
    stage = 'acquire'
    stage_start = start
    try:
        with tempfile.TemporaryDirectory(prefix='.isolated-', dir=output) as tmp:
            work = Path(tmp)
            staged, restored = work / 'snapshot.db', work / 'restored.db'
            original_samples = None
            recovery_start = None
            for stage in stages:
                check_deadline(deadline)
                stage_start = time.monotonic()
                if stage == 'acquire':
                    size, digest = acquire(args, staged, deadline)
                    report['snapshot_bytes'], report['snapshot_sha256'] = size, digest
                elif stage == 'validate':
                    if args.sha256 is not None:
                        if digest != args.sha256.lower():
                            raise DrillError('CHECKSUM_MISMATCH')
                        report['checksum_verified'] = True
                    validate(staged, deadline)
                    original_samples = sampled_fingerprint(staged, deadline)
                elif stage == 'capacity':
                    capacity(work, 2 * size + RESERVE)
                elif stage == 'restore':
                    recovery_start = time.monotonic()
                    temporary = work / 'restored.tmp'
                    with staged.open('rb') as source:
                        _, copied_digest = copy_stream(source, temporary, args.max_bytes, deadline)
                    if copied_digest != digest:
                        raise DrillError('COPY_MISMATCH')
                    temporary.rename(restored)
                elif stage == 'reopen':
                    validate(restored, deadline)
                elif stage == 'sample':
                    samples = sampled_fingerprint(restored, deadline)
                    if samples != original_samples:
                        raise DrillError('SAMPLE_MISMATCH')
                    report['sampled_rows'] = {table: len(rows) for table, rows in samples.items()}
                    report['storage_rto_seconds'] = round(time.monotonic() - recovery_start, 6)
                elif stage == 'probe':
                    from probe_http import run_probe, ProbeFailure
                    try:
                        report['probe'] = run_probe(probe_bin, restored, work, deadline)
                    except ProbeFailure as error:
                        raise DrillError(str(error)) from error
                    report['migration_verified'] = True
                    report['probe_http_verified'] = True
                report['stages'][stage] = {'status': 'PASS', 'seconds': round(time.monotonic() - stage_start, 6)}
            report['status'] = 'PASS'
    except (Exception, KeyboardInterrupt) as error:
        report['stages'][stage] = {
            'status': 'FAIL', 'code': error_code(error, stage, deadline),
            'seconds': round(time.monotonic() - stage_start, 6),
        }
    report['elapsed_seconds'] = round(time.monotonic() - start, 6)
    report['copies_removed'] = not any(output.glob('.isolated-*'))
    if not report['copies_removed']:
        report['status'] = 'FAIL'
        report['cleanup_error'] = 'COPY_CLEANUP_FAILED'
    # Directory is private and newly created; no raw exception/path/URL/row values are emitted.
    with (output / 'report.json').open('x') as file:
        os.chmod(file.name, 0o600)
        json.dump(report, file, ensure_ascii=False, indent=2)
        file.write('\n')
    lines = ['# SQLite 隔离恢复演练', '', f"结果：**{report['status']}**", '',
             '范围：SQLite 副本恢复；可选专用迁移 / HTTP 探针；未启动生产面板内核。', '',
             '| 阶段 | 状态 | 错误码 | 秒 |', '| --- | --- | --- | --- |']
    for name, result in report['stages'].items():
        lines.append(f"| {name} | {result['status']} | {result.get('code', '')} | {result.get('seconds', '')} |")
    lines += ['', f"数据库恢复验证耗时：{report['storage_rto_seconds']} 秒（不是生产服务 RTO）。",
              f"备份年龄：{report['backup_age_seconds']} 秒（如提供，为操作员声明的时间）。",
              f"迁移验证：{report['migration_verified']}；专用 HTTP 探针验证：{report['probe_http_verified']}。",
              'RPO：未知。未验证生产内核、生产鉴权、密钥解密、任务、节点与外部数据。',
              f"演练副本已清理：{report['copies_removed']}。", '']
    with (output / 'report.md').open('x') as file:
        os.chmod(file.name, 0o600)
        file.write('\n'.join(lines))
    return report


def backup_time(value):
    try:
        parsed = dt.datetime.fromisoformat(value.replace('Z', '+00:00'))
        if parsed.tzinfo is None or parsed > dt.datetime.now(dt.timezone.utc):
            raise ValueError
        return parsed
    except ValueError as error:
        raise argparse.ArgumentTypeError('需要含时区且不在未来的 ISO-8601 时间') from error


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    sources = parser.add_mutually_exclusive_group(required=True)
    sources.add_argument('--backup', help='已导出的独立 SQLite 备份；不接受活库/WAL/符号链接')
    sources.add_argument('--url', help='HTTPS 备份地址；不允许重定向或 URL 用户密码；优先预先下载到本地')
    parser.add_argument('--out', required=True, help='不存在的报告目录；父目录必须已存在')
    parser.add_argument('--max-bytes', type=int, default=512 * 1024 * 1024)
    parser.add_argument('--timeout', type=float, default=120)
    parser.add_argument('--backup-time', type=backup_time)
    parser.add_argument('--probe-bin', help='可选：本项目构建的可信 fp-restore-probe 路径；不是生产内核')
    parser.add_argument('--sha256', help='可选：从可信渠道获得的快照 SHA-256')
    args = parser.parse_args()
    if args.sha256 is not None and not re.fullmatch(r'[a-fA-F0-9]{64}', args.sha256):
        parser.error('SHA-256 必须是 64 位十六进制')
    if not 0 < args.max_bytes <= 8 * 1024**3 or not 0 < args.timeout <= 3600:
        parser.error('大小范围 1..8GiB，超时范围 0..3600 秒')
    def interrupt(signum, frame):
        raise InterruptedError
    signal.signal(signal.SIGTERM, interrupt)
    try:
        report = run(args)
    except (OSError, KeyboardInterrupt, InterruptedError):
        print('REPORT_UNAVAILABLE: 输出目录必须不存在且可写；如发生中断请检查残留副本。')
        return 2
    print(f"RESTORE_DRILL: {report['status']}")
    return 0 if report['status'] == 'PASS' else 1


if __name__ == '__main__':
    raise SystemExit(main())
