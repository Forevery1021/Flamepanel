#!/usr/bin/env bash
# 打包 GitHub Releases 资产（install.sh 依赖的产物）
# 用法:
#   ./scripts/package-release.sh [输出目录] [版本号]
#   默认输出目录: release-assets
#   默认版本号:  从 Cargo.toml 自动读取（flame-kernel 的 package.version）
# 生成:
#   flamepanel-linux-<ARCH>.tar.gz      顶层为 flamepanel 二进制（本机架构）
#   flamepanel-frontend.tar.gz          顶层为 dist 内容
#   flamepanel-<VERSION>-checksums.txt  SHA256 校验和
set -euo pipefail

# ─── 参数 ──────────────────────────────────────────────────────────────────────
OUT="${1:-release-assets}"
VERSION="${2:-$(grep -m1 '^version' flame-kernel/Cargo.toml | sed 's/.*"\(.*\)".*/\1/')}"
mkdir -p "$OUT"
echo "==> FlamePanel 发行打包 v${VERSION}"

# ─── 校验产物完整性 ────────────────────────────────────────────────────────────
echo "==> 运行后端测试..."
cargo test --package flame-kernel > /dev/null 2>&1 && echo "  -> 测试通过" || { echo "  -> 测试失败，中止"; exit 1; }

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

# ─── 生成校验和 ────────────────────────────────────────────────────────────────
echo "==> 生成 SHA256 校验和..."
CHECKSUMS="$OUT/flamepanel-${VERSION}-checksums.txt"
: > "$CHECKSUMS"
for f in "$OUT"/flamepanel-linux-*.tar.gz "$OUT"/flamepanel-frontend.tar.gz; do
    [[ -f "$f" ]] || continue
    sha256sum "$f" | sed "s|$OUT/||" >> "$CHECKSUMS"
done

# ─── 产物信息 ──────────────────────────────────────────────────────────────────
echo ""
echo "==> 产物列表:"
ls -lh "$OUT"
echo ""
echo "==> 校验和 (${CHECKSUMS}):"
cat "$CHECKSUMS"

echo ""
echo "==> 发布提示:"
echo "  1. 打标签触发 CI 发布: git tag v${VERSION} && git push origin v${VERSION}"
echo "  2. 或手动将 ${OUT} 下产物上传到 GitHub Releases"
echo "  3. 安装脚本将自动下载 flamepanel-linux-${PKG_ARCH}.tar.gz 与 flamepanel-frontend.tar.gz"
