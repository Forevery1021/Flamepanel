#!/usr/bin/env python3
"""R-01E-1: read-only report consistency checks and an identity-validation plan, never recovery approval."""
import argparse
import json
import math
import os
from pathlib import Path
import signal
import stat
import time

import drill
import inventory

SCHEMA = 'flamepanel/recovery-preflight/v1'
REPORT_LIMIT = 128 * 1024
MAX_COUNT = 2**63 - 1
CIPHER_FIELDS = {f'{table}.{column}' for table, column, _ in inventory.CIPHERS}
DRILL_FIELDS = {'schema', 'status', 'scope', 'kernel_started', 'migration_verified', 'probe_http_verified',
    'source_kind', 'checksum_verified', 'started_at', 'stages', 'production_rto_seconds', 'rpo_seconds',
    'backup_age_seconds', 'storage_rto_seconds', 'backup_time_basis', 'limitations', 'snapshot_bytes',
    'snapshot_sha256', 'sampled_rows', 'probe', 'copies_removed', 'elapsed_seconds', 'cleanup_error'}
INVENTORY_FIELDS = {'schema', 'status', 'restore_ready', 'key_match_verified', 'secret_contents_read',
    'production_auth_verified', 'assets', 'observations', 'manual_review', 'jwt_strategy', 'error',
    'copies_removed', 'cleanup_error', 'elapsed_seconds', 'snapshot_bytes', 'snapshot_sha256'}
KEYCHECK_FIELDS = {'schema', 'scope', 'status', 'secret_read_authorized', 'secret_contents_read',
    'key_match_verified', 'restore_ready', 'production_auth_verified', 'kernel_started', 'checksum_verified',
    'observations', 'authenticated_records', 'rejected_records', 'manual_review', 'snapshot_bytes',
    'snapshot_sha256', 'copies_removed', 'elapsed_seconds', 'error', 'reason', 'cleanup_error'}


def require(condition, code):
    if not condition:
        raise drill.DrillError(code)


def shape(value, allowed, required=None):
    require(isinstance(value, dict), 'EVIDENCE_SHAPE_INVALID')
    require(not set(value) - allowed, 'EVIDENCE_FIELDS_INVALID')
    require((required if required is not None else allowed).issubset(value), 'EVIDENCE_FIELDS_MISSING')


def flag(data, name, expected):
    require(name in data and data[name] is expected, 'EVIDENCE_FLAG_INVALID')


def count(value):
    require(type(value) is int and 0 <= value <= MAX_COUNT, 'EVIDENCE_COUNT_INVALID')
    return value


def seconds(value):
    require(type(value) in (int, float) and math.isfinite(value) and 0 <= value <= 10000000,
            'EVIDENCE_TIME_INVALID')


def root_report(data, schema, allowed, required):
    shape(data, allowed, required | {'schema', 'status', 'snapshot_sha256', 'snapshot_bytes', 'copies_removed', 'elapsed_seconds'})
    require(data['schema'] == schema, 'EVIDENCE_SCHEMA_UNSUPPORTED')
    require(not {'error', 'cleanup_error'} & set(data), 'EVIDENCE_FAILURE_PRESENT')
    flag(data, 'copies_removed', True)
    seconds(data['elapsed_seconds'])
    digest = data['snapshot_sha256']
    require(isinstance(digest, str) and len(digest) == 64 and all(c in '0123456789abcdefABCDEF' for c in digest),
            'SNAPSHOT_DIGEST_INVALID')
    require(type(data['snapshot_bytes']) is int and 0 < data['snapshot_bytes'] <= 8 * 1024**3,
            'SNAPSHOT_SIZE_INVALID')
    return digest.lower(), data['snapshot_bytes']


def validate_drill(data):
    snapshot = root_report(data, 'flamepanel/restore-drill/v1', DRILL_FIELDS,
        {'scope', 'kernel_started', 'migration_verified', 'probe_http_verified', 'source_kind',
         'checksum_verified', 'stages', 'probe', 'sampled_rows', 'production_rto_seconds', 'rpo_seconds', 'storage_rto_seconds'})
    require(data['status'] == 'PASS' and data['scope'] == 'sqlite-with-dedicated-probe', 'PROBE_EVIDENCE_REQUIRED')
    flag(data, 'kernel_started', False)
    for name in ('migration_verified', 'probe_http_verified'):
        flag(data, name, True)
    require(type(data['checksum_verified']) is bool and data['source_kind'] in ('local', 'https'), 'EVIDENCE_FLAG_INVALID')
    require(data['production_rto_seconds'] is None and data['rpo_seconds'] is None, 'PRODUCTION_CLAIM_REFUSED')
    seconds(data['storage_rto_seconds'])
    expected = set(drill.STAGES) | {'probe'}
    shape(data['stages'], expected)
    for stage in data['stages'].values():
        shape(stage, {'status', 'seconds'})
        require(stage['status'] == 'PASS', 'DRILL_STAGE_NOT_PASSED')
        seconds(stage['seconds'])
    shape(data['sampled_rows'], set(drill.SCHEMA))
    for value in data['sampled_rows'].values():
        require(count(value) <= 20, 'SAMPLE_COUNT_INVALID')
    probe = data['probe']
    shape(probe, {'mode', 'migration_passed', 'http_auth_passed', 'write_routes_absent', 'counts',
                  'production_kernel', 'production_auth_verified', 'probe_seconds', 'stopped_cleanly'})
    require(probe['mode'] == 'flamepanel/restore-probe/v1', 'PROBE_MODE_INVALID')
    for name in ('migration_passed', 'http_auth_passed', 'write_routes_absent', 'stopped_cleanly'):
        flag(probe, name, True)
    for name in ('production_kernel', 'production_auth_verified'):
        flag(probe, name, False)
    seconds(probe['probe_seconds'])
    shape(probe['counts'], {'users', 'websites', 'scheduled_tasks'})
    for value in probe['counts'].values():
        count(value)
    return snapshot


def validate_inventory(data, strategy):
    snapshot = root_report(data, 'flamepanel/recovery-inventory/v1', INVENTORY_FIELDS,
        {'restore_ready', 'key_match_verified', 'secret_contents_read', 'production_auth_verified',
         'assets', 'observations', 'jwt_strategy'})
    require(data['status'] == 'REVIEW_REQUIRED', 'INVENTORY_NOT_REVIEWABLE')
    for name in ('restore_ready', 'key_match_verified', 'secret_contents_read', 'production_auth_verified'):
        flag(data, name, False)
    require(data['jwt_strategy'] in ('preserve', 'reissue') and data['jwt_strategy'] == strategy, 'JWT_PLAN_MISMATCH')
    observed = data['observations']
    shape(observed, {'encrypted_fields', 'resource_counts', 'coverage_gaps', 'acme_records', 'cipher_scan_complete'})
    flag(observed, 'cipher_scan_complete', True)
    require(observed['coverage_gaps'] == [], 'INVENTORY_COVERAGE_INCOMPLETE')
    shape(observed['encrypted_fields'], CIPHER_FIELDS)
    encrypted = {name: count(value) for name, value in observed['encrypted_fields'].items()}
    shape(observed['resource_counts'], set(inventory.RESOURCE_ROLES))
    resources = {name: count(value) for name, value in observed['resource_counts'].items()}
    acme = count(observed['acme_records'])
    require(acme <= resources['certificates'], 'EVIDENCE_COUNT_CONFLICT')
    total = sum(encrypted.values())
    require(total <= MAX_COUNT, 'EVIDENCE_COUNT_INVALID')
    required = {'deployment_config'}
    if strategy == 'preserve':
        required.add('jwt_signing')
    if total:
        required.add('encryption_master')
    required.update(role for table, role in inventory.RESOURCE_ROLES.items() if resources[table])
    if resources['nodes']:
        required.add('bootstrap_token')
    if acme:
        required.add('acme_account')
    require(isinstance(data['assets'], list) and len(data['assets']) == len(inventory.ROLES), 'ASSET_EVIDENCE_INVALID')
    roles = set()
    for asset in data['assets']:
        shape(asset, {'role', 'required_by_observation', 'status'})
        role = asset['role']
        require(isinstance(role, str) and role in inventory.ROLES and role not in roles, 'ASSET_EVIDENCE_INVALID')
        roles.add(role)
        require(type(asset['required_by_observation']) is bool and asset['required_by_observation'] == (role in required),
                'ASSET_REQUIREMENT_CONFLICT')
        require(asset['status'] in ('PRESENT_UNVERIFIED', 'NOT_DECLARED'), 'ASSET_EVIDENCE_BLOCKED')
        if role in required:
            require(asset['status'] == 'PRESENT_UNVERIFIED', 'REQUIRED_ASSET_NOT_PRESENT')
    return snapshot, encrypted


def validate_keycheck(data, encrypted):
    snapshot = root_report(data, 'flamepanel/recovery-keycheck/v1', KEYCHECK_FIELDS,
        {'scope', 'secret_read_authorized', 'secret_contents_read', 'key_match_verified', 'restore_ready',
         'production_auth_verified', 'kernel_started', 'checksum_verified', 'observations',
         'authenticated_records', 'rejected_records'})
    require(data['scope'] == 'observed-allowlisted-v1-ciphertexts', 'KEYCHECK_SCOPE_INVALID')
    for name in ('restore_ready', 'production_auth_verified', 'kernel_started'):
        flag(data, name, False)
    flag(data, 'secret_read_authorized', True)
    require(type(data['checksum_verified']) is bool, 'EVIDENCE_FLAG_INVALID')
    observed = data['observations']
    shape(observed, {'encrypted_fields', 'coverage_gaps', 'unsupported_ciphertexts', 'legacy_plaintext_records',
                     'cipher_scan_complete', 'supported_ciphertexts'})
    flag(observed, 'cipher_scan_complete', True)
    require(observed['coverage_gaps'] == [] and count(observed['unsupported_ciphertexts']) == 0, 'KEYCHECK_SCAN_INCOMPLETE')
    count(observed['legacy_plaintext_records'])
    shape(observed['encrypted_fields'], CIPHER_FIELDS)
    checked_fields = {name: count(value) for name, value in observed['encrypted_fields'].items()}
    total = sum(encrypted.values())
    require(checked_fields == encrypted and count(observed['supported_ciphertexts']) == total, 'CIPHER_COUNT_CONFLICT')
    if total:
        require(data['status'] == 'MATCHED_WITHIN_SCOPE' and 'reason' not in data, 'KEYCHECK_NOT_MATCHED')
        flag(data, 'secret_contents_read', True)
        flag(data, 'key_match_verified', True)
        require(count(data['authenticated_records']) == total and count(data['rejected_records']) == 0, 'KEYCHECK_COUNT_CONFLICT')
    else:
        require(data['status'] == 'INCONCLUSIVE' and data.get('reason') == 'NO_SUPPORTED_CIPHERTEXT', 'EMPTY_CIPHER_EVIDENCE_INVALID')
        flag(data, 'secret_contents_read', False)
        flag(data, 'key_match_verified', False)
        require(data['authenticated_records'] is None and data['rejected_records'] is None, 'EMPTY_CIPHER_EVIDENCE_INVALID')
    return snapshot


def load_report(root_fd, relative, deadline):
    def unique(pairs):
        data = {}
        for name, value in pairs:
            require(name not in data, 'DUPLICATE_REPORT_FIELD')
            data[name] = value
        return data
    drill.check_deadline(deadline)
    with inventory.open_beneath(root_fd, relative) as descriptor:
        before = os.fstat(descriptor)
        mode = stat.S_IMODE(before.st_mode)
        require(stat.S_ISREG(before.st_mode) and before.st_nlink == 1, 'PRIVATE_REPORT_FILE_REQUIRED')
        require(not mode & 0o077 and bool(mode & 0o400), 'REPORT_PERMISSIONS')
        require(0 < before.st_size <= REPORT_LIMIT, 'REPORT_SIZE_LIMIT')
        with os.fdopen(os.dup(descriptor), 'rb') as source:
            raw = source.read(REPORT_LIMIT + 1)
        after = os.fstat(descriptor)
        require(len(raw) == before.st_size and len(raw) <= REPORT_LIMIT, 'REPORT_CHANGED_OR_OVERSIZED')
        require(all(getattr(before, name) == getattr(after, name) for name in ('st_size', 'st_mtime_ns', 'st_ctime_ns', 'st_mode', 'st_nlink')),
                'REPORT_CHANGED_OR_OVERSIZED')
    drill.check_deadline(deadline)
    try:
        return json.loads(raw, object_pairs_hook=unique)
    except (ValueError, UnicodeError, RecursionError) as error:
        raise drill.DrillError('REPORT_JSON_INVALID') from error


def write_reports(output, report):
    with (output / 'preflight.json').open('x') as file:
        os.chmod(file.name, 0o600)
        json.dump(report, file, ensure_ascii=False, indent=2)
        file.write('\n')
    lines = ['# 恢复证据与身份前置计划', '', f"状态：**{report['status']}**", '',
        '仅检查调用者提供报告的结构与一致性，不验证报告真实性或生产身份，不是恢复授权。',
        '未打开资产秘密文件、数据库或外部服务；未启动生产内核。', '',
        '| 检查 | 结果 |', '| --- | --- |']
    lines += [f'| {name} | {value} |' for name, value in report['checks'].items()]
    if 'error' in report:
        lines += ['', f"阻断阶段：{report['error']['phase']}；错误码：{report['error']['code']}。"]
    if 'jwt_strategy' in report:
        lines += ['', f"拟议 JWT 策略：{report['jwt_strategy']}（仅计划，没有会话或密钥操作）。"]
    lines += ['', 'restore_ready=false；production_auth_verified=false；key_match_verified=false。',
        '报告真实性与新鲜度未验证；生产 RTO/RPO 未知，后续仍需独立审批和验收。', '']
    with (output / 'preflight.md').open('x') as file:
        os.chmod(file.name, 0o600)
        file.write('\n'.join(lines))


def run(args):
    output = Path(args.out).absolute()
    output.mkdir(mode=0o700)
    os.chmod(output, 0o700)
    started = time.monotonic()
    deadline = started + args.timeout
    report = {'schema': SCHEMA, 'status': 'BLOCKED', 'scope': 'supplied-report-consistency-only',
        'evidence_consistent': False, 'report_authenticity_verified': False, 'freshness_verified': False,
        'secret_contents_read': False, 'asset_secret_files_opened': False, 'database_opened': False,
        'kernel_started': False, 'sessions_modified': False, 'keys_generated_or_rotated': False,
        'restore_ready': False, 'key_match_verified': False,
        'production_auth_verified': False, 'production_rto_seconds': None, 'rpo_seconds': None,
        'checks': {name: 'NOT_RUN' for name in ('drill', 'inventory', 'keycheck', 'snapshot_binding', 'jwt_plan')},
        'next_actions': ['approve_identity_validation_boundary', 'review_jwt_preserve_or_reissue',
                         'verify_business_routes_and_external_assets', 'approve_startup_isolation',
                         'measure_real_production_rto_rpo']}
    phase = 'arguments'
    try:
        require(type(args.timeout) in (int, float) and math.isfinite(args.timeout) and 0 < args.timeout <= 300, 'LIMITS_INVALID')
        require(args.jwt_strategy in ('preserve', 'reissue'), 'JWT_PLAN_INVALID')
        paths = [args.drill_report, args.inventory_report] + ([args.keycheck_report] if args.keycheck_report is not None else [])
        require(len(set(paths)) == len(paths), 'DUPLICATE_REPORT_PATH')
        for path in paths:
            inventory.relative_parts(path)
        phase = 'evidence_root'
        descriptor = os.open(args.evidence_root, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW)
        try:
            mode = stat.S_IMODE(os.fstat(descriptor).st_mode)
            require(not mode & 0o077 and mode & 0o500 == 0o500, 'EVIDENCE_ROOT_PERMISSIONS')
            phase = 'drill'
            first = validate_drill(load_report(descriptor, args.drill_report, deadline))
            report['checks']['drill'] = 'PROBE_CONTRACT_PASSED'
            phase = 'inventory'
            second, encrypted = validate_inventory(load_report(descriptor, args.inventory_report, deadline), args.jwt_strategy)
            report['checks']['inventory'] = 'METADATA_REVIEWABLE'
            phase = 'keycheck'
            total = sum(encrypted.values())
            require(not total or args.keycheck_report is not None, 'KEYCHECK_EVIDENCE_REQUIRED')
            third = validate_keycheck(load_report(descriptor, args.keycheck_report, deadline), encrypted) if args.keycheck_report is not None else None
            report['checks']['keycheck'] = 'SCOPED_MATCH_REPORTED' if total else 'NO_ENCRYPTED_REFERENCE_OBSERVED'
            phase = 'snapshot_binding'
            require(first == second and (third is None or third == first), 'SNAPSHOT_BINDING_MISMATCH')
            report['checks']['snapshot_binding'] = 'SAME_SOURCE_DIGEST_AND_SIZE'
            phase = 'jwt_plan'
            report['jwt_strategy'] = args.jwt_strategy
            report['checks']['jwt_plan'] = 'PLAN_ONLY_APPROVAL_STILL_REQUIRED'
            if args.jwt_strategy == 'reissue':
                report['next_actions'].append('approve_session_reissue_no_action_performed')
            report['source_snapshot_sha256'], report['source_snapshot_bytes'] = first
            report['encrypted_references'] = total
            drill.check_deadline(deadline)
            report.update(status='REVIEW_REQUIRED', evidence_consistent=True)
        finally:
            os.close(descriptor)
    except (Exception, KeyboardInterrupt) as error:
        report['error'] = {'phase': phase, 'code': drill.error_code(error, phase, deadline)}
        report.update(status='BLOCKED', evidence_consistent=False)
    report['elapsed_seconds'] = round(time.monotonic() - started, 6)
    write_reports(output, report)
    return report


class SafeParser(argparse.ArgumentParser):
    def error(self, message):
        self.exit(2, 'PREFLIGHT_USAGE_INVALID\n')


def main(argv=None):
    parser = SafeParser(description=__doc__)
    parser.add_argument('--evidence-root', required=True)
    parser.add_argument('--drill-report', required=True, help='根内相对路径，不是数据库路径')
    parser.add_argument('--inventory-report', required=True)
    parser.add_argument('--keycheck-report')
    parser.add_argument('--jwt-strategy', required=True, choices=('preserve', 'reissue'))
    parser.add_argument('--out', required=True)
    parser.add_argument('--timeout', type=float, default=30)
    args = parser.parse_args(argv)
    def interrupt(signum, frame):
        raise KeyboardInterrupt
    signal.signal(signal.SIGTERM, interrupt)
    try:
        report = run(args)
    except (OSError, KeyboardInterrupt, InterruptedError, ValueError):
        print('PREFLIGHT_REPORT_UNAVAILABLE')
        return 2
    print('RECOVERY_PREFLIGHT: ' + report['status'])
    return 0 if report['status'] == 'REVIEW_REQUIRED' else 1


if __name__ == '__main__':
    raise SystemExit(main())
