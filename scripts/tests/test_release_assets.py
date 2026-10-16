"""Offline release metadata and artifact-selection regression tests."""
import hashlib
import importlib.util
from pathlib import Path
import tempfile
import unittest

SCRIPT = Path(__file__).resolve().parents[1] / "release-assets.py"
SPEC = importlib.util.spec_from_file_location("release_assets", SCRIPT)
release = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(release)


class ReleaseAssetsTests(unittest.TestCase):
    def test_manual_default_uses_package_not_branch_name(self):
        self.assertEqual(release.resolve_version({"GITHUB_REF": "refs/heads/main"}, "1.0.0"), "1.0.0")

    def test_matching_tag_and_input(self):
        self.assertEqual(release.resolve_version({"GITHUB_REF": "refs/tags/v1.0.0", "RELEASE_VERSION": "1.0.0"}, "1.0.0"), "1.0.0")

    def test_invalid_and_injected_versions_rejected(self):
        for value in ("v1.0.0", "main", "01.0.0", "1.0.0-01", "../../escape", "$(id)", "1.0.0\nfoo=bar"):
            with self.subTest(value=value), self.assertRaises(ValueError):
                release.resolve_version({"RELEASE_VERSION": value}, "1.0.0")

    def test_version_must_match_binary_metadata(self):
        for env in ({"RELEASE_VERSION": "2.0.0"}, {"GITHUB_REF": "refs/tags/v2.0.0"}, {"GITHUB_REF": "refs/tags/v1.0.0", "RELEASE_VERSION": "1.0.1"}):
            with self.subTest(env=env), self.assertRaises(ValueError):
                release.resolve_version(env, "1.0.0")

    def test_prerelease_and_build_metadata(self):
        value = "1.0.0-rc.2+build.7"
        self.assertEqual(release.resolve_version({}, value), value)

    def populate(self, directory):
        names = {"flamepanel-frontend.tar.gz"}
        for arch in release.ARCHES:
            names.update((f"flamepanel-linux-{arch}.tar.gz", f"flamepanel-agent-linux-{arch}.tar.gz"))
        for name in names:
            (directory / name).write_bytes(b"fixture archive")
        return names

    def test_manifest_contains_exactly_nine_archives(self):
        with tempfile.TemporaryDirectory() as tmp:
            directory = Path(tmp)
            names = self.populate(directory)
            manifest = release.write_checksums(directory, "1.0.0")
            rows = manifest.read_text().splitlines()
            self.assertEqual([line.split("  ", 1)[1] for line in rows], sorted(names))
            self.assertTrue(all(line.startswith(hashlib.sha256(b"fixture archive").hexdigest()) for line in rows))

    def test_missing_frontend_rejected(self):
        with tempfile.TemporaryDirectory() as tmp:
            directory = Path(tmp)
            self.populate(directory)
            (directory / "flamepanel-frontend.tar.gz").unlink()
            with self.assertRaises(ValueError):
                release.write_checksums(directory, "1.0.0")

    def test_unexpected_artifact_directory_and_export_rejected(self):
        for name in ("frontend-dist", "export-openapi", "extra.tar.gz"):
            with self.subTest(name=name), tempfile.TemporaryDirectory() as tmp:
                directory = Path(tmp)
                self.populate(directory)
                (directory / name).mkdir()
                with self.assertRaises(ValueError):
                    release.write_checksums(directory, "1.0.0")

    def test_empty_and_symlink_archive_rejected(self):
        for symlink in (False, True):
            with self.subTest(symlink=symlink), tempfile.TemporaryDirectory() as tmp:
                directory = Path(tmp)
                self.populate(directory)
                path = directory / "flamepanel-frontend.tar.gz"
                path.unlink()
                path.symlink_to(directory / "flamepanel-linux-amd64.tar.gz") if symlink else path.touch()
                with self.assertRaises(ValueError):
                    release.write_checksums(directory, "1.0.0")


if __name__ == "__main__":
    unittest.main()
