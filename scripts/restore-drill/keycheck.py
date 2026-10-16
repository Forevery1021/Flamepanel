#!/usr/bin/env python3
"""R-01D: explicit, offline matching of a staged master key to observed enc:v1: data."""
import argparse
import base64
import json
import math
import os
from pathlib import Path
import selectors
import signal
import stat
import subprocess
import tempfile
import time

import drill
import inventory

SCHEMA = 'flamepanel/recovery-keycheck/v1'
WORKER_SCHEMA = 'flamepanel/restore-keycheck-worker/v1'
MASTER_LIMIT = 4096
RECORD_LIMIT = 1024
VALUE_LIMIT = 64 * 1024
CIPHERTEXT_LIMIT = 1024 * 1024
RESPONSE_LIMIT = 4096


def inspect_snapshot(path, deadline):
    """Fixed allowlist only. Do not fetch historical plaintext or caller-selected SQL."""
    drill.validate(path, deadline)
    result = {'encrypted_fields': {}, 'coverage_gaps': [], 'unsupported_ciphertexts': 0,
              'legacy_plaintext_records': 0, 'cipher_scan_complete': False}
    plans, total_count, total_bytes = [], 0, 0
    with drill.reader(path, deadline) as conn:
        active = conn.execute(
            "SELECT (SELECT COUNT(*) FROM sqlite_schema WHERE type IN ('view','trigger')) + "
            "(SELECT COUNT(*) FROM pragma_table_list WHERE type IN ('virtual','shadow'))").fetchone()[0]
        if active:
            raise drill.DrillError('ACTIVE_SCHEMA_REFUSED')
        for table, column, condition in inventory.CIPHERS:
            drill.check_deadline(deadline)
            label = f'{table}.{column}'
            kind = conn.execute("SELECT type FROM pragma_table_list WHERE schema='main' AND name=?", (table,)).fetchone()
            columns = {row[1]: row[6] for row in conn.execute(f'PRAGMA table_xinfo("{table}")')} if kind and kind[0] == 'table' else {}
            required = {column, 'key'} if condition else {column}
            if not required.issubset(columns) or any(columns[name] for name in required):
                result['coverage_gaps'].append(label)
                continue
            suffix = (' AND ' + condition) if condition else ''
            def count(predicate):
                return conn.execute(f'SELECT COUNT(*) FROM "{table}" WHERE {predicate}{suffix}').fetchone()[0]
            text = f'typeof("{column}")=\'text\''
            supported = f'{text} AND substr("{column}",1,7)=\'enc:v1:\''
            unknown = count(f'{text} AND substr("{column}",1,4)=\'enc:\' AND substr("{column}",1,7)!=\'enc:v1:\'')
            supported_count = count(supported)
            result['encrypted_fields'][label] = supported_count + unknown
            result['unsupported_ciphertexts'] += unknown
            result['legacy_plaintext_records'] += count(f'{text} AND substr("{column}",1,4)!=\'enc:\'')
            if count(f'"{column}" IS NOT NULL AND typeof("{column}")!=\'text\''):
                result['coverage_gaps'].append(label + '.non_text')
            if count(f'{supported} AND length(CAST("{column}" AS BLOB))>{VALUE_LIMIT}'):
                result['coverage_gaps'].append(label + '.value_limit')
            total_count += supported_count
            total_bytes += conn.execute(
                f'SELECT COALESCE(SUM(length(CAST("{column}" AS BLOB))),0) FROM "{table}" WHERE {supported}{suffix}').fetchone()[0]
            plans.append((table, column, supported + suffix))
        if result['unsupported_ciphertexts']:
            result['coverage_gaps'].append('unsupported_ciphertext_version')
        if total_count > RECORD_LIMIT or total_bytes > CIPHERTEXT_LIMIT:
            result['coverage_gaps'].append('ciphertext_budget')
        result['supported_ciphertexts'] = total_count
        if result['coverage_gaps']:
            return result, []
        values = []
        for table, column, where in plans:
            for (stored,) in conn.execute(f'SELECT "{column}" FROM "{table}" WHERE {where}'):
                drill.check_deadline(deadline)
                values.append(stored)
        if len(values) != total_count or sum(len(value.encode('utf8')) for value in values) > CIPHERTEXT_LIMIT:
            raise drill.DrillError('CIPHER_SCAN_CHANGED')
    result['cipher_scan_complete'] = True
    return result, values


def validate_key_metadata(fd):
    info = os.fstat(fd)
    mode = stat.S_IMODE(info.st_mode)
    if not stat.S_ISREG(info.st_mode) or info.st_nlink != 1:
        raise drill.DrillError('KEY_REGULAR_PRIVATE_FILE_REQUIRED')
    if mode & 0o077 or not mode & 0o400:
        raise drill.DrillError('KEY_PERMISSIONS')
    if not 0 < info.st_size <= MASTER_LIMIT:
        raise drill.DrillError('KEY_SIZE_LIMIT')
    return info


def read_master(fd, report, deadline):
    before = validate_key_metadata(fd)
    # Conservative: an attempted content read is recorded, even if the read fails.
    report['secret_contents_read'] = True
    parts, size = [], 0
    while True:
        drill.check_deadline(deadline)
        data = os.read(fd, MASTER_LIMIT + 1 - size)
        if not data:
            break
        parts.append(data)
        size += len(data)
        if size > MASTER_LIMIT:
            raise drill.DrillError('KEY_SIZE_LIMIT')
    after = validate_key_metadata(fd)
    fields = ('st_size', 'st_mtime_ns', 'st_ctime_ns', 'st_nlink', 'st_mode')
    if any(getattr(before, field) != getattr(after, field) for field in fields) or size != after.st_size:
        raise drill.DrillError('KEY_CHANGED')
    return b''.join(parts)


def worker_path(value):
    path = Path(value).absolute()
    try:
        info = path.lstat()
    except OSError as error:
        raise drill.DrillError('WORKER_UNAVAILABLE') from error
    if path.name != 'fp-restore-keycheck' or not stat.S_ISREG(info.st_mode) or not os.access(path, os.X_OK):
        raise drill.DrillError('TRUSTED_WORKER_REQUIRED')
    return path


def parse_worker_response(raw, expected):
    def unique(pairs):
        result = {}
        for key, value in pairs:
            if key in result:
                raise ValueError
            result[key] = value
        return result
    try:
        data = json.loads(raw.decode('utf8'), object_pairs_hook=unique)
        if not isinstance(data, dict) or set(data) != {'schema', 'checked', 'authenticated', 'rejected'}:
            raise ValueError
        if data['schema'] != WORKER_SCHEMA or any(type(data[name]) is not int or not 0 <= data[name] <= RECORD_LIMIT for name in ('checked', 'authenticated', 'rejected')):
            raise ValueError
        if data['checked'] != expected or data['authenticated'] + data['rejected'] != expected:
            raise ValueError
        return data
    except (ValueError, UnicodeError, TypeError) as error:
        raise drill.DrillError('WORKER_PROTOCOL_INVALID') from error


def invoke_worker(binary, master, values, work, deadline):
    payload = json.dumps({'schema': WORKER_SCHEMA, 'master_b64': base64.b64encode(master).decode('ascii'),
                          'ciphertexts': values}, ensure_ascii=True).encode('utf8')
    process = subprocess.Popen([str(binary), '--stdin-authorized'], cwd=work,
        env={'PATH': os.defpath, 'LANG': 'C.UTF-8', 'TMPDIR': str(work)},
        stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL,
        start_new_session=True)
    output, sent = bytearray(), 0
    try:
        with selectors.DefaultSelector() as selector:
            os.set_blocking(process.stdin.fileno(), False)
            os.set_blocking(process.stdout.fileno(), False)
            selector.register(process.stdin, selectors.EVENT_WRITE)
            selector.register(process.stdout, selectors.EVENT_READ)
            while selector.get_map():
                drill.check_deadline(deadline)
                for key, _ in selector.select(min(0.1, max(0, deadline - time.monotonic()))):
                    if key.fileobj is process.stdin:
                        try:
                            sent += os.write(process.stdin.fileno(), payload[sent:sent + 65536])
                        except BrokenPipeError as error:
                            raise drill.DrillError('WORKER_FAILED') from error
                        if sent == len(payload):
                            selector.unregister(process.stdin)
                            process.stdin.close()
                    else:
                        data = os.read(process.stdout.fileno(), RESPONSE_LIMIT + 1 - len(output))
                        if not data:
                            selector.unregister(process.stdout)
                        else:
                            output.extend(data)
                            if len(output) > RESPONSE_LIMIT:
                                raise drill.DrillError('WORKER_PROTOCOL_INVALID')
        try:
            code = process.wait(timeout=max(0.001, deadline - time.monotonic()))
        except subprocess.TimeoutExpired as error:
            raise drill.DrillError('TIMEOUT') from error
        drill.check_deadline(deadline)
        if code != 0:
            raise drill.DrillError('WORKER_FAILED')
        return parse_worker_response(output, len(values))
    finally:
        if process.poll() is None:
            try:
                os.killpg(process.pid, signal.SIGTERM)
            except ProcessLookupError:
                pass
            try:
                process.wait(timeout=2)
            except subprocess.TimeoutExpired:
                try:
                    os.killpg(process.pid, signal.SIGKILL)
                except ProcessLookupError:
                    pass
                process.wait(timeout=2)
        for stream in (process.stdin, process.stdout):
            if stream and not stream.closed:
                stream.close()


def write_reports(output, report):
    with (output / 'keycheck.json').open('x') as file:
        os.chmod(file.name, 0o600)
        json.dump(report, file, ensure_ascii=False, indent=2)
        file.write('\n')
    lines = ['# 恢复密钥匹配检查', '', f"状态：**{report['status']}**", '',
        '只验证固定范围内观测到的 enc:v1 密文；不是生产身份或恢复就绪证明。',
        f"显式授权读取密钥：{report['secret_read_authorized']}；尝试读取密钥内容：{report['secret_contents_read']}。",
        f"范围内匹配：{report['key_match_verified']}；副本清理：{report['copies_removed']}。",
        'restore_ready=false；production_auth_verified=false；kernel_started=false。',
        '不返回密钥、指纹、路径、密文或明文；错误密钥与损坏密文可能不可区分。', '']
    if 'error' in report:
        lines.append(f"错误阶段：{report['error']['phase']}；错误码：{report['error']['code']}。")
    if 'reason' in report:
        lines.append(f"结果原因：{report['reason']}。")
    with (output / 'keycheck.md').open('x') as file:
        os.chmod(file.name, 0o600)
        file.write('\n'.join(lines) + '\n')


def run(args):
    output = Path(args.out).absolute()
    output.mkdir(mode=0o700)
    os.chmod(output, 0o700)
    started = time.monotonic()
    deadline = started + args.timeout
    report = {'schema': SCHEMA, 'scope': 'observed-allowlisted-v1-ciphertexts', 'status': 'BLOCKED',
        'secret_read_authorized': args.allow_secret_read is True, 'secret_contents_read': False,
        'key_match_verified': False, 'restore_ready': False, 'production_auth_verified': False,
        'kernel_started': False, 'checksum_verified': False, 'observations': None,
        'authenticated_records': None, 'rejected_records': None,
        'manual_review': ['key_provenance_and_original_service_identity',
                          'other_secret_fields_versions_and_external_assets',
                          'production_authentication_and_real_rto_rpo']}
    phase = 'authorization'
    try:
        if args.allow_secret_read is not True:
            raise drill.DrillError('SECRET_READ_NOT_AUTHORIZED')
        if type(args.max_bytes) is not int or not 0 < args.max_bytes <= 512 * 1024 * 1024 or not math.isfinite(args.timeout) or not 0 < args.timeout <= 3600:
            raise drill.DrillError('LIMITS_INVALID')
        phase = 'worker'
        binary = worker_path(args.keycheck_bin)
        phase = 'asset_root'
        root_fd = os.open(args.assets_root, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW)
        try:
            mode = stat.S_IMODE(os.fstat(root_fd).st_mode)
            if mode & 0o077 or mode & 0o500 != 0o500:
                raise drill.DrillError('ASSET_ROOT_PERMISSIONS')
            phase = 'key_metadata'
            with inventory.open_beneath(root_fd, args.key_path) as key_fd:
                validate_key_metadata(key_fd)
                with tempfile.TemporaryDirectory(prefix='.keycheck-', dir=output) as temporary:
                    work = Path(temporary)
                    staged = work / 'snapshot.db'
                    phase = 'snapshot'
                    size, digest = drill.acquire(argparse.Namespace(backup=args.backup, url=None, max_bytes=args.max_bytes), staged, deadline)
                    report['snapshot_bytes'], report['snapshot_sha256'] = size, digest
                    if args.sha256 is not None:
                        if digest != args.sha256.lower():
                            raise drill.DrillError('CHECKSUM_MISMATCH')
                        report['checksum_verified'] = True
                    observations, values = inspect_snapshot(staged, deadline)
                    report['observations'] = observations
                    if not observations['cipher_scan_complete']:
                        report.update(status='INCONCLUSIVE', reason='CIPHER_SCAN_INCOMPLETE')
                    elif not values:
                        report.update(status='INCONCLUSIVE', reason='NO_SUPPORTED_CIPHERTEXT')
                    else:
                        phase = 'key_read'
                        master = read_master(key_fd, report, deadline)
                        phase = 'key_match'
                        result = invoke_worker(binary, master, values, work, deadline)
                        report['authenticated_records'], report['rejected_records'] = result['authenticated'], result['rejected']
                        if result['rejected']:
                            report.update(status='NOT_VERIFIED', reason='KEY_MISMATCH_OR_CORRUPT_CIPHERTEXT')
                        else:
                            report.update(status='MATCHED_WITHIN_SCOPE', key_match_verified=True)
                    drill.check_deadline(deadline)
        finally:
            os.close(root_fd)
    except (Exception, KeyboardInterrupt) as error:
        report.update(status='BLOCKED', key_match_verified=False)
        report['error'] = {'phase': phase, 'code': drill.error_code(error, phase, deadline)}
    report['copies_removed'] = not any(output.glob('.keycheck-*'))
    if not report['copies_removed']:
        report.update(status='BLOCKED', key_match_verified=False, cleanup_error='COPY_CLEANUP_FAILED')
    report['elapsed_seconds'] = round(time.monotonic() - started, 6)
    write_reports(output, report)
    return report


class SafeParser(argparse.ArgumentParser):
    def error(self, message):
        # Unknown arguments may accidentally contain secret values. Never echo them.
        self.exit(2, 'KEYCHECK_USAGE_INVALID\n')


def main(argv=None):
    parser = SafeParser(description=__doc__)
    parser.add_argument('--backup', required=True)
    parser.add_argument('--assets-root', required=True)
    parser.add_argument('--key-path', required=True, help='暂存根内的规范相对路径，不是密钥值')
    parser.add_argument('--keycheck-bin', required=True, help='本项目构建的可信 fp-restore-keycheck，不是生产内核')
    parser.add_argument('--allow-secret-read', action='store_true', help='显式授权本次读取暂存主密钥，仅经匿名管道处理')
    parser.add_argument('--out', required=True)
    parser.add_argument('--max-bytes', type=int, default=512 * 1024 * 1024)
    parser.add_argument('--timeout', type=float, default=120)
    parser.add_argument('--sha256')
    args = parser.parse_args(argv)
    if args.sha256 is not None and (len(args.sha256) != 64 or any(c not in '0123456789abcdefABCDEF' for c in args.sha256)):
        parser.error('invalid checksum')
    def interrupt(signum, frame):
        # selectors deliberately catches InterruptedError; use a non-OSError cancellation.
        raise KeyboardInterrupt
    signal.signal(signal.SIGTERM, interrupt)
    try:
        report = run(args)
    except (OSError, KeyboardInterrupt, InterruptedError, ValueError):
        print('KEYCHECK_REPORT_UNAVAILABLE')
        return 2
    print('RECOVERY_KEYCHECK: ' + report['status'])
    return 0 if report['status'] == 'MATCHED_WITHIN_SCOPE' else 1


if __name__ == '__main__':
    raise SystemExit(main())
