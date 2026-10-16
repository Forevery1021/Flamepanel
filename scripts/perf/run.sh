#!/usr/bin/env bash
# Flamepanel 性能 / 稳定性压测一键入口（Doc/32 §8 · W2-b）。
#
# 流程：release 构建 → 临时状态目录起真实内核（SQLite）→ load.py 走 setup API 拿 token
#       → idle / load / soak 三场景 + /proc 采样 → 报告 → 停内核。
#
# 设计为在 **CNB 云原生开发环境**（scripts/cnb-workspace.sh）里跑，不消耗流水线核时；
# 也可在任意 Linux 机器上跑。依赖：cargo、python3（仅标准库）、curl。
#
# 用法：
#   scripts/perf/run.sh                 # 完整：idle 5min + load 10min(200 并发) + soak 30min
#   scripts/perf/run.sh --quick 0.1     # 冒烟：约 1/10 时长（soak 斜率仅记录不判定）
#   PERF_BIN=target/release/flame-kernel scripts/perf/run.sh   # 跳过构建
#   其余参数原样透传给 load.py（如 --concurrency 100 --scenarios idle,load）
#
# 产物：perf-out/perf-report.{json,md}、perf-out/kernel.log（内核 stdout/stderr）
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$REPO_ROOT"

OUT_DIR="${PERF_OUT:-$REPO_ROOT/perf-out}"
PORT="${PERF_PORT:-18090}"
BIN="${PERF_BIN:-}"

log() { printf '\033[1;36m[perf]\033[0m %s\n' "$*"; }
die() { printf '\033[1;31m[perf] %s\033[0m\n' "$*" >&2; exit 2; }

command -v python3 >/dev/null || die "需要 python3"
command -v curl >/dev/null || die "需要 curl"
[[ "$(uname -s)" == "Linux" ]] || die "仅支持 Linux（依赖 /proc 采样）"

if [[ -z "$BIN" ]]; then
  log "cargo build --release -p flame-kernel"
  cargo build --release -p flame-kernel
  BIN="$REPO_ROOT/target/release/flame-kernel"
fi
[[ -x "$BIN" ]] || die "内核二进制不存在：$BIN"

# 端口被占用即失败——压到别的实例上会得到毫无意义的数据
if (exec 3<>"/dev/tcp/127.0.0.1/$PORT") 2>/dev/null; then
  die "端口 $PORT 已被占用（设 PERF_PORT 换端口）"
fi

STATE_DIR="$(mktemp -d -t fp-perf.XXXXXX)"
mkdir -p "$OUT_DIR" "$STATE_DIR/files"
KPID=""
cleanup() {
  if [[ -n "$KPID" ]] && kill -0 "$KPID" 2>/dev/null; then
    kill -TERM "$KPID" 2>/dev/null || true
    for _ in $(seq 1 50); do kill -0 "$KPID" 2>/dev/null || break; sleep 0.1; done
    kill -KILL "$KPID" 2>/dev/null || true
  fi
  # 保留内核自身的分级日志文件，便于压测后排查
  if [[ -d "$STATE_DIR/logs" ]]; then
    rm -rf "$OUT_DIR/kernel-logs"
    cp -r "$STATE_DIR/logs" "$OUT_DIR/kernel-logs" 2>/dev/null || true
  fi
  rm -rf "$STATE_DIR"
}
trap cleanup EXIT

# 与 E2E 相同：CWD＝状态目录（内核把密钥写在 CWD 相对的 ./data 下，不能落进仓库）。
# OP_RATE_LIMIT_MAX 调到极大：压测来自单一 IP，默认 120 次/分钟会把一切变成 429。
log "启动内核：$BIN（端口 $PORT，状态目录 $STATE_DIR）"
(
  cd "$STATE_DIR"
  OP_PORT="$PORT" \
  OP_HOST=127.0.0.1 \
  OP_DATABASE_BACKEND=sqlite \
  OP_DATABASE_URL="sqlite:$STATE_DIR/perf.db?mode=rwc" \
  OP_FILE_ROOT="$STATE_DIR/files" \
  OP_TERMINAL_CWD="$STATE_DIR/files" \
  OP_RATE_LIMIT_MAX=100000000 \
  OP_LOG_DIR="$STATE_DIR/logs" \
  OP_LOG_LEVEL="${PERF_LOG_LEVEL:-warn}" \
  exec "$BIN"
) >"$OUT_DIR/kernel.log" 2>&1 &
KPID=$!

for _ in $(seq 1 120); do
  kill -0 "$KPID" 2>/dev/null || { tail -50 "$OUT_DIR/kernel.log" >&2; die "内核启动即退出"; }
  curl -fsS "http://127.0.0.1:$PORT/health" >/dev/null 2>&1 && break
  sleep 0.25
done
curl -fsS "http://127.0.0.1:$PORT/health" >/dev/null || die "内核 30s 内未就绪"
log "内核就绪（PID $KPID）"

set +e
python3 "$REPO_ROOT/scripts/perf/load.py" \
  --base-url "http://127.0.0.1:$PORT" --pid "$KPID" --setup --out "$OUT_DIR" "$@"
rc=$?
set -e

if grep -qiE 'panicked|thread .* panicked' "$OUT_DIR/kernel.log"; then
  log "内核日志出现 panic —— 判定失败"
  grep -iE -m5 'panicked' "$OUT_DIR/kernel.log" >&2 || true
  rc=1
fi
log "报告：$OUT_DIR/perf-report.md（退出码 $rc）"
exit "$rc"
