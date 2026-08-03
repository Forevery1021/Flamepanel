#!/usr/bin/env bash
# 打包 GitHub Releases 资产（install.sh 依赖的产物）
# 用法: ./scripts/package-release.sh [输出目录]  (默认 release-assets)
#   生成:
#     flamepanel-linux-<ARCH>.tar.gz  顶层为 flamepanel 二进制
#     flamepanel-frontend.tar.gz      顶层为 dist 内容
set -euo pipefail

OUT="${1:-release-assets}"
mkdir -p "$OUT"

echo "==> 构建后端 (release)..."
cargo build --release --package flame-kernel

ARCH=$(uname -m)
case "$ARCH" in
    x86_64)  PKG_ARCH="amd64" ;;
    aarch64) PKG_ARCH="arm64" ;;
    *)       echo "不支持的架构: $ARCH"; exit 1 ;;
esac

echo "==> 打包后端 (linux-${PKG_ARCH})..."
rm -rf "$OUT/backend-stage"
mkdir -p "$OUT/backend-stage"
cp target/release/flame-kernel "$OUT/backend-stage/flamepanel"
tar -czf "$OUT/flamepanel-linux-${PKG_ARCH}.tar.gz" -C "$OUT/backend-stage" flamepanel
rm -rf "$OUT/backend-stage"

echo "==> 构建前端..."
(cd frontend && npm run build)

echo "==> 打包前端..."
tar -czf "$OUT/flamepanel-frontend.tar.gz" -C frontend/dist .

echo ""
echo "==> 产物列表:"
ls -lh "$OUT"
