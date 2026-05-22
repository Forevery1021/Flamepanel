FROM node:20-alpine as frontend-builder
WORKDIR /app/frontend
COPY frontend/package*.json ./
RUN npm ci --registry=https://registry.npmmirror.com
COPY frontend/ .
RUN npm run build

FROM rust:1.85-slim as backend-builder
WORKDIR /app
COPY --from=frontend-builder /app/frontend/dist ./backend/resources/static
COPY backend ./backend
WORKDIR /app/backend
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates nginx curl && rm -rf /var/lib/apt/lists/*

# 复制 Rust 二进制
COPY --from=backend-builder /app/backend/target/release/ops-panel /usr/local/bin/ops-panel

# Nginx 配置（生产反向代理）
COPY nginx.conf /etc/nginx/nginx.conf

EXPOSE 8080
VOLUME ["/app/data", "/var/run/docker.sock"]

WORKDIR /app
CMD ["ops-panel"]