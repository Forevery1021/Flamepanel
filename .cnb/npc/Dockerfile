# FlamePanel NPC 自定义运行镜像
# 在 cnbcool/default-npc 基础上预装 Rust 工具链与后端编译依赖，
# 使 NPC 会话可直接执行 AGENTS.md 中的后端门禁（fmt/clippy/test）。
# 由 main.push 流水线自动构建并推送：
#   docker.cnb.cool/yin0410/flamepanel/npc-env:latest
FROM cnbcool/default-npc:latest

# 后端编译依赖（可复现编译链）：
# - gcc/make/perl: cc 系列 crates（zstd-sys / libsqlite3-sys / openssl-src 等）
# - pkg-config + libssl-dev: openssl-sys 非 vendored 路径的探测兜底
# - file: openssl vendored 构建脚本探测依赖
# - python3-venv: 后端 Python 运行时用例需要 `python3 -m venv`（Debian 系需另装 ensurepip）
RUN apt-get update \
    && apt-get install -y --no-install-recommends \
         ca-certificates build-essential pkg-config libssl-dev file curl python3-venv \
    && rm -rf /var/lib/apt/lists/*

# Rust 1.98.1 工具链（与 rust-toolchain.toml 同步；最小 profile：rustc/cargo/rust-std，含 clippy 与 rustfmt 两个门禁组件）
ENV RUSTUP_HOME=/usr/local/rustup \
    CARGO_HOME=/usr/local/cargo \
    PATH=/usr/local/cargo/bin:$PATH
RUN curl -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal --default-toolchain 1.98.1 \
        -c clippy -c rustfmt \
    && chmod -R a+rwX $RUSTUP_HOME $CARGO_HOME \
    && rustc --version && cargo --version

# 提示 Rust 可用（可通过环境变量定位真实路径的 shell 包装）
RUN rm -f /usr/bin/rustc /usr/bin/cargo /usr/bin/rustup || true
RUN for t in rustc cargo rustup rustfmt cargo-clippy clippy-driver; do \
      printf '#!/bin/sh\nexec /usr/local/cargo/bin/%s "$@"\n' "$t" > /usr/local/bin/$t \
      && chmod +x /usr/local/bin/$t; \
    done

# crates.io 稀疏索引与 CI 内优化（国内构建机访问快）
RUN mkdir -p /usr/local/cargo \
    && printf '[net]\ngit-fetch-with-cli = true\n' > /usr/local/cargo/config.toml

# 冒烟自检
RUN cargo --version && rustc --version && cargo clippy --version
