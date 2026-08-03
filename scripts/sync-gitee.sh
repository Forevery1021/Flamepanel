#!/usr/bin/env bash
# 一键同步 GitHub → Gitee 镜像（GitHub 为上游，Gitee 仅作镜像）
# 用法: ./scripts/sync-gitee.sh [remote名]  (默认 gitee)
set -euo pipefail

REMOTE="${1:-gitee}"
GITEE_URL="https://gitee.com/Forever1021yy/Flamepanel.git"

if ! git remote | grep -qx "$REMOTE"; then
  git remote add "$REMOTE" "$GITEE_URL"
  echo "已添加 remote: $REMOTE -> $GITEE_URL"
fi

echo "==> 拉取 GitHub 上游最新引用 (失败时继续，使用本地已有引用)..."
git fetch origin --prune --no-tags || echo "警告: fetch 失败（网络/限速），将按本地引用推送"

echo "==> 推送全部分支 + 标签到 Gitee (镜像覆盖)..."
git push "$REMOTE" --all --force
git push "$REMOTE" --tags --force

echo "==> 完成: Gitee 镜像已与 GitHub 同步"
