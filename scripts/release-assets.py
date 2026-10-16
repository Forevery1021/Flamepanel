#!/usr/bin/env python3
"""Validate release metadata and hash only the expected distributable archives."""
from __future__ import annotations

import hashlib
import os
from pathlib import Path
import re
import sys
import tomllib

ROOT = Path(__file__).resolve().parents[1]
ARCHES = ("amd64", "arm64", "amd64-musl", "arm64-musl")
SEMVER = re.compile(
    r"(?:0|[1-9][0-9]*)\.(?:0|[1-9][0-9]*)\.(?:0|[1-9][0-9]*)"
    r"(?:-(?:0|[1-9][0-9]*|[0-9]*[A-Za-z-][0-9A-Za-z-]*)"
    r"(?:\.(?:0|[1-9][0-9]*|[0-9]*[A-Za-z-][0-9A-Za-z-]*))*)?"
    r"(?:\+[0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*)?"
)


def resolve_version(env: dict[str, str], package_version: str) -> str:
    requested = env.get("RELEASE_VERSION", "").strip()
    tag = env.get("GITHUB_REF", "")
    tagged = tag.removeprefix("refs/tags/v") if tag.startswith("refs/tags/v") else ""
    value = tagged or requested or package_version
    if not SEMVER.fullmatch(value):
        raise ValueError("Release version must be SemVer without a v prefix")
    if requested and tagged and requested != tagged:
        raise ValueError("Input version does not match the tag")
    if value != package_version:
        raise ValueError("Release version must match flame-kernel/Cargo.toml")
    return value


def write_checksums(directory: Path, version: str) -> Path:
    if not SEMVER.fullmatch(version):
        raise ValueError("Invalid manifest version")
    expected = {"flamepanel-frontend.tar.gz"}
    expected.update(f"flamepanel-linux-{arch}.tar.gz" for arch in ARCHES)
    expected.update(f"flamepanel-agent-linux-{arch}.tar.gz" for arch in ARCHES)
    actual = {path.name for path in directory.iterdir()}
    if actual != expected:
        raise ValueError(f"Unexpected asset set; missing={sorted(expected - actual)}, extra={sorted(actual - expected)}")
    lines = []
    for name in sorted(expected):
        path = directory / name
        if path.is_symlink() or not path.is_file() or path.stat().st_size == 0:
            raise ValueError(f"Archive is not a nonempty regular file: {name}")
        with path.open("rb") as stream:
            digest = hashlib.file_digest(stream, "sha256").hexdigest()
        lines.append(f"{digest}  {name}\n")
    manifest = directory / f"flamepanel-{version}-checksums.txt"
    manifest.write_text("".join(lines), encoding="utf-8")
    return manifest


def main() -> int:
    try:
        with (ROOT / "flame-kernel/Cargo.toml").open("rb") as stream:
            package_version = tomllib.load(stream)["package"]["version"]
        version = resolve_version(dict(os.environ), package_version)
        if sys.argv[1:] == ["version"]:
            print(f"version={version}")
        elif len(sys.argv) == 3 and sys.argv[1] == "checksums":
            print(write_checksums(Path(sys.argv[2]), version))
        else:
            raise ValueError("Usage: release-assets.py version | checksums DIRECTORY")
        return 0
    except (ValueError, OSError, KeyError) as error:
        print(f"release-assets: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    sys.exit(main())
