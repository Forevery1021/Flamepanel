#!/usr/bin/env python3
"""Exercise the real CI entry script with inert fixture suites, without recursion."""
from pathlib import Path
import subprocess
import tempfile
import unittest

ENTRY = Path(__file__).resolve().parents[1] / 'check-install.sh'


class CiEntryTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory(prefix='flamepanel ci entry ')
        self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name)
        self.repo = self.root / 'repo with spaces'
        self.tests = self.repo / 'scripts' / 'tests'
        self.tests.mkdir(parents=True)
        self.entry = self.repo / 'scripts' / 'check-install.sh'
        self.entry.write_text(ENTRY.read_text())
        # Executing the installer would fail, even though bash -n must pass.
        (self.repo / 'install.sh').write_text('#!/bin/bash\nexit 97\n')
        (self.repo / 'uninstall.sh').write_text('#!/bin/bash\nexit 98\n')
        self.first = self.tests / 'test_install_download.py'
        self.second = self.tests / 'test_install_ci_entry.py'
        self.third = self.tests / 'test_install_platform.py'
        self.first.write_text("print('FIRST_SUITE_RAN')\n")
        self.second.write_text("print('SECOND_SUITE_RAN')\n")
        self.third.write_text("print('THIRD_SUITE_RAN')\n")

    def run_entry(self):
        env = {'PATH': '/usr/local/bin:/usr/bin:/bin',
               'HOME': str(self.root), 'LANG': 'C.UTF-8'}
        # Deliberately run outside the repository; entry must resolve its own root.
        return subprocess.run(['bash', str(self.entry)], cwd=self.root, env=env,
                              capture_output=True, text=True, timeout=10)

    def test_arbitrary_working_directory_and_spaces(self):
        result = self.run_entry()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn('FIRST_SUITE_RAN', result.stdout)
        self.assertIn('SECOND_SUITE_RAN', result.stdout)
        self.assertIn('THIRD_SUITE_RAN', result.stdout)

    def test_third_suite_failure_is_propagated(self):
        self.third.write_text('raise SystemExit(9)\n')
        result = self.run_entry()
        self.assertEqual(result.returncode, 9)

    def test_installer_syntax_failure_blocks_suites(self):
        (self.repo / 'install.sh').write_text('if then\n')
        result = self.run_entry()
        self.assertNotEqual(result.returncode, 0)
        self.assertNotIn('SUITE_RAN', result.stdout)

    def test_first_suite_failure_is_propagated(self):
        self.first.write_text('raise SystemExit(7)\n')
        result = self.run_entry()
        self.assertEqual(result.returncode, 7)
        self.assertNotIn('SECOND_SUITE_RAN', result.stdout)

    def test_second_suite_failure_is_propagated(self):
        self.second.write_text('raise SystemExit(8)\n')
        result = self.run_entry()
        self.assertEqual(result.returncode, 8)
        self.assertIn('FIRST_SUITE_RAN', result.stdout)

    def test_missing_suite_is_not_silently_skipped(self):
        self.first.unlink()
        result = self.run_entry()
        self.assertNotEqual(result.returncode, 0)
        self.assertNotIn('SECOND_SUITE_RAN', result.stdout)


if __name__ == '__main__':
    unittest.main(verbosity=2)
