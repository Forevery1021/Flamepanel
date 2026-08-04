#!/bin/sh
set -e

# 启动 Nginx（前端静态资源 + API/WS 反向代理）
nginx -g 'daemon off;' &
NGINX_PID=$!

# 启动后端
exec /usr/local/bin/flamepanel
