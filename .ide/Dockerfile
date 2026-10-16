# FlamePanel 云原生开发环境镜像（CNB Workspace）
# 基于 NPC 运行镜像（已含 Rust 编译器 + clippy + rustfmt + 编译依赖），
# Node 镜像标签与 .node-version 通过工具链测试保持一致；npm 版本来自 frontend/package.json。
FROM node:24.20.0-bookworm-slim AS node-toolchain
FROM docker.cnb.cool/yin0410/flamepanel/npc-env:latest

# 使用确定的官方 Node 二进制，不再混用 NodeSource /usr/bin 与继承的 /usr/local/bin。
COPY --from=node-toolchain /usr/local/bin/node /usr/local/bin/node
COPY --from=node-toolchain /usr/local/lib/node_modules/npm /opt/flamepanel-npm
RUN ln -sf /opt/flamepanel-npm/bin/npm-cli.js /usr/local/bin/npm \
    && ln -sf /opt/flamepanel-npm/bin/npx-cli.js /usr/local/bin/npx

# Exact Rust toolchain comes from the repository, not the mutable base image.
COPY rust-toolchain.toml /opt/flamepanel-toolchain/rust-toolchain.toml
RUN version="$(sed -n 's/^channel = "\(.*\)"/\1/p' /opt/flamepanel-toolchain/rust-toolchain.toml)" \
    && test -n "$version" \
    && rustup toolchain install "$version" --profile minimal --component rustfmt --component clippy \
    && rustup default "$version" \
    && test "$(rustc --version | cut -d ' ' -f 2)" = "$version"

COPY .node-version /opt/flamepanel-toolchain/.node-version
COPY frontend/package.json /opt/flamepanel-toolchain/frontend/package.json
RUN test "$(node --version)" = "v$(cat /opt/flamepanel-toolchain/.node-version)" \
    && npm config set registry https://registry.npmjs.org/ \
    && npm install --global "$(node -p "require('/opt/flamepanel-toolchain/frontend/package.json').packageManager")" \
    && test "$(npm --version)" = "$(node -p "require('/opt/flamepanel-toolchain/frontend/package.json').packageManager.slice(4)")"

RUN apt-get update \
    && apt-get install -y --no-install-recommends openssh-server git-lfs rsync jq vim lsof htop net-tools dnsutils unzip sqlite3 \
    && rm -rf /var/lib/apt/lists/*

# CNB Remote-SSH 使用 UsePAM=no，不能依赖镜像 ENV 或用户 shell 配置。
# 由服务端为每个 SSH 会话补齐 Rust home（与 .cnb/npc/Dockerfile 一致）。
COPY .ide/10-flamepanel-rust.conf /etc/ssh/sshd_config.d/10-flamepanel-rust.conf
RUN chmod 0644 /etc/ssh/sshd_config.d/10-flamepanel-rust.conf \
    && grep -Eq '^[[:space:]]*Include[[:space:]]+/etc/ssh/sshd_config.d/\*\.conf([[:space:]]|$)' /etc/ssh/sshd_config

# just（仓库 justfile 任务入口）
RUN cargo install just --locked || true

# WebIDE
RUN curl -fsSL https://code-server.dev/install.sh | sh \
    && code-server --install-extension cnbcool.cnb-welcome \
    && code-server --install-extension rust-lang.rust-analyzer \
    && code-server --install-extension Vue.volar \
    && code-server --install-extension redhat.vscode-yaml \
    && code-server --install-extension mhutchie.git-graph \
    || true

ENV LANG=C.UTF-8 LANGUAGE=C.UTF-8
