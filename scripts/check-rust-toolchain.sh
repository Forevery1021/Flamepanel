#!/usr/bin/env bash
# Maintenance guard: rust-toolchain.toml is the exact supported compiler baseline.
# Do not infer the project's MSRV from dependency metadata or a floating stable channel.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

expected="$(sed -nE 's/^[[:space:]]*channel[[:space:]]*=[[:space:]]*"([0-9]+\.[0-9]+\.[0-9]+)"[[:space:]]*(#.*)?$/\1/p' rust-toolchain.toml)"
if [[ ! "$expected" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
  echo 'rust-toolchain.toml must declare an exact major.minor.patch channel' >&2
  exit 1
fi

actual="$(rustc --version)"
actual="${actual#rustc }"
actual="${actual%% *}"
if [[ "$actual" != "$expected" ]]; then
  printf 'Rust toolchain drift: expected %s, got %s. Check RUSTUP_TOOLCHAIN and rustup overrides.\n' "$expected" "$actual" >&2
  exit 1
fi

printf 'Rust toolchain OK: %s\n' "$expected"
cargo --version
