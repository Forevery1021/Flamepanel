#!/usr/bin/env python3
"""Inventory line-leading Chinese comment candidates; never rewrite source.

This is a heuristic, not a language parser: comment-like text in strings may be
counted and trailing comments may be missed. The output is a review queue, not a
safe-deletion list or a CI threshold.
"""
import csv
from pathlib import Path
import re
import sys

ROOT = Path(__file__).resolve().parents[1]
BASES = ("flame-kernel", "agent", "frontend/src", "scripts")
SUFFIXES = {".rs", ".ts", ".vue", ".py", ".sh", ".mjs"}
COMMENT = re.compile(r"^\s*(?://|/\*|\*|#(?!\[)|<!--)")
CHINESE = re.compile(r"[\u3400-\u9fff]")


def inventory():
    rows = []
    for base in BASES:
        for path in (ROOT / base).rglob("*"):
            if not path.is_file() or path.suffix not in SUFFIXES:
                continue
            lines = path.read_text(encoding="utf-8", errors="replace").splitlines()
            count = sum(bool(COMMENT.match(line) and CHINESE.search(line)) for line in lines)
            if count:
                rows.append((path.relative_to(ROOT).as_posix(), count))
    return sorted(rows, key=lambda row: (-row[1], row[0]))


if __name__ == "__main__":
    writer = csv.writer(sys.stdout, lineterminator="\n")
    writer.writerow(("path", "chinese_comment_candidate_lines"))
    writer.writerows(inventory())
