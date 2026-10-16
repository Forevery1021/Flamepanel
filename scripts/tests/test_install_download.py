#!/usr/bin/env python3
"""Offline installer regression tests. Never execute the complete root installer.

Run: python3 scripts/tests/test_install_download.py
Only gh_download and the reviewed deployment section are extracted. curl and
chown are mocked; all artifact, install and temporary paths are inside a
TemporaryDirectory. No network, package installation or service operations.
"""
import argparse
import hashlib
import io
import json
import os
from pathlib import Path
import shlex
import subprocess
import sys
import tarfile
import tempfile
import unittest

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--installer', type=Path, default=Path(__file__).resolve().parents[2] / 'install.sh')
parser.add_argument('--download-only', action='store_true')
options = parser.parse_args()
SOURCE = options.installer.read_text()
FUNCTION = SOURCE.split('gh_download() {', 1)[1].split('\n}\n', 1)[0]
FUNCTION = 'gh_download() {' + FUNCTION + '\n}\n'

MOCK_CURL = r'''
import json, os, pathlib, shutil, sys
root = pathlib.Path(os.environ['FIXTURE_ROOT'])
args = sys.argv[1:]
url = next(a for a in args if a.startswith('http'))
log = root / 'calls.jsonl'
previous = [json.loads(l) for l in log.read_text().splitlines()] if log.exists() else []
with log.open('a') as f:
    f.write(json.dumps(args) + '\n')
def side(call):
    u = next(a for a in call if a.startswith('http'))
    return '%{url_effective}' in call or u.endswith('checksums.txt')
# 版本解析：按顺序消费 redirects.json（url 为 null 表示该来源失败）
if '%{url_effective}' in args:
    redirects = json.loads((root / 'redirects.json').read_text())
    n = sum(1 for c in previous if '%{url_effective}' in c)
    target = redirects[min(n, len(redirects) - 1)]
    if target is None:
        sys.exit(22)
    sys.stdout.write(target)
    sys.exit(0)
out = pathlib.Path(args[args.index('-o') + 1])
out.resolve().relative_to(root.resolve())
# 校验清单：不消耗 plan；checksums.txt 缺失即 404，direct_fails 时直连 404（走镜像）
if url.endswith('checksums.txt'):
    sums = root / 'checksums.txt'
    if not sums.exists() or ((root / 'checksums_direct_fails').exists() and url.startswith(os.environ.get('FIXTURE_GH_BASE', '#'))):
        out.write_text('<html>404 Not Found</html>')
        sys.exit(22)
    shutil.copyfile(sums, out)
    sys.exit(0)
plan = json.loads((root / 'plan.json').read_text())
calls = sum(1 for c in previous if not side(c))
entry = plan[min(calls, len(plan) - 1)]
# The mock must not write outside this test's temporary directory.
out.resolve().relative_to(root.resolve())
with (root / 'permissions.jsonl').open('a') as f:
    f.write(json.dumps({'parent': out.parent.stat().st_mode & 0o777,
                        'file': (out.stat().st_mode & 0o777) if out.exists() else None}) + '\n')
kind = entry['kind']
if kind == 'http_error':
    out.write_text('<html>404 Not Found</html>')
    sys.exit(22 if '--fail' in args or '-f' in args else 0)
if kind == 'transfer_error':
    out.write_text('partial response')
    sys.exit(18)
if kind == 'empty':
    out.write_bytes(b'')
else:
    shutil.copyfile(entry['path'], out)
'''


class Fixture(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory(prefix='flamepanel install test ')
        self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name)
        self.project = self.root / 'project'
        self.project.mkdir()
        self.bin = self.root / 'bin'
        self.bin.mkdir()
        self.tmpdir = self.root / 'tmp'
        self.tmpdir.mkdir()
        self.installed = self.root / 'installed'
        self.installed.mkdir()
        self.binary = self.installed / 'flamepanel'
        self.ctl = self.installed / 'fpctl'
        self.frontend = self.installed / 'frontend' / 'dist'
        self.frontend.mkdir(parents=True)
        (self.frontend / 'index.html').write_text('old frontend')
        self.mock('curl', '#!' + sys.executable + '\n' + MOCK_CURL)
        self.mock('chown', '#!/bin/sh\nexit 0\n')
        self.mock('uname', '#!/bin/sh\nprintf "x86_64\\n"\n')
        # Do not propagate the cloud environment's credentials into test programs.
        self.env = {'PATH': str(self.bin) + ':/usr/bin:/bin',
                    'HOME': str(self.root), 'TMPDIR': str(self.tmpdir),
                    'LANG': 'C.UTF-8', 'FIXTURE_ROOT': str(self.root),
                    'FIXTURE_GH_BASE': 'https://github.com/', 'FLAMEPANEL_VERSION': '9.9.9'}
        # None = 按 plan 自动生成正确的校验清单；测试可改为字符串（自定义内容）或 False（不提供清单）
        self.checksums = None
        self.payload = self.root / 'payload'
        self.payload.write_text('release contents')

    def mock(self, name, body):
        p = self.bin / name
        p.write_text(body)
        p.chmod(0o755)

    def plan(self, *entries):
        (self.root / 'plan.json').write_text(json.dumps(entries))

    def success(self, path=None):
        return {'kind': 'success', 'path': str(path or self.payload)}

    def calls(self):
        p = self.root / 'calls.jsonl'
        return [json.loads(s) for s in p.read_text().splitlines()] if p.exists() else []

    def write_checksums(self):
        sums = self.root / 'checksums.txt'
        if self.checksums is False:
            sums.unlink(missing_ok=True)
            return
        if isinstance(self.checksums, str):
            sums.write_text(self.checksums)
            return
        plan_path = self.root / 'plan.json'
        plan = json.loads(plan_path.read_text()) if plan_path.exists() else []
        local_binary = any((self.project / p).is_file() for p in
                           ('target/release/flame-kernel', 'flame-kernel/target/release/flame-kernel'))
        local_frontend = any((self.project / p / 'index.html').is_file() for p in ('frontend/dist', 'dist'))
        assets = ([] if local_binary else ['flamepanel-linux-amd64.tar.gz']) + \
                 ([] if local_frontend else ['flamepanel-frontend.tar.gz'])
        lines = []
        for i, name in enumerate(assets):
            if not plan:
                break
            entry = plan[min(i, len(plan) - 1)]
            if entry['kind'] == 'success':
                digest = hashlib.sha256(Path(entry['path']).read_bytes()).hexdigest()
                lines.append(f'{digest}  {name}\n')
        sums.write_text(''.join(lines))

    def run_script(self, text):
        self.write_checksums()
        script = self.project / 'fixture.sh'
        script.write_text('set -euo pipefail\n' + text)
        return subprocess.run(['bash', str(script)], cwd=self.project, env=self.env,
                              text=True, capture_output=True, timeout=15)

    def download(self):
        self.out = self.root / 'download.tar.gz'
        return self.run_script('GH_BASE=https://release.invalid/repo\n'
            'GH_MIRRORS=(https://mirror1.invalid/repo https://mirror2.invalid/repo)\n'
            + FUNCTION + '\nif gh_download "$GH_BASE/asset" ' + shlex.quote(str(self.out))
            + '; then exit 0; else exit 9; fi\n')

    def assert_no_partial_downloads(self):
        self.assertEqual(list(self.root.glob('*.part.*')), [])

    def archive(self, name, files):
        path = self.root / name
        with tarfile.open(path, 'w:gz') as archive:
            for filename, content in files.items():
                content = content.encode()
                info = tarfile.TarInfo(filename)
                info.size = len(content)
                archive.addfile(info, io.BytesIO(content))
        return path

    def local_binary(self, contents='local binary'):
        path = self.project / 'target' / 'release' / 'flame-kernel'
        path.parent.mkdir(parents=True)
        path.write_text(contents)

    def deployment(self):
        section = SOURCE.split('# 查找本地编译好的二进制', 1)[1]
        section = '# 查找本地编译好的二进制' + section.split('# ─── 配置 nginx 反向代理', 1)[0]
        # Refuse unsafe baseline versions: never execute hard-coded system writes.
        self.assertNotIn('/usr/local/bin/', section)
        self.assertNotIn('/tmp/flamepanel', section)
        prefix = 'RED= GREEN= YELLOW= CYAN= NC=\nRUN_USER=test\nRUN_GROUP=test\n'
        prefix += 'INSTALL_DIR=' + shlex.quote(str(self.installed)) + '\n'
        prefix += 'BINARY_PATH=' + shlex.quote(str(self.binary)) + '\n'
        prefix += 'CTL_PATH=' + shlex.quote(str(self.ctl)) + '\n'
        result = self.run_script(prefix + section + '\necho REACHED_SERVICE_CONFIGURATION\n')
        self.assertEqual(list(self.tmpdir.iterdir()), [], result.stderr)
        return result


class DownloadTests(Fixture):
    def test_success_does_not_try_mirrors(self):
        self.plan(self.success())
        r = self.download()
        self.assertEqual(r.returncode, 0, r.stderr)
        self.assertEqual(self.out.read_bytes(), self.payload.read_bytes())
        self.assertEqual(len(self.calls()), 1)
        self.assert_no_partial_downloads()

    def test_http_error_falls_back_to_mirror(self):
        self.plan({'kind': 'http_error'}, self.success())
        r = self.download()
        self.assertEqual(r.returncode, 0, r.stderr)
        self.assertEqual(self.out.read_bytes(), self.payload.read_bytes())
        self.assertEqual(len(self.calls()), 2)
        self.assertIn('https://mirror1.invalid/repo/asset', self.calls()[1])
        self.assert_no_partial_downloads()

    def test_all_http_errors_fail_without_output(self):
        self.plan({'kind': 'http_error'})
        r = self.download()
        self.assertNotEqual(r.returncode, 0)
        self.assertFalse(self.out.exists())
        self.assertEqual(len(self.calls()), 3)
        self.assert_no_partial_downloads()

    def test_partial_transfer_preserves_existing_output(self):
        (self.root / 'download.tar.gz').write_text('previous artifact')
        self.plan({'kind': 'transfer_error'})
        r = self.download()
        self.assertNotEqual(r.returncode, 0)
        self.assertEqual(self.out.read_text(), 'previous artifact')
        self.assert_no_partial_downloads()

    def test_empty_responses_are_not_artifacts(self):
        self.plan({'kind': 'empty'})
        r = self.download()
        self.assertNotEqual(r.returncode, 0)
        self.assertFalse(self.out.exists())
        self.assert_no_partial_downloads()

    def test_publish_failure_is_not_success(self):
        (self.root / 'download.tar.gz').write_text('previous artifact')
        self.mock('mv', '#!/bin/sh\nexit 1\n')
        self.plan(self.success())
        r = self.download()
        self.assertNotEqual(r.returncode, 0)
        self.assertEqual(self.out.read_text(), 'previous artifact')
        self.assert_no_partial_downloads()

    def test_empty_primary_response_tries_next_source(self):
        self.plan({'kind': 'empty'}, self.success())
        r = self.download()
        self.assertEqual(r.returncode, 0, r.stderr)
        self.assertEqual(self.out.read_bytes(), self.payload.read_bytes())
        self.assertEqual(len(self.calls()), 2)
        self.assert_no_partial_downloads()

    def test_second_mirror_after_partial_and_http_errors(self):
        self.plan({'kind': 'transfer_error'}, {'kind': 'http_error'}, self.success())
        r = self.download()
        self.assertEqual(r.returncode, 0, r.stderr)
        self.assertEqual(self.out.read_bytes(), self.payload.read_bytes())
        self.assertEqual(len(self.calls()), 3)
        self.assertIn('https://mirror2.invalid/repo/asset', self.calls()[2])
        self.assert_no_partial_downloads()

    def test_temporary_file_failure_does_not_start_download(self):
        self.mock('mktemp', '#!/bin/sh\nexit 1\n')
        self.plan(self.success())
        r = self.download()
        self.assertNotEqual(r.returncode, 0)
        self.assertEqual(self.calls(), [])
        self.assertFalse(self.out.exists())

    def test_download_temporary_file_is_private(self):
        self.plan(self.success())
        r = self.download()
        self.assertEqual(r.returncode, 0, r.stderr)
        permissions = json.loads((self.root / 'permissions.jsonl').read_text().splitlines()[0])
        self.assertEqual(permissions['file'], 0o600)
        self.assert_no_partial_downloads()


class DeploymentTests(Fixture):
    def assert_stopped(self, result):
        self.assertNotEqual(result.returncode, 0, result.stdout)
        self.assertNotIn('REACHED_SERVICE_CONFIGURATION', result.stdout)

    def test_missing_backend_stops_before_frontend(self):
        self.plan({'kind': 'http_error'})
        r = self.deployment()
        self.assert_stopped(r)
        self.assertNotIn('二进制部署完成', r.stdout)
        self.assertFalse(self.binary.exists())

    def test_invalid_backend_archive_keeps_existing_binary(self):
        self.binary.write_text('old binary')
        self.plan(self.success())  # Not a tar archive.
        r = self.deployment()
        self.assert_stopped(r)
        self.assertEqual(self.binary.read_text(), 'old binary')

    def test_backend_archive_missing_binary_is_rejected(self):
        self.plan(self.success(self.archive('backend.tgz', {'README': 'wrong layout'})))
        r = self.deployment()
        self.assert_stopped(r)
        self.assertFalse(self.binary.exists())

    def test_empty_local_binary_is_rejected(self):
        self.local_binary('')
        self.plan(self.success())
        r = self.deployment()
        self.assert_stopped(r)
        self.assertEqual(self.calls(), [])

    def test_frontend_download_failure_keeps_existing_frontend(self):
        self.local_binary()
        self.plan({'kind': 'http_error'})
        r = self.deployment()
        self.assert_stopped(r)
        self.assertEqual((self.frontend / 'index.html').read_text(), 'old frontend')

    def test_invalid_frontend_archive_keeps_existing_frontend(self):
        self.local_binary()
        self.plan(self.success())
        r = self.deployment()
        self.assert_stopped(r)
        self.assertEqual((self.frontend / 'index.html').read_text(), 'old frontend')

    def test_frontend_missing_index_is_rejected(self):
        self.local_binary()
        self.plan(self.success(self.archive('frontend.tgz', {'other.html': 'wrong layout'})))
        r = self.deployment()
        self.assert_stopped(r)
        self.assertEqual((self.frontend / 'index.html').read_text(), 'old frontend')

    def test_empty_frontend_index_is_rejected(self):
        self.local_binary()
        self.plan(self.success(self.archive('frontend.tgz', {'index.html': ''})))
        self.assert_stopped(self.deployment())
        self.assertEqual((self.frontend / 'index.html').read_text(), 'old frontend')

    def test_valid_downloaded_artifacts(self):
        backend = self.archive('backend.tgz', {'flamepanel': 'new binary'})
        frontend = self.archive('frontend.tgz', {'index.html': 'new frontend', 'assets/app.js': '// app'})
        self.plan(self.success(backend), self.success(frontend))
        r = self.deployment()
        self.assertEqual(r.returncode, 0, r.stderr)
        self.assertIn('REACHED_SERVICE_CONFIGURATION', r.stdout)
        self.assertEqual(self.binary.read_text(), 'new binary')
        self.assertTrue(os.access(self.binary, os.X_OK))
        self.assertEqual((self.frontend / 'index.html').read_text(), 'new frontend')
        self.assertTrue((self.frontend / 'assets' / 'app.js').is_file())

    def test_local_artifacts_do_not_download(self):
        self.local_binary()
        dist = self.project / 'frontend' / 'dist'
        dist.mkdir(parents=True)
        (dist / 'index.html').write_text('local frontend')
        self.plan({'kind': 'http_error'})
        r = self.deployment()
        self.assertEqual(r.returncode, 0, r.stderr)
        self.assertEqual(self.calls(), [])
        self.assertEqual((self.frontend / 'index.html').read_text(), 'local frontend')


    def test_reinstall_over_versioned_layout_writes_real_files(self):
        # Doc/32 §5：稳定路径是指向 current 的链接时，重装必须摘链接后写入真实文件，
        # 绝不能透过链接改写 releases/<版本> 里的旧版本（那是回滚目标）
        release = self.installed / 'releases' / '1.0.0'
        (release / 'bin').mkdir(parents=True)
        (release / 'bin' / 'flamepanel').write_text('release binary')
        (release / 'bin' / 'fpctl').write_text('release cli')
        (release / 'frontend' / 'dist').mkdir(parents=True)
        (release / 'frontend' / 'dist' / 'index.html').write_text('release frontend')
        (self.installed / 'current').symlink_to('releases/1.0.0')
        (self.frontend / 'index.html').unlink()
        self.frontend.rmdir()
        (self.installed / 'frontend').rmdir()
        (self.installed / 'frontend').symlink_to('current/frontend')
        self.binary.symlink_to(self.installed / 'current' / 'bin' / 'flamepanel')
        self.ctl.symlink_to(self.installed / 'current' / 'bin' / 'fpctl')
        backend = self.archive('backend.tgz', {'flamepanel': 'new binary', 'fpctl': 'new cli'})
        frontend = self.archive('frontend.tgz', {'index.html': 'new frontend'})
        self.plan(self.success(backend), self.success(frontend))
        r = self.deployment()
        self.assertEqual(r.returncode, 0, r.stderr)
        for p, content in ((self.binary, 'new binary'), (self.ctl, 'new cli'),
                           (self.frontend / 'index.html', 'new frontend')):
            self.assertFalse(p.is_symlink(), p)
            self.assertEqual(p.read_text(), content)
        self.assertFalse((self.installed / 'frontend').is_symlink())
        self.assertEqual((release / 'bin' / 'flamepanel').read_text(), 'release binary')
        self.assertEqual((release / 'bin' / 'fpctl').read_text(), 'release cli')
        self.assertEqual((release / 'frontend' / 'dist' / 'index.html').read_text(), 'release frontend')

    def test_fpctl_is_deployed_from_release_archive(self):
        backend = self.archive('backend.tgz', {'flamepanel': 'new binary', 'fpctl': 'cli binary'})
        frontend = self.archive('frontend.tgz', {'index.html': 'new frontend'})
        self.plan(self.success(backend), self.success(frontend))
        r = self.deployment()
        self.assertEqual(r.returncode, 0, r.stderr)
        self.assertEqual(self.ctl.read_text(), 'cli binary')
        self.assertEqual(self.ctl.stat().st_mode & 0o777, 0o755)
        self.assertIn('命令行工具部署完成', r.stdout)

    def test_archive_without_fpctl_still_installs(self):
        # 旧版资源包不含 fpctl：只告警，不阻断安装
        backend = self.archive('backend.tgz', {'flamepanel': 'new binary'})
        frontend = self.archive('frontend.tgz', {'index.html': 'new frontend'})
        self.plan(self.success(backend), self.success(frontend))
        r = self.deployment()
        self.assertEqual(r.returncode, 0, r.stderr)
        self.assertIn('REACHED_SERVICE_CONFIGURATION', r.stdout)
        self.assertFalse(self.ctl.exists())
        self.assertIn('未找到 fpctl', r.stdout)

    def test_local_fpctl_next_to_local_binary_is_deployed(self):
        self.local_binary()
        (self.project / 'target' / 'release' / 'fpctl').write_text('local cli')
        dist = self.project / 'frontend' / 'dist'
        dist.mkdir(parents=True)
        (dist / 'index.html').write_text('local frontend')
        self.plan({'kind': 'http_error'})
        r = self.deployment()
        self.assertEqual(r.returncode, 0, r.stderr)
        self.assertEqual(self.ctl.read_text(), 'local cli')

    def symlink_archive(self, name, link_name, target_name, content):
        path = self.archive(name, {target_name: content})
        # Rewrite with a target and symlink, without extracting it in the test helper.
        with tarfile.open(path, 'w:gz') as archive:
            target = tarfile.TarInfo(target_name)
            payload = content.encode()
            target.size = len(payload)
            archive.addfile(target, io.BytesIO(payload))
            link = tarfile.TarInfo(link_name)
            link.type = tarfile.SYMTYPE
            link.linkname = target_name
            archive.addfile(link)
        return path

    def test_empty_downloaded_binary_keeps_existing_binary(self):
        self.binary.write_text('old binary')
        self.plan(self.success(self.archive('backend.tgz', {'flamepanel': ''})))
        self.assert_stopped(self.deployment())
        self.assertEqual(self.binary.read_text(), 'old binary')

    def test_symlink_binary_is_rejected(self):
        self.binary.write_text('old binary')
        archive = self.symlink_archive('backend.tgz', 'flamepanel', 'other', 'new binary')
        self.plan(self.success(archive))
        self.assert_stopped(self.deployment())
        self.assertEqual(self.binary.read_text(), 'old binary')

    def test_symlink_frontend_index_is_rejected(self):
        self.local_binary()
        archive = self.symlink_archive('frontend.tgz', 'index.html', 'other.html', 'new frontend')
        self.plan(self.success(archive))
        self.assert_stopped(self.deployment())
        self.assertEqual((self.frontend / 'index.html').read_text(), 'old frontend')

    def test_empty_local_frontend_keeps_existing_frontend(self):
        self.local_binary()
        dist = self.project / 'frontend' / 'dist'
        dist.mkdir(parents=True)
        (dist / 'index.html').write_text('')
        self.plan(self.success())
        self.assert_stopped(self.deployment())
        self.assertEqual(self.calls(), [])
        self.assertEqual((self.frontend / 'index.html').read_text(), 'old frontend')

    def test_binary_copy_failure_stops_without_success_message(self):
        self.local_binary()
        self.binary.write_text('old binary')
        self.mock('cp', '#!/bin/sh\nexit 1\n')
        self.plan(self.success())
        r = self.deployment()
        self.assert_stopped(r)
        self.assertNotIn('二进制部署完成', r.stdout)
        self.assertEqual(self.binary.read_text(), 'old binary')

    def test_binary_chmod_failure_stops_without_success_message(self):
        self.local_binary()
        self.mock('chmod', '#!/bin/sh\nexit 1\n')
        self.plan(self.success())
        r = self.deployment()
        self.assert_stopped(r)
        self.assertNotIn('二进制部署完成', r.stdout)
        self.assertEqual(self.calls(), [])

    def test_download_directory_is_private(self):
        backend = self.archive('backend.tgz', {'flamepanel': 'new binary'})
        frontend = self.archive('frontend.tgz', {'index.html': 'new frontend'})
        self.plan(self.success(backend), self.success(frontend))
        r = self.deployment()
        self.assertEqual(r.returncode, 0, r.stderr)
        for line in (self.root / 'permissions.jsonl').read_text().splitlines():
            permissions = json.loads(line)
            self.assertEqual(permissions['parent'], 0o700)
            self.assertEqual(permissions['file'], 0o600)

    def test_staging_directory_failure_stops_before_replacement(self):
        self.local_binary()
        self.binary.write_text('old binary')
        self.mock('mktemp', '#!/bin/sh\nexit 1\n')
        self.plan(self.success())
        self.assert_stopped(self.deployment())
        self.assertEqual(self.binary.read_text(), 'old binary')
        self.assertEqual(self.calls(), [])


class ReleaseVerifyTests(Fixture):
    """发布包 SHA-256 校验与版本固定（Doc/33 排除项）。"""

    def valid_plan(self):
        backend = self.archive('backend.tgz', {'flamepanel': 'new binary'})
        frontend = self.archive('frontend.tgz', {'index.html': 'new frontend'})
        self.plan(self.success(backend), self.success(frontend))

    def assert_stopped(self, result):
        self.assertNotEqual(result.returncode, 0, result.stdout)
        self.assertNotIn('REACHED_SERVICE_CONFIGURATION', result.stdout)

    def asset_urls(self):
        return [next(a for a in c if a.startswith('http')) for c in self.calls()]

    def test_downloads_pinned_tag_and_verifies_both_assets(self):
        self.valid_plan()
        r = self.deployment()
        self.assertEqual(r.returncode, 0, r.stderr)
        self.assertEqual(r.stdout.count('SHA-256 校验通过'), 2, r.stdout)
        urls = self.asset_urls()
        self.assertTrue(all('/releases/download/v9.9.9/' in u for u in urls), urls)
        self.assertTrue(any(u.endswith('flamepanel-9.9.9-checksums.txt') for u in urls), urls)
        self.assertNotIn('警告', r.stdout)

    def test_digest_mismatch_stops_before_extracting(self):
        self.valid_plan()
        self.checksums = ('0' * 64 + '  flamepanel-linux-amd64.tar.gz\n')
        self.binary.write_text('old binary')
        r = self.deployment()
        self.assert_stopped(r)
        self.assertIn('SHA-256 不符', r.stderr)
        self.assertEqual(self.binary.read_text(), 'old binary')

    def test_frontend_mismatch_keeps_existing_frontend(self):
        backend = self.archive('backend.tgz', {'flamepanel': 'new binary'})
        frontend = self.archive('frontend.tgz', {'index.html': 'tampered'})
        self.plan(self.success(backend), self.success(frontend))
        digest = hashlib.sha256(backend.read_bytes()).hexdigest()
        self.checksums = f'{digest}  flamepanel-linux-amd64.tar.gz\n{"1" * 64}  flamepanel-frontend.tar.gz\n'
        r = self.deployment()
        self.assert_stopped(r)
        self.assertIn('flamepanel-frontend.tar.gz SHA-256 不符', r.stderr)
        self.assertEqual((self.frontend / 'index.html').read_text(), 'old frontend')

    def test_missing_checksums_file_fails_closed(self):
        self.valid_plan()
        self.checksums = False
        r = self.deployment()
        self.assert_stopped(r)
        self.assertIn('校验清单', r.stderr)
        self.assertFalse(self.binary.exists())

    def test_asset_not_listed_fails_closed(self):
        self.valid_plan()
        self.checksums = 'a' * 64 + '  something-else.tar.gz\n'
        r = self.deployment()
        self.assert_stopped(r)
        self.assertIn('校验清单中没有 flamepanel-linux-amd64.tar.gz', r.stderr)

    def test_checksums_from_mirror_are_used_with_warning(self):
        self.valid_plan()
        (self.root / 'checksums_direct_fails').write_text('')
        r = self.deployment()
        self.assertEqual(r.returncode, 0, r.stderr)
        self.assertIn('校验清单来自镜像站', r.stdout)
        self.assertEqual(r.stdout.count('SHA-256 校验通过'), 2)

    def test_explicit_skip_installs_without_checksums(self):
        self.valid_plan()
        self.checksums = False
        self.env['FLAMEPANEL_SKIP_VERIFY'] = '1'
        r = self.deployment()
        self.assertEqual(r.returncode, 0, r.stderr)
        self.assertIn('已跳过', r.stdout)
        self.assertFalse(any(u.endswith('checksums.txt') for u in self.asset_urls()))

    def test_latest_is_resolved_to_a_concrete_tag(self):
        self.valid_plan()
        del self.env['FLAMEPANEL_VERSION']
        (self.root / 'redirects.json').write_text(json.dumps(
            [None, 'https://ghfast.top/https://github.com/Forevery1021/Flamepanel/releases/tag/v1.4.2']))
        r = self.deployment()
        self.assertEqual(r.returncode, 0, r.stderr)
        self.assertIn('发布版本: v1.4.2', r.stdout)
        urls = [u for u in self.asset_urls() if not u.endswith('/releases/latest')]
        self.assertTrue(urls and all('/releases/download/v1.4.2/' in u for u in urls), urls)

    def test_unresolvable_version_fails_before_download(self):
        self.valid_plan()
        del self.env['FLAMEPANEL_VERSION']
        (self.root / 'redirects.json').write_text(json.dumps([None]))
        r = self.deployment()
        self.assert_stopped(r)
        self.assertIn('无法确定要安装的发布版本', r.stderr)
        self.assertTrue(all(u.endswith('/releases/latest') for u in self.asset_urls()), self.asset_urls())

    def test_malformed_release_argument_is_rejected(self):
        self.valid_plan()
        self.env['FLAMEPANEL_VERSION'] = '1.0.0/../../evil'
        r = self.deployment()
        self.assert_stopped(r)
        self.assertEqual(self.calls(), [])


if __name__ == '__main__':
    suite = unittest.TestLoader().loadTestsFromTestCase(DownloadTests)
    if not options.download_only:
        suite.addTests(unittest.TestLoader().loadTestsFromTestCase(DeploymentTests))
        suite.addTests(unittest.TestLoader().loadTestsFromTestCase(ReleaseVerifyTests))
    result = unittest.TextTestRunner(verbosity=2).run(suite)
    sys.exit(not result.wasSuccessful())
