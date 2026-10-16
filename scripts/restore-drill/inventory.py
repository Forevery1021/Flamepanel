#!/usr/bin/env python3
"""R-01C: read-only staged recovery-asset inventory. Never reads secret file contents."""
import argparse
import contextlib
import json
import os
from pathlib import Path
import stat
import tempfile
import time

import drill

SCHEMA = 'flamepanel/recovery-assets/v1'
# kind, sensitive (permissions must not grant group/other access)
ROLES = {
    'deployment_config': ('file', True),
    'encryption_master': ('file', True),
    'jwt_signing': ('file', True),
    'bootstrap_token': ('file', True),
    'acme_account': ('file', True),
    'certificate_bundle': ('directory', True),
    'website_data': ('directory', False),
    'project_data': ('directory', False),
    'application_data': ('directory', False),
    'external_database_backups': ('directory', True),
    'agent_material': ('directory', True),
}
# Audited references only; no values/paths/IDs selected from the database.
CIPHERS = (
    ('backup_accounts', 'config_json', None),
    ('dns_provider_accounts', 'credentials_json', None),
    ('user_two_factor', 'secret', None),
    ('user_two_factor', 'pending_secret', None),
    ('panel_settings', 'value', "key = 'assistant.api_key'"),
)
RESOURCE_ROLES = {
    'websites': 'website_data', 'projects': 'project_data',
    'installed_apps': 'application_data', 'databases': 'external_database_backups',
    'nodes': 'agent_material', 'certificates': 'certificate_bundle',
}


def relative_parts(value):
    if not isinstance(value, str) or not 0 < len(value) <= 1024:
        raise drill.DrillError('MANIFEST_INVALID')
    parts = value.split('/')
    if any(p in ('', '.', '..') for p in parts) or '\\' in value or any(ord(c) < 32 or 127 <= ord(c) <= 159 for c in value):
        raise drill.DrillError('MANIFEST_INVALID')
    return parts


def load_manifest(path):
    fd = os.open(path, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK)
    with os.fdopen(fd, 'rb') as source:
        info = os.fstat(source.fileno())
        if not stat.S_ISREG(info.st_mode) or info.st_size > 65536:
            raise drill.DrillError('MANIFEST_INVALID')
        raw = source.read(65537)
    if len(raw) > 65536:
        raise drill.DrillError('MANIFEST_INVALID')
    def unique(pairs):
        result = {}
        for key, value in pairs:
            if key in result:
                raise drill.DrillError('MANIFEST_INVALID')
            result[key] = value
        return result
    try:
        manifest = json.loads(raw, object_pairs_hook=unique)
    except (ValueError, UnicodeError) as error:
        raise drill.DrillError('MANIFEST_INVALID') from error
    if not isinstance(manifest, dict) or set(manifest) != {'schema', 'jwt_strategy', 'assets'}:
        raise drill.DrillError('MANIFEST_INVALID')
    if manifest['schema'] != SCHEMA or manifest['jwt_strategy'] not in ('preserve', 'reissue'):
        raise drill.DrillError('MANIFEST_INVALID')
    assets = manifest['assets']
    if not isinstance(assets, list) or len(assets) > len(ROLES):
        raise drill.DrillError('MANIFEST_INVALID')
    seen, paths = set(), set()
    for asset in assets:
        if not isinstance(asset, dict) or set(asset) != {'role', 'path'}:
            raise drill.DrillError('MANIFEST_INVALID')
        role, path = asset['role'], asset['path']
        if not isinstance(role, str) or role not in ROLES or role in seen:
            raise drill.DrillError('MANIFEST_INVALID')
        relative_parts(path)
        if path in paths:
            raise drill.DrillError('MANIFEST_INVALID')
        seen.add(role)
        paths.add(path)
    return manifest


@contextlib.contextmanager
def open_beneath(root_fd, relative):
    """Descriptor-relative walk. No component may be a symlink; no recursive directory scan."""
    parts = relative_parts(relative)
    parent = os.dup(root_fd)
    result = None
    try:
        for part in parts[:-1]:
            child = os.open(part, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW, dir_fd=parent)
            os.close(parent)
            parent = child
        result = os.open(parts[-1], os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK, dir_fd=parent)
        yield result
    finally:
        if result is not None:
            os.close(result)
        os.close(parent)


def inspect_asset(root_fd, asset):
    role = asset['role']
    kind, sensitive = ROLES[role]
    try:
        with open_beneath(root_fd, asset['path']) as fd:
            info = os.fstat(fd)  # Intentionally no read, decode, hash, fingerprint or copy.
            mode = stat.S_IMODE(info.st_mode)
            if kind == 'file' and not stat.S_ISREG(info.st_mode):
                return 'WRONG_TYPE'
            if kind == 'directory' and not stat.S_ISDIR(info.st_mode):
                return 'WRONG_TYPE'
            if kind == 'file' and info.st_nlink != 1:
                return 'HARDLINK_REFUSED'
            if sensitive and mode & 0o077:
                return 'UNSAFE_PERMISSIONS'
            needed = 0o400 if kind == 'file' else 0o500
            if mode & needed != needed:
                return 'OWNER_ACCESS_MISSING'
            if kind == 'file' and info.st_size == 0:
                return 'EMPTY_FILE'
            return 'PRESENT_UNVERIFIED'
    except FileNotFoundError:
        return 'MISSING'
    except OSError:
        return 'UNSAFE_OR_UNREADABLE_PATH'


def inspect_snapshot(staged, deadline):
    drill.validate(staged, deadline)
    observations = {'encrypted_fields': {}, 'resource_counts': {}, 'coverage_gaps': []}
    with drill.reader(staged, deadline) as conn:
        def columns(table):
            row = conn.execute(
                "SELECT type FROM pragma_table_list WHERE schema='main' AND name=?", (table,)).fetchone()
            if not row or row[0] != 'table':
                return set()
            return {r[1] for r in conn.execute(f'PRAGMA table_info("{table}")')}
        for table, column, condition in CIPHERS:
            drill.check_deadline(deadline)
            label = f'{table}.{column}'
            required = {column, 'key'} if condition else {column}
            if not required.issubset(columns(table)):
                observations['coverage_gaps'].append(label)
                continue
            where = (' AND ' + condition) if condition else ''
            # Includes unknown enc:* versions, rather than silently treating them as plaintext.
            count = conn.execute(
                f'SELECT COUNT(*) FROM "{table}" WHERE substr("{column}",1,4)=\'enc:\'{where}').fetchone()[0]
            observations['encrypted_fields'][label] = count
            non_text = conn.execute(
                f'SELECT COUNT(*) FROM "{table}" WHERE "{column}" IS NOT NULL AND typeof("{column}") != \'text\'{where}').fetchone()[0]
            if non_text:
                observations['coverage_gaps'].append(label + '.non_text')
        for table in RESOURCE_ROLES:
            drill.check_deadline(deadline)
            if not columns(table):
                observations['coverage_gaps'].append(table)
                continue
            observations['resource_counts'][table] = conn.execute(f'SELECT COUNT(*) FROM "{table}"').fetchone()[0]
        if 'source' in columns('certificates'):
            observations['acme_records'] = conn.execute(
                "SELECT COUNT(*) FROM certificates WHERE source NOT IN ('uploaded','self_signed') OR source IS NULL"
            ).fetchone()[0]
        else:
            observations['acme_records'] = None
            observations['coverage_gaps'].append('certificates.source')
    observations['cipher_scan_complete'] = all(
        f'{table}.{column}' in observations['encrypted_fields']
        and f'{table}.{column}.non_text' not in observations['coverage_gaps']
        for table, column, _ in CIPHERS)
    return observations


def evaluate(manifest, observations, root_fd, deadline):
    required = {'deployment_config'}
    if manifest['jwt_strategy'] == 'preserve':
        required.add('jwt_signing')
    if sum(observations['encrypted_fields'].values()) or not observations['cipher_scan_complete']:
        required.add('encryption_master')
    for table, role in RESOURCE_ROLES.items():
        count = observations['resource_counts'].get(table)
        # Unknown tables are a review gap, not proof of absence or an automatic external path scan.
        if count:
            required.add(role)
    if observations['resource_counts'].get('nodes', 0):
        required.add('bootstrap_token')
    if observations['acme_records']:
        required.add('acme_account')
    declared = {asset['role']: asset for asset in manifest['assets']}
    results = []
    for role in ROLES:
        drill.check_deadline(deadline)
        status = inspect_asset(root_fd, declared[role]) if role in declared else 'NOT_DECLARED'
        results.append({'role': role, 'required_by_observation': role in required, 'status': status})
    blocked = any((item['required_by_observation'] or item['role'] in declared)
                  and item['status'] != 'PRESENT_UNVERIFIED' for item in results)
    return results, 'BLOCKED' if blocked else 'REVIEW_REQUIRED'


def run(args):
    output = Path(args.out).absolute()
    output.mkdir(mode=0o700)
    deadline = time.monotonic() + args.timeout
    started = time.monotonic()
    report = {
        'schema': 'flamepanel/recovery-inventory/v1', 'status': 'BLOCKED',
        'restore_ready': False, 'key_match_verified': False, 'secret_contents_read': False,
        'production_auth_verified': False, 'assets': [], 'observations': None,
        'manual_review': ['complete_recovery_asset_mapping', 'original_working_directory_and_overrides',
                          'key_to_ciphertext_match', 'directory_contents_and_external_snapshots',
                          'owners_acl_and_service_identity', 'production_authentication_and_rpo'],
    }
    phase = 'manifest'
    try:
        manifest = load_manifest(args.manifest)
        report['jwt_strategy'] = manifest['jwt_strategy']
        if manifest['jwt_strategy'] == 'reissue':
            report['manual_review'].append('approve_session_reissue_no_action_performed')
        phase = 'asset_root'
        root_fd = os.open(args.assets_root, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW)
        try:
            mode = stat.S_IMODE(os.fstat(root_fd).st_mode)
            if mode & 0o077 or mode & 0o500 != 0o500:
                raise drill.DrillError('ASSET_ROOT_PERMISSIONS')
            with tempfile.TemporaryDirectory(prefix='.inventory-', dir=output) as tmp:
                staged = Path(tmp) / 'snapshot.db'
                phase = 'snapshot'
                acquire_args = argparse.Namespace(backup=args.backup, url=None, max_bytes=args.max_bytes)
                size, digest = drill.acquire(acquire_args, staged, deadline)
                report['snapshot_bytes'], report['snapshot_sha256'] = size, digest
                observations = inspect_snapshot(staged, deadline)
                report['observations'] = observations
                phase = 'inventory'
                drill.check_deadline(deadline)
                report['assets'], report['status'] = evaluate(manifest, observations, root_fd, deadline)
                drill.check_deadline(deadline)
        finally:
            os.close(root_fd)
    except (Exception, KeyboardInterrupt) as error:
        report['error'] = {'phase': phase, 'code': drill.error_code(error, phase, deadline)}
        report['status'] = 'BLOCKED'
    report['copies_removed'] = not any(output.glob('.inventory-*'))
    if not report['copies_removed']:
        report['status'] = 'BLOCKED'
        report['cleanup_error'] = 'COPY_CLEANUP_FAILED'
    report['elapsed_seconds'] = round(time.monotonic() - started, 6)
    with (output / 'inventory.json').open('x') as file:
        os.chmod(file.name, 0o600)
        json.dump(report, file, ensure_ascii=False, indent=2)
        file.write('\n')
    lines = ['# 恢复资产清单检查', '', f"状态：**{report['status']}**", '',
             '不是恢复完成证明：密钥内容未读取、密钥匹配未验证、目录完整性未验证。', '',
             '| 资产角色 | 根据观测要求 | 状态 |', '| --- | --- | --- |']
    lines += [f"| {a['role']} | {a['required_by_observation']} | {a['status']} |" for a in report['assets']]
    if 'error' in report:
        lines += ['', f"失败阶段：{report['error']['phase']}；错误码：{report['error']['code']}。"]
    lines += ['', 'restore_ready=false；生产身份、RTO/RPO、外部系统均未验证。', '']
    with (output / 'inventory.md').open('x') as file:
        os.chmod(file.name, 0o600)
        file.write('\n'.join(lines))
    return report


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--backup', required=True)
    parser.add_argument('--assets-root', required=True, help='权限受控的独立资产暂存根目录')
    parser.add_argument('--manifest', required=True, help='只含角色和相对路径；禁止内嵌秘密值')
    parser.add_argument('--out', required=True, help='必须不存在的报告目录')
    parser.add_argument('--max-bytes', type=int, default=512 * 1024 * 1024)
    parser.add_argument('--timeout', type=float, default=120)
    args = parser.parse_args()
    if not 0 < args.max_bytes <= 8 * 1024**3 or not 0 < args.timeout <= 3600:
        parser.error('大小范围 1..8GiB，超时范围 0..3600 秒')
    import signal
    def interrupt(signum, frame):
        raise InterruptedError
    signal.signal(signal.SIGTERM, interrupt)
    try:
        report = run(args)
    except (OSError, KeyboardInterrupt, InterruptedError):
        print('INVENTORY_REPORT_UNAVAILABLE')
        return 2
    print('RECOVERY_INVENTORY: ' + report['status'])
    return 1 if report['status'] == 'BLOCKED' else 0


if __name__ == '__main__':
    raise SystemExit(main())
