#!/usr/bin/env bash
# 特权助手真机冒烟（Doc/32 W3-b）：需要 root，在一次性环境（CNB 云原生开发 / 容器 / 虚机）里跑，
# 不要在生产机上执行——会创建临时用户 fphelpersmoke 并在 /etc/nginx 下写入 / 删除测试文件。
#
#   bash scripts/helper-smoke.sh [flamepanel 二进制路径]   # 默认 target/debug/flame-kernel
#
# 覆盖：root 检查、socket 权限 0660 root:<面板组>、SO_PEERCRED（面板用户放行 / 其它用户拒绝）、
# 路径白名单与符号链接逃逸、命令白名单、nginx 在场时真实 `nginx -t` 经助手执行。
set -euo pipefail

BIN="${1:-target/debug/flame-kernel}"
USER_NAME=fphelpersmoke
SOCK_DIR=$(mktemp -d /run/fp-helper-smoke.XXXXXX)
SOCK="$SOCK_DIR/helper.sock"
PASS=0
FAIL=0

[[ $(id -u) -eq 0 ]] || { echo "must run as root" >&2; exit 2; }
[[ -x "$BIN" ]] || { echo "binary not found: $BIN" >&2; exit 2; }
command -v python3 >/dev/null || { echo "python3 required" >&2; exit 2; }

ok() { echo "PASS  $1"; PASS=$((PASS + 1)); }
bad() { echo "FAIL  $1"; FAIL=$((FAIL + 1)); }
check() { # check <描述> <期望子串> <实际>
    if [[ "$3" == *"$2"* ]]; then ok "$1"; else bad "$1 (expected *$2*, got: $3)"; fi
}

id "$USER_NAME" >/dev/null 2>&1 || useradd --system --no-create-home --shell /usr/sbin/nologin "$USER_NAME"
chmod 755 "$SOCK_DIR"
CREATED_NGINX_DIR=0
[[ -d /etc/nginx ]] || { mkdir -p /etc/nginx; CREATED_NGINX_DIR=1; }

cleanup() {
    [[ -n "${HELPER_PID:-}" ]] && kill "$HELPER_PID" 2>/dev/null || true
    rm -rf "$SOCK_DIR" /etc/nginx/fp-smoke /etc/nginx/fp-smoke-link
    [[ ${CREATED_WAF_LOG:-0} -eq 1 ]] && rm -f /var/log/nginx/flamepanel-waf.log
    [[ $CREATED_NGINX_DIR -eq 1 ]] && rm -rf /etc/nginx
    userdel "$USER_NAME" 2>/dev/null || true
}
trap cleanup EXIT

# 非 root 启动必须失败
out=$(runuser -u "$USER_NAME" -- "$BIN" helper --socket "$SOCK_DIR/x.sock" 2>&1 || true)
check "non-root start refused" "must run as root" "$out"

RUST_LOG=info "$BIN" helper --socket "$SOCK" --allow-user "$USER_NAME" 2>"$SOCK_DIR/helper.log" &
HELPER_PID=$!
for _ in $(seq 1 50); do [[ -S "$SOCK" ]] && break; sleep 0.1; done
[[ -S "$SOCK" ]] || { cat "$SOCK_DIR/helper.log"; bad "socket did not appear"; exit 1; }

perm=$(stat -c '%a %U:%G' "$SOCK")
check "socket mode/owner" "660 root:$(id -gn "$USER_NAME")" "$perm"

# req <运行身份> <JSON>：发一行请求，打印一行响应
req() {
    local as="$1" body="$2"
    local py='import socket,sys
s=socket.socket(socket.AF_UNIX); s.connect(sys.argv[1]); s.sendall(sys.argv[2].encode()+b"\n")
print(s.makefile().readline().strip())'
    if [[ "$as" == root ]]; then
        python3 -c "$py" "$SOCK" "$body" 2>&1 || true
    else
        runuser -u "$as" -- python3 -c "$py" "$SOCK" "$body" 2>&1 || true
    fi
}

check "panel user ping" '"ok":true' "$(req "$USER_NAME" '{"v":1,"op":"ping"}')"
check "root ping" '"ok":true' "$(req root '{"v":1,"op":"ping"}')"
# nobody 连不上（socket 0660）或被 SO_PEERCRED 拒绝，二者都算通过
out=$(req nobody '{"v":1,"op":"ping"}')
if [[ "$out" == *"Permission denied"* || "$out" == *'"kind":"forbidden"'* ]]; then
    ok "other user rejected"
else
    bad "other user rejected (got: $out)"
fi

check "write allowed path" '"ok":true' \
    "$(req "$USER_NAME" '{"v":1,"op":"write","path":"/etc/nginx/fp-smoke/a.conf","content":"# smoke\n"}')"
check "file owned by root" "root:root" "$(stat -c '%U:%G' /etc/nginx/fp-smoke/a.conf)"
check "read back" '"data":"# smoke\n"' \
    "$(req "$USER_NAME" '{"v":1,"op":"read","path":"/etc/nginx/fp-smoke/a.conf"}')"
check "writable sites dir" '"data":true' \
    "$(req "$USER_NAME" '{"v":1,"op":"writable","path":"/etc/nginx/fp-smoke"}')"
check "write /etc/shadow refused" '"kind":"forbidden"' \
    "$(req "$USER_NAME" '{"v":1,"op":"write","path":"/etc/shadow","content":"x"}')"
check "traversal refused" '"kind":"forbidden"' \
    "$(req "$USER_NAME" '{"v":1,"op":"read","path":"/etc/nginx/../shadow"}')"
ln -s /etc /etc/nginx/fp-smoke-link
check "symlink escape refused" '"kind":"forbidden"' \
    "$(req "$USER_NAME" '{"v":1,"op":"read","path":"/etc/nginx/fp-smoke-link/shadow"}')"
check "arbitrary command refused" '"kind":"forbidden"' \
    "$(req "$USER_NAME" '{"v":1,"op":"run","program":"bash","args":["-c","id"]}')"
check "unknown op refused" '"kind":"bad_request"' \
    "$(req "$USER_NAME" '{"v":1,"op":"exec","cmd":"id"}')"
check "remove" '"data":true' \
    "$(req "$USER_NAME" '{"v":1,"op":"remove","path":"/etc/nginx/fp-smoke/a.conf"}')"

# read_log（W3-d）：仅限 WAF 日志精确路径，拒绝符号链接替换
WAF_LOG=/var/log/nginx/flamepanel-waf.log
if [[ ! -e $WAF_LOG ]]; then
    mkdir -p /var/log/nginx
    printf '{"rules":"r1:1"}\n' >"$WAF_LOG"
    CREATED_WAF_LOG=1
    check "read_log waf log" '"data":"{\"rules\":\"r1:1\"}\n"' \
        "$(req "$USER_NAME" '{"v":1,"op":"read_log","path":"/var/log/nginx/flamepanel-waf.log","offset":0,"max":4096}')"
    rm -f "$WAF_LOG"; ln -s /etc/shadow "$WAF_LOG"
    check "read_log symlink refused" '"kind":"forbidden"' \
        "$(req "$USER_NAME" '{"v":1,"op":"read_log","path":"/var/log/nginx/flamepanel-waf.log","offset":0,"max":4096}')"
    rm -f "$WAF_LOG"
else
    check "read_log waf log" '"ok":true' \
        "$(req "$USER_NAME" '{"v":1,"op":"read_log","path":"/var/log/nginx/flamepanel-waf.log","offset":0,"max":16}')"
fi
check "read_log other file refused" '"kind":"forbidden"' \
    "$(req "$USER_NAME" '{"v":1,"op":"read_log","path":"/etc/shadow","offset":0,"max":16}')"

if command -v nginx >/dev/null && [[ -f /etc/nginx/nginx.conf ]]; then
    check "nginx -t via helper" '"exit_code":0' \
        "$(req "$USER_NAME" '{"v":1,"op":"run","program":"nginx","args":["-t"]}')"
else
    echo "SKIP  nginx -t via helper (nginx not installed)"
fi

grep -q "helper: write /etc/nginx/fp-smoke/a.conf" "$SOCK_DIR/helper.log" \
    && ok "audit log records writes" || bad "audit log records writes"

echo "helper smoke: $PASS passed, $FAIL failed"
[[ $FAIL -eq 0 ]]
