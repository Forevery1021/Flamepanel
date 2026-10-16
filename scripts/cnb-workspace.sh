#!/usr/bin/env bash
# 在 CNB 云原生开发环境里跑全量门禁（Doc/31 §5.3）——开发期不消耗云原生构建额度。
#
# 需要：CNB_TOKEN（个人访问令牌，含 repo-cnb-trigger 等权限）、Python 3、ssh、ssh-keyscan、tar。
#   bash scripts/cnb-workspace.sh start [分支]      启动（或复用）开发环境，打印 SN 与 ssh 目标
#   bash scripts/cnb-workspace.sh sync  <SN>        把本地工作区（含未提交改动）同步到远端 $CNB_CI_DIR
#   bash scripts/cnb-workspace.sh run   <SN> [组…]  远端执行 scripts/ci-local.sh（默认 all --keep-going），前台等待
#   bash scripts/cnb-workspace.sh bg    <SN> [组…]  同上但后台执行（断开 ssh 不中断），日志 /root/ci-local.log
#   bash scripts/cnb-workspace.sh log   <SN> [行数]  查看后台运行的日志尾部与进度（完成后含 RESULT 行）
#   bash scripts/cnb-workspace.sh hold  <SN>        保持一条 ssh 连接直到后台运行结束（见下方「保活」）
#   bash scripts/cnb-workspace.sh ssh   <SN> [命令] 进入 / 在远端执行任意命令
#   bash scripts/cnb-workspace.sh stop  <SN>        停止开发环境（需令牌含用户级权限；否则闲置自动回收）
#
# 环境变量：CNB_REPO（默认 yin0410/Flamepanel）、CNB_API（默认 https://api.cnb.cool）、
#           CNB_CI_DIR（远端门禁目录，默认 /root/fp-ci）。
#
# 注意：**从不写 /workspace**。CNB 会把上一个开发环境的未提交改动恢复进新环境的 /workspace，
# 那里可能有他人的在制品；门禁只在独立目录里跑，target/ 与 node_modules 跨次复用。
# 令牌只从环境读取，不落盘、不回显。
#
# 保活：CNB 在 10 分钟检测不到环境内的 http/ssh 连接时回收开发环境（keepAliveTimeout 默认值）。
# `bg` 起的进程已脱离 ssh，不算连接——超过 10 分钟的任务（release 构建、perf 压测）必须同时
# 挂一个 `hold`（前台或后台进程均可）：它持有 ssh 直到日志出现 RESULT 行后退出，
# 环境随后约 10 分钟自动回收，因此即使令牌无权调用 stop 也不会长期占用额度。
set -euo pipefail

REPO="${CNB_REPO:-yin0410/Flamepanel}"
API="${CNB_API:-https://api.cnb.cool}"
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SSH_OPTS=(-o StrictHostKeyChecking=yes -o GlobalKnownHostsFile=/dev/null
  -o KnownHostsCommand=none -o VerifyHostKeyDNS=no -o UpdateHostKeys=no
  -o HostKeyAlgorithms=rsa-sha2-512,rsa-sha2-256 -o BatchMode=yes
  -o ServerAliveInterval=30 -o ConnectTimeout=20)
SSH_VERIFY_DIR=""

die() { echo "cnb-workspace: $*" >&2; exit 1; }

# 首次连接也必须匹配官方固定指纹，不依赖用户已有 known_hosts / TOFU。
# 官方来源：https://docs.cnb.cool/zh/workspaces/fingerprint.html
# 指纹轮换须核对官方公告后显式更新；禁止通过环境变量或 accept-new 绕过。
cleanup_ssh() {
  [ -z "$SSH_VERIFY_DIR" ] || rm -rf -- "$SSH_VERIFY_DIR"
}
trap cleanup_ssh EXIT

prepare_ssh() {
  local target="$1" candidate known
  # remoteSsh 来自 API，仍须限制主机与用户名，防止 SSH 选项注入。
  [[ "$target" =~ ^[A-Za-z0-9][A-Za-z0-9._-]*@cnb[.]space$ ]] \
    || die "SSH 目标无效：仅支持官方 cnb.space 开发环境"
  SSH_VERIFY_DIR="$(mktemp -d "${TMPDIR:-/tmp}/flamepanel-cnb-ssh.XXXXXXXX")" \
    || die "无法创建私有 SSH 校验目录"
  candidate="$SSH_VERIFY_DIR/candidate"
  known="$SSH_VERIFY_DIR/known_hosts"
  if ! (umask 077; ssh-keyscan -T 20 -t rsa cnb.space > "$candidate" 2>/dev/null); then
    die "无法取得 CNB SSH 主机公钥，拒绝连接"
  fi
  if ! python3 - "$candidate" "$known" <<'PY'
import base64
import binascii
import hashlib
import os
import sys

expected = 'SHA256:fnWZvpqd+VAIRJxaZdV1KVMFfDgcCjYrP2VSWQ68T/E'
try:
    with open(sys.argv[1], 'rb') as stream:
        raw = stream.read(65537)
    if len(raw) > 65536:
        raise ValueError('oversized scan')
    verified = set()
    for line in raw.decode('ascii').splitlines():
        line = line.strip()
        if not line or line.startswith('#'):
            continue
        fields = line.split()
        if len(fields) != 3 or fields[:2] != ['cnb.space', 'ssh-rsa']:
            raise ValueError('unexpected host or key type')
        blob = base64.b64decode(fields[2], validate=True)
        fingerprint = 'SHA256:' + base64.b64encode(hashlib.sha256(blob).digest()).decode().rstrip('=')
        if fingerprint != expected:
            raise ValueError('fingerprint mismatch')
        verified.add(' '.join(fields))
    if not verified:
        raise ValueError('no verified key')
    fd = os.open(sys.argv[2], os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
    with os.fdopen(fd, 'w') as stream:
        stream.write('\n'.join(sorted(verified)) + '\n')
except (OSError, ValueError, UnicodeError, binascii.Error):
    print('CNB SSH 公钥未通过官方指纹校验，拒绝连接；请核对官方指纹公告。', file=sys.stderr)
    raise SystemExit(1)
PY
  then
    die "SSH 主机验证失败（不回退自动信任）"
  fi
  SSH_OPTS+=(-o "UserKnownHostsFile=$known")
}
CI_DIR="${CNB_CI_DIR:-/root/fp-ci}"
case "$CI_DIR" in /workspace | /workspace/*) die "CNB_CI_DIR 不能位于 /workspace" ;; esac
need_token() { [ -n "${CNB_TOKEN:-}" ] || die "请设置 CNB_TOKEN"; }

api() { # api <METHOD> <path> [json]
  need_token
  curl -fsS -X "$1" -H "Authorization: Bearer ${CNB_TOKEN}" -H "Accept: application/json" \
    -H "Content-Type: application/json" ${3:+-d "$3"} "${API}/${REPO}/-/$2"
}

# 环境刚启动时 detail 可能返回空体 / 非 JSON：静默视为「尚无该字段」，由调用方轮询
json_get() {
  python3 -c "
import sys, json
try:
    print(json.load(sys.stdin).get('$1') or '')
except (ValueError, AttributeError):
    print('')"
}

target_of() { # 由 SN 取 ssh 目标（环境刚启动时需等待就绪）
  local sn="$1" t=""
  for _ in $(seq 1 30); do
    t="$(api GET "workspace/detail/${sn}" 2>/dev/null | json_get remoteSsh || true)"
    [ -n "$t" ] && { echo "$t"; return 0; }
    sleep 10
  done
  die "开发环境 ${sn} 5 分钟内未就绪"
}

# 远端：解包到门禁目录，删除清单外的文件（保留 target/ 与 node_modules/ 作为缓存）
# shellcheck disable=SC2016
REMOTE_SYNC='set -e; d="$1"; mkdir -p "$d"; cd "$d"; tar -xzf -
find . \( -path ./target -o -path ./node_modules -o -path ./frontend/node_modules \) -prune -o -type f -print \
  | sed "s#^\./##" | sort > /tmp/.sync-present
sort .sync-manifest > /tmp/.sync-want
comm -23 /tmp/.sync-present /tmp/.sync-want | while IFS= read -r f; do rm -f -- "$f"; done
rm -f .sync-manifest
echo "synced $(wc -l < /tmp/.sync-want) files -> $d"'

cmd="${1:-}"; shift || true
case "$cmd" in
  start)
    branch="${1:-main}"
    out="$(api POST workspace/start "{\"branch\":\"${branch}\"}")"
    sn="$(echo "$out" | json_get sn)"
    # 已有环境时接口不返回 sn，只返回 WebIDE 地址（…/vscode-web/<SN>-001）
    [ -n "$sn" ] || sn="$(echo "$out" | json_get url | sed -n 's#.*/vscode-web/\(cnb-[a-z0-9-]*\)-[0-9]\{3\}.*#\1#p')"
    [ -n "$sn" ] || die "无法解析 SN：$out"
    echo "SN=${sn}"
    echo "SSH=$(target_of "$sn")"
    ;;
  sync)
    sn="${1:?用法：sync <SN>}"; t="$(target_of "$sn")"; prepare_ssh "$t"
    # 以本地工作区为准：已跟踪 + 未跟踪未忽略的现存文件，附清单 .sync-manifest；
    # 远端目录里不在清单中的文件一律删除（target/、node_modules/ 作为构建缓存保留）。
    (
      cd "$ROOT"
      # 远端目录无 .git：用 .ci-rev 标注被测版本（ci-local.sh 报告读取）
      echo "$(git rev-parse --short HEAD)$(git diff --quiet HEAD || echo +dirty) $(git branch --show-current)" > .ci-rev
      { git ls-files -z --cached --others --exclude-standard; printf '.ci-rev\0'; } \
        | python3 -c "import sys,os; fs=[f for f in sys.stdin.buffer.read().split(b'\0') if f and os.path.lexists(f)]; open('.sync-manifest','wb').write(b'\n'.join(fs)+b'\n'); sys.stdout.buffer.write(b'\0'.join(fs+[b'.sync-manifest']))" \
        | tar --null -T - -czf -
      rm -f .sync-manifest .ci-rev
    ) | ssh "${SSH_OPTS[@]}" "$t" "bash -c $(printf %q "$REMOTE_SYNC") _ $(printf %q "$CI_DIR")"
    ;;
  run)
    sn="${1:?用法：run <SN> [组…]}"; shift; t="$(target_of "$sn")"; prepare_ssh "$t"
    groups="${*:-all}"
    ssh "${SSH_OPTS[@]}" "$t" "cd '$CI_DIR' && bash scripts/ci-local.sh --keep-going ${groups}"
    ;;
  bg)
    sn="${1:?用法：bg <SN> [组…]}"; shift; t="$(target_of "$sn")"; prepare_ssh "$t"
    groups="${*:-all}"
    ssh "${SSH_OPTS[@]}" "$t" "cd '$CI_DIR' && setsid nohup bash scripts/ci-local.sh --keep-going ${groups} \
      > /root/ci-local.log 2>&1 < /dev/null & echo 'started: tail via  cnb-workspace.sh log ${sn}'"
    ;;
  hold)
    sn="${1:?用法：hold <SN>}"; t="$(target_of "$sn")"; prepare_ssh "$t"
    # 每 60 s 输出一行进度：既是 CNB 心跳，也让调用方看到最新步骤；上限 16 h（环境最长寿命）
    ssh "${SSH_OPTS[@]}" -o ServerAliveInterval=30 "$t" 'for _ in $(seq 1 960); do
        if grep -q "^RESULT" /root/ci-local.log 2>/dev/null; then grep "^RESULT" /root/ci-local.log; exit 0; fi
        printf "[hold %s] %s\n" "$(date +%T)" "$(grep -a "▶" /root/ci-local.log 2>/dev/null | tail -1 | sed "s/\x1b\[[0-9;]*m//g")"
        sleep 60
      done'
    ;;
  log)
    sn="${1:?用法：log <SN> [行数]}"; t="$(target_of "$sn")"; prepare_ssh "$t"
    ssh "${SSH_OPTS[@]}" "$t" "pgrep -f 'ci-local.sh' >/dev/null && echo '[running]' || echo '[finished]'; \
      grep -a -E '^(PASS|FAIL|SKIP|RESULT|ci-local)|▶' /root/ci-local.log | sed 's/\x1b\[[0-9;]*m//g' | tail -n ${2:-40}"
    ;;
  ssh)
    sn="${1:?用法：ssh <SN> [命令]}"; shift; t="$(target_of "$sn")"; prepare_ssh "$t"
    # 不 exec：返回后由 EXIT trap 清理临时 known_hosts，并保留 SSH 退出码。
    ssh "${SSH_OPTS[@]}" "$t" "$@"
    ;;
  stop)
    sn="${1:?用法：stop <SN>}"
    need_token
    if out="$(curl -fsS -X POST -H "Authorization: Bearer ${CNB_TOKEN}" -H "Content-Type: application/json" \
      -d "{\"sn\":\"${sn}\"}" "${API}/workspace/stop" 2>&1)"; then
      echo "$out" | json_get message
    else
      # /workspace/* 是用户级接口，令牌需额外勾选用户相关权限；否则闲置后由平台自动回收
      echo "停止失败（${out##*error: }）：令牌可能缺少用户级权限；可在 WebIDE 页面手动关闭，或等待闲置自动回收" >&2
      exit 1
    fi
    ;;
  *) sed -n '2,13p' "$0"; [ -z "$cmd" ] && exit 0 || exit 2 ;;
esac
