#!/bin/sh
set -e

# 启动 Nginx（前端静态资源 + API/WS 反向代理）
nginx -g 'daemon off;' &
NGINX_PID=$!

# 启动后端
/usr/local/bin/flamepanel &
BACKEND_PID=$!

# 健康等待：后端就绪前不认为容器可对外（最多 60 秒）
i=0
until curl -fs http://127.0.0.1:8080/health >/dev/null 2>&1; do
    i=$((i + 1))
    if [ "$i" -ge 60 ]; then
        echo "flamepanel backend failed to become healthy in 60s" >&2
        kill "$BACKEND_PID" "$NGINX_PID" 2>/dev/null || true
        exit 1
    fi
    if ! kill -0 "$BACKEND_PID" 2>/dev/null; then
        echo "flamepanel backend exited during startup" >&2
        kill "$NGINX_PID" 2>/dev/null || true
        exit 1
    fi
    sleep 1
done

echo "flamepanel backend healthy; forwarding signals to children"

# 信号转发：SIGTERM/SIGINT 先通知后端优雅退出（nginx 无状态，随容器销毁即可）
forward_signal() {
    trap - "$1"
    kill -"$1" "$BACKEND_PID" 2>/dev/null || true
    wait "$BACKEND_PID" 2>/dev/null || true
    BACKEND_EXIT=$?
    kill "$NGINX_PID" 2>/dev/null || true
    exit "${BACKEND_EXIT:-0}"
}
trap 'forward_signal TERM' TERM
trap 'forward_signal INT' INT

# 等待后端退出（进程退出码即为容器退出码，便于 restart: unless-stopped 生效）
wait "$BACKEND_PID"
BACKEND_EXIT=$?
kill "$NGINX_PID" 2>/dev/null || true
exit "$BACKEND_EXIT"