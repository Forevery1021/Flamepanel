"""Migration invariants for retained Chinese references and selected comment-only edits.

The comment comparison is deliberately scoped to the reviewed entry files; it is
not a general Rust/Vue parser or proof about unrelated files.
"""
import hashlib
import json
from pathlib import Path
import re
import unittest

ROOT = Path(__file__).resolve().parents[2]
FIXTURES = Path(__file__).resolve().parent / "fixtures"


def digest(data):
    return hashlib.sha256(data).hexdigest()


def without_reviewed_comments(text, vue=False):
    if vue:
        text = re.sub(r"<!--.*?-->|/\*.*?\*/", "", text, flags=re.S)
    return "\n".join(line for line in text.splitlines() if line.strip() and not line.lstrip().startswith("//"))


class MigrationDocumentationTests(unittest.TestCase):
    def test_original_chinese_references_are_byte_preserved(self):
        fixtures = json.loads((FIXTURES / "chinese-doc-baseline.json").read_text())
        for name, expected in fixtures.items():
            with self.subTest(name=name):
                self.assertEqual(digest((ROOT / name).read_bytes()), expected)

    def test_selected_source_changes_are_comment_only(self):
        fixtures = json.loads((FIXTURES / "comment-only-baseline.json").read_text())
        for name, expected in fixtures.items():
            with self.subTest(name=name):
                current = without_reviewed_comments((ROOT / name).read_text(), name.endswith(".vue"))
                self.assertEqual(digest(current.encode()), expected)

    def test_new_english_relative_file_links_exist(self):
        pages = [ROOT / "README.md", ROOT / "AGENTS.md", *sorted((ROOT / "docs/en").glob("*.md"))]
        checked = 0
        for page in pages:
            text = re.sub(r"```.*?```", "", page.read_text(), flags=re.S)
            for target in re.findall(r"\]\(([^)]+)\)", text):
                if target.startswith(("https://", "http://", "#", "mailto:")):
                    continue
                target = target.split("#", 1)[0]
                if not target:
                    continue
                with self.subTest(page=page.name, target=target):
                    self.assertTrue((page.parent / target).exists(), "Missing relative file link")
                checked += 1
        self.assertGreater(checked, 60)

    def test_condensed_translations_are_not_claimed_as_full(self):
        numbered = sorted((ROOT / "docs/en").glob("[0-9][0-9]-*.md"))
        self.assertEqual(len(numbered), 27)
        for page in numbered:
            with self.subTest(page=page.name):
                self.assertIn("Condensed English translation", page.read_text())
                self.assertIn("Complete Chinese original", page.read_text())
        index = (ROOT / "docs/en/README.md").read_text()
        self.assertIn("Still pending", index)
        self.assertIn("not complete line-by-line translations", index)

    def test_default_entry_and_chinese_copy_link_each_other(self):
        self.assertIn("./README.zh-CN.md", (ROOT / "README.md").read_text())
        self.assertIn("./README.md", (ROOT / "README.zh-CN.md").read_text())
        self.assertTrue((ROOT / "AGENTS.zh-CN.md").is_file())


if __name__ == "__main__":
    unittest.main()
