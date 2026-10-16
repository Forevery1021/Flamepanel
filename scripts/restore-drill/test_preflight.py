import argparse
import copy
import json
import os
from pathlib import Path
import sqlite3
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

import preflight


def evidence(strategy='preserve', encrypted=1):
    common = {'snapshot_sha256': 'a' * 64, 'snapshot_bytes': 16384, 'copies_removed': True, 'elapsed_seconds': 1.0}
    drill = common | {
        'schema': 'flamepanel/restore-drill/v1', 'status': 'PASS', 'scope': 'sqlite-with-dedicated-probe',
        'kernel_started': False, 'migration_verified': True, 'probe_http_verified': True,
        'source_kind': 'local', 'checksum_verified': False,
        'production_rto_seconds': None, 'rpo_seconds': None, 'storage_rto_seconds': 0.1,
        'stages': {name: {'status': 'PASS', 'seconds': 0.01} for name in (*preflight.drill.STAGES, 'probe')},
        'sampled_rows': {name: 0 for name in preflight.drill.SCHEMA},
        'probe': {'mode': 'flamepanel/restore-probe/v1', 'migration_passed': True, 'http_auth_passed': True,
                  'write_routes_absent': True, 'counts': {'users': 0, 'websites': 0, 'scheduled_tasks': 0},
                  'production_kernel': False, 'production_auth_verified': False,
                  'probe_seconds': 0.1, 'stopped_cleanly': True}}
    fields = {name: encrypted if name == 'backup_accounts.config_json' else 0 for name in preflight.CIPHER_FIELDS}
    required = {'deployment_config'} | ({'jwt_signing'} if strategy == 'preserve' else set()) | ({'encryption_master'} if encrypted else set())
    inventory = common | {
        'schema': 'flamepanel/recovery-inventory/v1', 'status': 'REVIEW_REQUIRED', 'restore_ready': False,
        'key_match_verified': False, 'secret_contents_read': False, 'production_auth_verified': False,
        'jwt_strategy': strategy,
        'assets': [{'role': name, 'required_by_observation': name in required,
                    'status': 'PRESENT_UNVERIFIED' if name in required else 'NOT_DECLARED'}
                   for name in preflight.inventory.ROLES],
        'observations': {'encrypted_fields': dict(fields), 'resource_counts': {name: 0 for name in preflight.inventory.RESOURCE_ROLES},
                         'coverage_gaps': [], 'acme_records': 0, 'cipher_scan_complete': True}}
    keycheck = common | {
        'schema': 'flamepanel/recovery-keycheck/v1', 'scope': 'observed-allowlisted-v1-ciphertexts',
        'status': 'MATCHED_WITHIN_SCOPE' if encrypted else 'INCONCLUSIVE',
        'secret_read_authorized': True, 'secret_contents_read': bool(encrypted), 'key_match_verified': bool(encrypted),
        'restore_ready': False, 'production_auth_verified': False, 'kernel_started': False, 'checksum_verified': False,
        'observations': {'encrypted_fields': dict(fields), 'coverage_gaps': [], 'unsupported_ciphertexts': 0,
                         'legacy_plaintext_records': 0, 'cipher_scan_complete': True, 'supported_ciphertexts': encrypted},
        'authenticated_records': encrypted if encrypted else None, 'rejected_records': 0 if encrypted else None}
    if not encrypted:
        keycheck['reason'] = 'NO_SUPPORTED_CIPHERTEXT'
    return {'drill': drill, 'inventory': inventory, 'keycheck': keycheck}


class PreflightTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name)
        self.inputs = self.root / 'evidence'
        self.inputs.mkdir(mode=0o700)
        self.data = evidence()
        self.args = argparse.Namespace(evidence_root=str(self.inputs), drill_report='drill.json',
            inventory_report='inventory.json', keycheck_report='keycheck.json', jwt_strategy='preserve',
            out=str(self.root / 'preflight'), timeout=10)

    def write_inputs(self):
        for role, data in self.data.items():
            file = self.inputs / (role + '.json')
            file.write_text(json.dumps(data))
            file.chmod(0o600)

    def run_case(self, expected='BLOCKED', write=True):
        if write:
            self.write_inputs()
        before = {file.name: file.read_bytes() for file in self.inputs.iterdir() if file.is_file() and not file.is_symlink()}
        result = preflight.run(self.args)
        self.assertEqual(result['status'], expected, result)
        for field in ['restore_ready', 'production_auth_verified', 'key_match_verified', 'kernel_started',
                      'secret_contents_read', 'asset_secret_files_opened', 'database_opened', 'sessions_modified',
                      'keys_generated_or_rotated', 'report_authenticity_verified', 'freshness_verified']:
            self.assertIs(result[field], False)
        self.assertIsNone(result['production_rto_seconds'])
        self.assertIsNone(result['rpo_seconds'])
        for name, original in before.items():
            self.assertEqual((self.inputs / name).read_bytes(), original)
        self.assertEqual(Path(self.args.out).stat().st_mode & 0o777, 0o700)
        for file in Path(self.args.out).iterdir():
            self.assertEqual(file.stat().st_mode & 0o777, 0o600)
            text = file.read_text()
            for private in ['PRIVATE_REPORT_NOTE_CANARY', str(self.inputs), str(self.root), 'secret-source-path']:
                self.assertNotIn(private, text)
        return result

    def test_complete_consistent_chain_remains_review_only_not_authority(self):
        report = self.run_case('REVIEW_REQUIRED')
        self.assertTrue(report['evidence_consistent'])
        self.assertEqual(report['source_snapshot_sha256'], 'a' * 64)
        self.assertEqual(report['checks']['keycheck'], 'SCOPED_MATCH_REPORTED')

    def test_reissue_is_only_a_plan_not_revocation_or_key_generation(self):
        self.data = evidence(strategy='reissue')
        self.args.jwt_strategy = 'reissue'
        report = self.run_case('REVIEW_REQUIRED')
        self.assertIn('approve_session_reissue_no_action_performed', report['next_actions'])

    def test_no_ciphertext_does_not_require_a_fake_key_match(self):
        self.data = evidence(encrypted=0)
        self.args.keycheck_report = None
        report = self.run_case('REVIEW_REQUIRED')
        self.assertEqual(report['checks']['keycheck'], 'NO_ENCRYPTED_REFERENCE_OBSERVED')
        self.assertEqual(report['encrypted_references'], 0)

    def test_optional_zero_ciphertext_report_can_only_be_inconclusive(self):
        self.data = evidence(encrypted=0)
        self.run_case('REVIEW_REQUIRED')

    def test_uppercase_digest_normalizes_without_changing_binding(self):
        self.data['keycheck']['snapshot_sha256'] = 'A' * 64
        self.run_case('REVIEW_REQUIRED')

    def test_mixed_snapshot_digest_is_blocked(self):
        self.data['keycheck']['snapshot_sha256'] = 'b' * 64
        report = self.run_case()
        self.assertEqual(report['error']['code'], 'SNAPSHOT_BINDING_MISMATCH')
        self.assertFalse(report['evidence_consistent'])

    def test_mixed_snapshot_size_is_blocked(self):
        self.data['inventory']['snapshot_bytes'] += 1
        self.assertEqual(self.run_case()['error']['code'], 'SNAPSHOT_BINDING_MISMATCH')

    def test_missing_keycheck_for_observed_ciphertext_is_blocked(self):
        self.args.keycheck_report = None
        self.assertEqual(self.run_case()['error']['code'], 'KEYCHECK_EVIDENCE_REQUIRED')

    def test_empty_optional_path_cannot_bypass_required_keycheck(self):
        self.args.keycheck_report = ''
        self.assertEqual(self.run_case()['error']['code'], 'MANIFEST_INVALID')

    def test_ciphertext_per_field_count_conflict_is_blocked(self):
        self.data['keycheck']['observations']['encrypted_fields']['backup_accounts.config_json'] = 0
        self.assertEqual(self.run_case()['error']['code'], 'CIPHER_COUNT_CONFLICT')

    def test_partial_match_cannot_be_upgraded_to_verified(self):
        self.data['keycheck'].update(status='NOT_VERIFIED', key_match_verified=False, rejected_records=1, authenticated_records=0)
        self.assertEqual(self.run_case()['error']['code'], 'KEYCHECK_NOT_MATCHED')

    def test_inconsistent_keycheck_success_counts_are_blocked(self):
        self.data['keycheck']['authenticated_records'] = 0
        self.assertEqual(self.run_case()['error']['code'], 'KEYCHECK_COUNT_CONFLICT')

    def test_probe_is_required_not_just_an_offline_pass(self):
        self.data['drill']['scope'] = 'offline-sqlite-only'
        self.assertEqual(self.run_case()['error']['code'], 'PROBE_EVIDENCE_REQUIRED')

    def test_probe_auth_and_shutdown_contracts_are_not_truthy_strings(self):
        self.data['drill']['probe']['stopped_cleanly'] = 'true'
        self.assertEqual(self.run_case()['error']['code'], 'EVIDENCE_FLAG_INVALID')

    def test_no_production_auth_escalation_can_be_claimed_by_probe(self):
        self.data['drill']['probe']['production_auth_verified'] = True
        self.assertEqual(self.run_case()['error']['code'], 'EVIDENCE_FLAG_INVALID')

    def test_production_rto_or_rpo_claims_are_refused(self):
        self.data['drill']['production_rto_seconds'] = 1
        self.assertEqual(self.run_case()['error']['code'], 'PRODUCTION_CLAIM_REFUSED')

    def test_stage_missing_or_not_run_cannot_be_hidden_by_pass(self):
        del self.data['drill']['stages']['validate']
        self.assertEqual(self.run_case()['error']['code'], 'EVIDENCE_FIELDS_MISSING')

    def test_sample_bounds_and_boolean_counts_are_checked(self):
        self.data['drill']['sampled_rows']['users'] = True
        self.assertEqual(self.run_case()['error']['code'], 'EVIDENCE_COUNT_INVALID')

    def test_nan_elapsed_time_is_not_valid_evidence(self):
        self.data['drill']['elapsed_seconds'] = float('nan')
        self.assertEqual(self.run_case()['error']['code'], 'EVIDENCE_TIME_INVALID')

    def test_inventory_coverage_gap_is_not_complete_preflight(self):
        self.data['inventory']['observations']['coverage_gaps'] = ['future-gap']
        self.assertEqual(self.run_case()['error']['code'], 'INVENTORY_COVERAGE_INCOMPLETE')

    def test_asset_requirement_flags_are_recomputed(self):
        asset = next(a for a in self.data['inventory']['assets'] if a['role'] == 'encryption_master')
        asset['required_by_observation'] = False
        self.assertEqual(self.run_case()['error']['code'], 'ASSET_REQUIREMENT_CONFLICT')

    def test_required_asset_presence_is_still_needed(self):
        asset = next(a for a in self.data['inventory']['assets'] if a['role'] == 'jwt_signing')
        asset['status'] = 'NOT_DECLARED'
        self.assertEqual(self.run_case()['error']['code'], 'REQUIRED_ASSET_NOT_PRESENT')

    def test_duplicate_asset_role_is_refused(self):
        self.data['inventory']['assets'][-1] = copy.deepcopy(self.data['inventory']['assets'][0])
        self.assertEqual(self.run_case()['error']['code'], 'ASSET_EVIDENCE_INVALID')

    def test_jwt_plan_must_match_inventory_choice(self):
        self.args.jwt_strategy = 'reissue'
        self.assertEqual(self.run_case()['error']['code'], 'JWT_PLAN_MISMATCH')

    def test_cleanup_failure_is_not_silently_accepted(self):
        self.data['keycheck']['copies_removed'] = False
        self.assertEqual(self.run_case()['error']['code'], 'EVIDENCE_FLAG_INVALID')

    def test_unknown_schema_is_refused(self):
        self.data['drill']['schema'] = 'flamepanel/restore-drill/v99'
        self.assertEqual(self.run_case()['error']['code'], 'EVIDENCE_SCHEMA_UNSUPPORTED')

    def test_extra_input_secret_fields_are_not_echoed(self):
        self.data['inventory']['secret-source-path'] = 'PRIVATE_REPORT_NOTE_CANARY'
        self.assertEqual(self.run_case()['error']['code'], 'EVIDENCE_FIELDS_INVALID')

    def test_ignored_descriptions_are_not_copied_to_output(self):
        self.data['drill']['limitations'] = ['PRIVATE_REPORT_NOTE_CANARY']
        self.data['inventory']['manual_review'] = ['PRIVATE_REPORT_NOTE_CANARY']
        self.data['keycheck']['manual_review'] = ['PRIVATE_REPORT_NOTE_CANARY']
        self.run_case('REVIEW_REQUIRED')

    def test_duplicate_json_field_is_rejected_without_overwrite(self):
        self.write_inputs()
        (self.inputs / 'drill.json').write_text('{"schema":"a","schema":"b"}')
        self.assertEqual(self.run_case(write=False)['error']['code'], 'DUPLICATE_REPORT_FIELD')

    def test_unsafe_report_permissions_are_refused(self):
        self.write_inputs()
        (self.inputs / 'drill.json').chmod(0o644)
        self.assertEqual(self.run_case(write=False)['error']['code'], 'REPORT_PERMISSIONS')

    def test_root_permissions_are_refused(self):
        self.inputs.chmod(0o755)
        self.assertEqual(self.run_case()['error']['code'], 'EVIDENCE_ROOT_PERMISSIONS')

    def test_hardlinked_report_is_refused(self):
        self.write_inputs()
        os.link(self.inputs / 'drill.json', self.root / 'alias')
        self.assertEqual(self.run_case(write=False)['error']['code'], 'PRIVATE_REPORT_FILE_REQUIRED')

    def test_final_symlink_is_refused(self):
        self.write_inputs()
        (self.inputs / 'drill.json').unlink()
        (self.inputs / 'drill.json').symlink_to(self.inputs / 'inventory.json')
        self.run_case(write=False)

    def test_parent_symlink_is_refused(self):
        self.write_inputs()
        (self.inputs / 'alias').symlink_to(self.inputs, target_is_directory=True)
        self.args.drill_report = 'alias/drill.json'
        self.run_case(write=False)

    def test_fifo_is_refused_without_reading_or_blocking(self):
        self.write_inputs()
        file = self.inputs / 'drill.json'
        file.unlink()
        os.mkfifo(file, 0o600)
        # Do not let the test's own input-integrity snapshot read a FIFO.
        with self.assertRaises(preflight.drill.DrillError):
            fd = os.open(self.inputs, os.O_RDONLY | os.O_DIRECTORY)
            try:
                preflight.load_report(fd, 'drill.json', 10**20)
            finally:
                os.close(fd)

    def test_oversized_report_is_refused_before_json_parse(self):
        self.write_inputs()
        (self.inputs / 'drill.json').write_bytes(b'x' * (preflight.REPORT_LIMIT + 1))
        self.assertEqual(self.run_case(write=False)['error']['code'], 'REPORT_SIZE_LIMIT')

    def test_traversal_and_duplicate_paths_are_rejected(self):
        self.args.keycheck_report = '../keycheck.json'
        self.assertEqual(self.run_case()['error']['code'], 'MANIFEST_INVALID')

    def test_duplicate_role_paths_are_rejected(self):
        self.args.keycheck_report = self.args.inventory_report
        self.assertEqual(self.run_case()['error']['code'], 'DUPLICATE_REPORT_PATH')

    def test_existing_output_is_not_overwritten(self):
        self.write_inputs()
        Path(self.args.out).mkdir()
        with self.assertRaises(FileExistsError):
            preflight.run(self.args)

    def test_preflight_does_not_open_database_or_spawn_processes(self):
        with patch.object(sqlite3, 'connect', side_effect=AssertionError('database opened')), \
                patch.object(subprocess, 'Popen', side_effect=AssertionError('process spawned')), \
                patch.object(preflight.drill, 'acquire', side_effect=AssertionError('snapshot acquired')):
            self.run_case('REVIEW_REQUIRED')

    def test_cli_summary_and_usage_errors_do_not_echo_input_values(self):
        self.write_inputs()
        command = [sys.executable, str(Path(preflight.__file__)), '--evidence-root', str(self.inputs),
            '--drill-report', 'drill.json', '--inventory-report', 'inventory.json',
            '--keycheck-report', 'keycheck.json', '--jwt-strategy', 'preserve', '--out', self.args.out]
        result = subprocess.run(command, capture_output=True, text=True, timeout=10)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(result.stdout.strip(), 'RECOVERY_PREFLIGHT: REVIEW_REQUIRED')
        invalid = subprocess.run(command + ['--secret', 'PRIVATE_REPORT_NOTE_CANARY'], capture_output=True, text=True, timeout=10)
        self.assertEqual(invalid.returncode, 2)
        self.assertNotIn('PRIVATE_REPORT_NOTE_CANARY', invalid.stdout + invalid.stderr)


if __name__ == '__main__':
    unittest.main()
