#!/usr/bin/env python3
"""Offline distro-matrix regression tests for install.sh / uninstall.sh (Doc/33 §4).

Run: python3 scripts/tests/test_install_platform.py
Only the platform helper functions and the nginx section are extracted and run
against temporary directories. Package managers, systemctl and nginx are stubbed;
a real `nginx -t` is used only when nginx is installed. Never touches /etc.
"""
from pathlib import Path
import os
import shutil
import subprocess
import tempfile
import textwrap
import unittest

ROOT = Path(__file__).resolve().parents[2]
INSTALL = (ROOT / 'install.sh').read_text()
UNINSTALL = (ROOT / 'uninstall.sh').read_text()


def between(src: str, start: str, end: str) -> str:
    a = src.index(start)
    return src[a:src.index(end, a)]


FUNCS = between(INSTALL, '# ─── 平台适配函数', '# ─── 系统检测 ──')
NGINX_SECTION = between(INSTALL, '# ─── 配置 nginx 反向代理 ───', '# ─── 配置 systemd ───')
CSP_FALLBACK = INSTALL.split('CSP_FALLBACK_POLICY="', 1)[1].split('"\n', 1)[0]
REAL_NGINX = next((p for p in ('/usr/sbin/nginx', shutil.which('nginx') or '') if p and os.path.exists(p)), None)

OS_RELEASES = {
    'debian12': ('ID=debian\nVERSION_ID="12"\n', 'debian'),
    'ubuntu2404': ('ID=ubuntu\nID_LIKE=debian\nVERSION_ID="24.04"\n', 'debian'),
    'mint': ('ID=linuxmint\nID_LIKE="ubuntu debian"\n', 'debian'),
    'rocky9': ('ID="rocky"\nID_LIKE="rhel centos fedora"\nVERSION_ID="9.4"\n', 'rhel'),
    'alma9': ('ID="almalinux"\nID_LIKE="rhel centos fedora"\n', 'rhel'),
    'rhel9': ('ID="rhel"\nID_LIKE="fedora"\n', 'rhel'),
    'centos-stream': ('ID="centos"\nID_LIKE="rhel fedora"\n', 'rhel'),
    'oracle9': ('ID="ol"\nID_LIKE="fedora"\n', 'rhel'),
    'fedora40': ('ID=fedora\nVERSION_ID=40\n', 'rhel'),
    'opensuse-leap': ('ID="opensuse-leap"\nID_LIKE="suse opensuse"\n', 'suse'),
    'arch': ('ID=arch\n', 'arch'),
    'alpine': ('ID=alpine\n', 'alpine'),
    'gentoo': ('ID=gentoo\n', 'unknown'),
}


class Sandbox(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory(prefix='fp-platform-')
        self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name)
        self.bin = self.root / 'bin'
        self.bin.mkdir()
        self.log = self.root / 'calls.log'

    def stub(self, name: str, body: str = 'exit 0'):
        path = self.bin / name
        path.write_text(f'#!/bin/bash\necho "{name} $*" >> "{self.log}"\n{body}\n')
        path.chmod(0o755)

    def bash(self, script: str, **env):
        full_env = {
            'PATH': f'{self.bin}:/usr/local/bin:/usr/bin:/bin:/usr/sbin:/sbin',
            'HOME': str(self.root), 'LANG': 'C.UTF-8',
            'RED': '', 'GREEN': '', 'YELLOW': '', 'CYAN': '', 'NC': '',
        }
        full_env.update({k: str(v) for k, v in env.items()})
        return subprocess.run(['bash', '-c', 'set -euo pipefail\n' + FUNCS + '\n' + script],
                              env=full_env, capture_output=True, text=True, timeout=30)


class DetectPlatform(Sandbox):
    def test_os_family_matrix(self):
        for name, (content, family) in OS_RELEASES.items():
            f = self.root / f'{name}.os-release'
            f.write_text(content)
            r = self.bash('detect_platform; echo "$OS_FAMILY|$OS_ID"', OS_RELEASE_FILE=f)
            self.assertEqual(r.returncode, 0, r.stderr)
            self.assertEqual(r.stdout.strip().split('|')[0], family, name)

    def test_missing_os_release_is_unknown(self):
        r = self.bash('detect_platform; echo "$OS_FAMILY|$OS_ID|$OS_VERSION"',
                      OS_RELEASE_FILE=self.root / 'absent')
        self.assertEqual(r.stdout.strip(), 'unknown|unknown|unknown')


class PackageDispatch(Sandbox):
    def test_family_uses_native_package_manager(self):
        for tool in ('apt-get', 'dnf', 'yum', 'zypper', 'pacman', 'apk'):
            self.stub(tool)
        expected = {
            'debian': 'apt-get install -y -qq nginx',
            'rhel': 'dnf install -y -q nginx',
            'suse': 'zypper --non-interactive --quiet install nginx',
            'arch': 'pacman -Sy --needed --noconfirm nginx',
            'alpine': 'apk add --no-cache nginx',
        }
        for family, call in expected.items():
            self.log.write_text('')
            r = self.bash(f'OS_FAMILY={family}; pkg_install nginx')
            self.assertEqual(r.returncode, 0, r.stderr)
            self.assertIn(call, self.log.read_text(), family)

    def test_failure_is_reported_not_swallowed(self):
        self.stub('apt-get', 'echo "E: Unable to locate package nginx" >&2; exit 100')
        r = self.bash('OS_FAMILY=debian; pkg_install nginx || echo FAILED')
        self.assertIn('FAILED', r.stdout)
        self.assertIn('Unable to locate package', r.stderr)


class NginxLayout(Sandbox):
    def make_nginx(self, main: str, *, sites_default=False) -> Path:
        nroot = self.root / 'nginx'
        nroot.mkdir()
        (nroot / 'nginx.conf').write_text(main.replace('@ROOT@', str(nroot)))
        if sites_default:
            (nroot / 'sites-available').mkdir()
            (nroot / 'sites-enabled').mkdir()
            (nroot / 'sites-available' / 'default').write_text('server { listen 80 default_server; }\n')
            os.symlink(nroot / 'sites-available' / 'default', nroot / 'sites-enabled' / 'default')
        return nroot

    def test_debian_uses_conf_d_and_disables_default_site(self):
        nroot = self.make_nginx('http {\n    include @ROOT@/conf.d/*.conf;\n    include @ROOT@/sites-enabled/*;\n}\n',
                                sites_default=True)
        r = self.bash('nginx_conf_dir; disable_distro_default_site; '
                      'echo "$NGINX_DIR|$NGINX_DEFAULT_SITE_DISABLED"', NGINX_ROOT=nroot)
        self.assertEqual(r.returncode, 0, r.stderr)
        self.assertEqual(r.stdout.strip(), f'{nroot}/conf.d|1')
        self.assertFalse((nroot / 'sites-enabled' / 'default').exists())

    def test_custom_default_site_is_left_alone(self):
        nroot = self.make_nginx('http {\n    include conf.d/*.conf;\n}\n')
        (nroot / 'sites-enabled').mkdir()
        (nroot / 'sites-enabled' / 'default').write_text('# user managed\n')
        r = self.bash('disable_distro_default_site; echo "$NGINX_DEFAULT_SITE_DISABLED"', NGINX_ROOT=nroot)
        self.assertEqual(r.stdout.strip(), '0')
        self.assertTrue((nroot / 'sites-enabled' / 'default').exists())

    def test_relative_include_and_alpine_http_d(self):
        nroot = self.make_nginx('http {\n    include http.d/*.conf;\n}\n')
        r = self.bash('nginx_conf_dir; echo "$NGINX_DIR"', NGINX_ROOT=nroot)
        self.assertEqual(r.stdout.strip(), f'{nroot}/http.d')

    def test_arch_include_is_added_once_with_backup(self):
        nroot = self.make_nginx('worker_processes 1;\nevents {}\nhttp {\n    server { listen 80; }\n}\n')
        r = self.bash('nginx_conf_dir; echo "$NGINX_DIR|$NGINX_INCLUDE_ADDED"; '
                      'nginx_conf_dir; echo "$NGINX_DIR|$NGINX_INCLUDE_ADDED"', NGINX_ROOT=nroot)
        self.assertEqual(r.returncode, 0, r.stderr)
        # 第一次补 include；第二次（重装）识别出带标记的 include，不重复插入但仍记为已添加
        self.assertEqual(r.stdout.splitlines(), [f'{nroot}/conf.d|1', f'{nroot}/conf.d|1'])
        main = (nroot / 'nginx.conf').read_text()
        self.assertEqual(main.count('conf.d/*.conf'), 1)
        self.assertLess(main.index('include'), main.index('server {'))
        self.assertTrue((nroot / 'nginx.conf.flamepanel.bak').exists())

    def test_worker_user_detection(self):
        nroot = self.make_nginx('user nginx;\nhttp {}\n')
        r = self.bash('nginx_worker_user', NGINX_ROOT=nroot)
        self.assertEqual(r.stdout.strip(), 'nginx')


class NginxRendering(Sandbox):
    """Render the real nginx section into a temp root and validate it."""

    def render(self, tls: bool) -> tuple[Path, str]:
        nroot = self.root / 'nginx'
        (nroot / 'conf.d').mkdir(parents=True)
        (nroot / 'nginx.conf').write_text(f'http {{\n    include {nroot}/conf.d/*.conf;\n}}\n')
        inst = self.root / 'opt'
        (inst / 'frontend' / 'dist').mkdir(parents=True, exist_ok=True)
        self.stub('nginx')          # nginx -t 由下方真实二进制单独校验
        self.stub('systemctl')
        self.stub('chown')
        env = dict(NGINX_ROOT=nroot, INSTALL_DIR=inst, FRONTEND_DIR=inst / 'frontend',
                   PANEL_PORT=18080, CSP_FALLBACK_POLICY=CSP_FALLBACK, ENABLE_TLS='true' if tls else 'false',
                   TLS_CERT_PATH='', TLS_KEY_PATH='', TLS_DOMAIN='', TLS_COUNTRY='CN', TLS_ORG='T')
        r = self.bash(NGINX_SECTION + '\necho "HOST=$PANEL_HOST"', **env)
        self.assertEqual(r.returncode, 0, r.stderr)
        self.assertIn('HOST=127.0.0.1', r.stdout)
        state = (inst / '.install-state').read_text()
        self.assertIn(f'NGINX_CONF_PATH={nroot}/conf.d/flamepanel.conf', state)
        return nroot, (nroot / 'conf.d' / 'flamepanel.conf').read_text()

    def nginx_t(self, nroot: Path, conf: str):
        if not REAL_NGINX:
            self.skipTest('nginx not installed')
        prefix = self.root / 'prefix'
        (prefix / 'logs').mkdir(parents=True)
        # 非 root 无法绑定 80/443：校验副本改用高位端口，其余内容原样
        testable = prefix / 'site.conf'
        testable.write_text(conf.replace(':80;', ':18480;').replace('listen 80;', 'listen 18480;')
                            .replace(':443 ssl;', ':18443 ssl;').replace('listen 443 ssl;', 'listen 18443 ssl;'))
        main = prefix / 'main.conf'
        main.write_text(textwrap.dedent(f'''\
            pid {prefix}/nginx.pid;
            error_log {prefix}/logs/error.log;
            events {{}}
            http {{
                access_log off;
                client_body_temp_path {prefix}/cb; proxy_temp_path {prefix}/px;
                fastcgi_temp_path {prefix}/fc; uwsgi_temp_path {prefix}/uw; scgi_temp_path {prefix}/sc;
                include {testable};
            }}
        '''))
        r = subprocess.run([REAL_NGINX, '-t', '-p', str(prefix), '-c', str(main)],
                           capture_output=True, text=True, timeout=30)
        self.assertEqual(r.returncode, 0, r.stderr + '\n' + conf)

    def test_http_config_is_valid_and_hardened(self):
        nroot, conf = self.render(tls=False)
        for needle in ('client_max_body_size 2g;', 'proxy_read_timeout 3600s;',
                       'location = /index.html', 'X-Frame-Options', f'include {nroot.parent}/opt/frontend/dist/csp.conf;',
                       'try_files $uri $uri/ /index.html;', 'proxy_pass http://127.0.0.1:18080;'):
            self.assertIn(needle, conf)
        self.assertNotIn('\\$', conf, '转义残留会让 nginx 变量失效')
        # Doc/33 P-6：CSP 经 include 单一来源，站点配置里不再内联策略
        self.assertNotIn('add_header Content-Security-Policy', conf)
        self.assertEqual(conf.count('/frontend/dist/csp.conf;'), 3)
        self.nginx_t(nroot, conf)

    def test_missing_csp_snippet_gets_fallback_and_existing_is_kept(self):
        # 旧版前端包无 csp.conf → 写降级片段（nginx -t 需要被 include 的文件存在）
        self.render(tls=False)
        snippet = self.root / 'opt' / 'frontend' / 'dist' / 'csp.conf'
        text = snippet.read_text()
        self.assertEqual(text, f'add_header Content-Security-Policy "{CSP_FALLBACK}" always;\n')
        self.assertNotIn('sha256-', text)

    def test_existing_csp_snippet_is_not_overwritten(self):
        dist = self.root / 'opt' / 'frontend' / 'dist'
        dist.mkdir(parents=True)
        built = 'add_header Content-Security-Policy "script-src \'self\' \'sha256-x\'" always;\n'
        (dist / 'csp.conf').write_text(built)
        self.render(tls=False)
        self.assertEqual((dist / 'csp.conf').read_text(), built)

    def test_tls_config_redirects_http_and_is_valid(self):
        nroot, conf = self.render(tls=True)
        self.assertIn('return 301 https://$host$request_uri;', conf)
        self.assertIn('listen 443 ssl;', conf)
        # 80 重定向 server 亦下发 CSP：两个 server + 两个自带 add_header 的 location
        self.assertEqual(conf.count('/frontend/dist/csp.conf;'), 4)
        self.nginx_t(nroot, conf)


class UninstallRollback(Sandbox):
    SECTION = between(UNINSTALL, '# ─── 删除 nginx 反向代理配置', '# ─── 删除 systemd 服务文件')

    def test_restores_default_site_and_include(self):
        nroot = self.root / 'nginx'
        (nroot / 'conf.d').mkdir(parents=True)
        (nroot / 'sites-available').mkdir()
        (nroot / 'sites-enabled').mkdir()
        (nroot / 'sites-available' / 'default').write_text('server {}\n')
        (nroot / 'conf.d' / 'flamepanel.conf').write_text('server {}\n')
        (nroot / 'nginx.conf').write_text(f'http {{\n    include {nroot}/conf.d/*.conf;  # flamepanel\n}}\n')
        inst = self.root / 'opt'
        inst.mkdir()
        (inst / '.install-state').write_text(
            f'NGINX_CONF_PATH={nroot}/conf.d/flamepanel.conf\nNGINX_INCLUDE_ADDED=1\n'
            f'NGINX_DEFAULT_SITE_DISABLED=1\nNGINX_ROOT={nroot}\n')
        self.stub('nginx')
        self.stub('systemctl')
        r = subprocess.run(['bash', '-c', 'set -e\n' + self.SECTION],
                           env={'PATH': f'{self.bin}:/usr/bin:/bin', 'INSTALL_DIR': str(inst),
                                'NGINX_CONF': '/nonexistent/flamepanel.conf',
                                'RED': '', 'GREEN': '', 'YELLOW': '', 'CYAN': '', 'NC': ''},
                           capture_output=True, text=True, timeout=30)
        self.assertEqual(r.returncode, 0, r.stderr)
        self.assertFalse((nroot / 'conf.d' / 'flamepanel.conf').exists())
        self.assertTrue((nroot / 'sites-enabled' / 'default').is_symlink())
        self.assertNotIn('# flamepanel', (nroot / 'nginx.conf').read_text())


class StaticChecks(unittest.TestCase):
    def test_install_root_is_traversable_by_nginx(self):
        self.assertIn('chmod 711 "$INSTALL_DIR"', INSTALL)
        self.assertNotIn('chmod 750 "$INSTALL_DIR" ', INSTALL)

    def test_no_hardcoded_nologin_or_conf_d_path(self):
        self.assertNotIn('--shell /usr/sbin/nologin', INSTALL)
        self.assertNotIn('cat > /etc/nginx/conf.d/flamepanel.conf', INSTALL)

    def test_systemd_is_required_before_any_change(self):
        self.assertLess(INSTALL.index('SYSTEMD_RUN_DIR'), INSTALL.index('pkg_install "${DEPS[@]}"'))


if __name__ == '__main__':
    unittest.main(verbosity=2)
