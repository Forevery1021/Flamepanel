#!/usr/bin/env bash
# 版本号单源（T-06）：一次修改三处版本 + 刷新 lockfile
# 用法: ./scripts/bump-version.sh <版本号>   例: ./scripts/bump-version.sh 0.9.0
set -euo pipefail

VER="${1:?用法: bump-version.sh <版本号>（如 0.9.0）}"
[[ "$VER" =~ ^[0-9]+\.[0-9]+\.[0-9]+([-.][0-9A-Za-z.]+)?$ ]] || { echo "非法版本号: $VER"; exit 1; }

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

# 1) flame-kernel/Cargo.toml + agent/Cargo.toml（仅 [package] 段首个 version）
for toml in flame-kernel/Cargo.toml agent/Cargo.toml; do
    sed -i "0,/^version = \".*\"/s//version = \"${VER}\"/" "$toml"
    echo "  -> $toml = $(grep -m1 '^version' "$toml")"
done

# 2) frontend/package.json（npm 官方命令，不产生 git tag）
(cd frontend && npm version --no-git-tag-version --allow-same-version "$VER" > /dev/null)
echo "  -> frontend/package.json = $(grep -m1 '"version"' frontend/package.json | tr -d ' ,')"

# 3) 刷新 Cargo.lock（离线，仅本 workspace 成员版本记录）
cargo update -p flame-kernel --offline > /dev/null 2>&1 || cargo update -p flame-kernel > /dev/null
cargo update -p flame-agent --offline > /dev/null 2>&1 || true
echo "  -> Cargo.lock 已刷新"

echo "完成：三处版本已对齐为 ${VER}（/health 与发行资产命名随 Cargo 版本自动生效）"
