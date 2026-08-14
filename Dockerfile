FROM node:20-alpine as frontend-builder
WORKDIR /app/frontend
COPY frontend/package*.json ./
RUN npm ci --registry=https://registry.npmmirror.com
COPY frontend/ .
RUN npm run build

FROM rust:1.97-slim as backend-builder
WORKDIR /app
# OpenSSL / pkg-config 构建依赖（openssl-sys / sqlx 需要）
RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*
COPY Cargo.toml Cargo.lock ./
COPY flame-kernel ./flame-kernel
COPY agent ./agent
WORKDIR /app/flame-kernel
# 国内 crates 镜像（加速依赖下载；境外环境可删）
RUN mkdir -p /usr/local/cargo/registry && \
    printf '[source.crates-io]\nreplace-with = "rsproxy"\n[source.rsproxy]\nregistry = "sparse+https://rsproxy.cn/index/"\n[registries.rsproxy]\nindex = "sparse+https://rsproxy.cn/index/"\n[net]\ngit-fetch-with-cli = true\n' > /usr/local/cargo/config.toml
# 指定 target 目录，产物路径确定（workspace 根 /app）
ENV CARGO_TARGET_DIR=/app/target
RUN cargo build --release --package flame-kernel

FROM debian:trixie-slim
RUN apt-get update && apt-get install -y ca-certificates nginx curl && rm -rf /var/lib/apt/lists/* \
    && useradd -r -s /usr/sbin/nologin nginx

# 复制 Rust 二进制（CARGO_TARGET_DIR=/app/target）
COPY --from=backend-builder /app/target/release/flame-kernel /usr/local/bin/flamepanel

# 前端静态资源（由 nginx.conf 中 root /app/frontend/dist 提供服务）
COPY --from=frontend-builder /app/frontend/dist /app/frontend/dist

# Nginx 配置（生产反向代理）
COPY nginx.conf /etc/nginx/nginx.conf

# 启动脚本：先起 nginx 再起后端（前端静态 + API/WS 反向代理）
COPY scripts/docker-entrypoint.sh /usr/local/bin/docker-entrypoint.sh
RUN chmod +x /usr/local/bin/docker-entrypoint.sh

# 沙箱工作区（文件/终端白名单根；镜像内 nginx 需绑定 80 特权端口且可挂载 docker.sock，
# 故容器保持 root 运行——非 root 加固针对宿主机 systemd 部署，见 Doc/06）
RUN mkdir -p /app/data /app/logs /app/workspace \
    && chmod 750 /app/data /app/logs /app/workspace

EXPOSE 80 8080
VOLUME ["/app/data", "/var/run/docker.sock"]

WORKDIR /app
ENV OP_FILE_ROOT=/app/workspace \
    OP_TERMINAL_CWD=/app/workspace
ENTRYPOINT ["/usr/local/bin/docker-entrypoint.sh"]
