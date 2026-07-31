FROM node:20-alpine as frontend-builder
WORKDIR /app/frontend
COPY frontend/package*.json ./
RUN npm ci --registry=https://registry.npmmirror.com
COPY frontend/ .
RUN npm run build

FROM rust:1.85-slim as backend-builder
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY flame-kernel ./flame-kernel
COPY agent ./agent
WORKDIR /app/flame-kernel
RUN cargo build --release --package flame-kernel

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates nginx curl && rm -rf /var/lib/apt/lists/*

# 复制 Rust 二进制
COPY --from=backend-builder /app/flame-kernel/target/release/flame-kernel /usr/local/bin/flamepanel

# 前端静态资源（由 nginx.conf 中 root /app/frontend/dist 提供服务）
COPY --from=frontend-builder /app/frontend/dist /app/frontend/dist

# Nginx 配置（生产反向代理）
COPY nginx.conf /etc/nginx/nginx.conf

EXPOSE 80 8080
VOLUME ["/app/data", "/var/run/docker.sock"]

WORKDIR /app
CMD ["flamepanel"]
