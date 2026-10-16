# Build from the repository root: docker build -f agent/Dockerfile .
FROM rust:1.98.1-slim AS builder
WORKDIR /app
COPY Cargo.toml Cargo.lock rust-toolchain.toml ./
COPY agent ./agent
# Cargo loads every workspace member's manifest, even for an Agent-only build.
COPY flame-kernel ./flame-kernel
RUN cargo build --locked --release --package flamepanel-agent

FROM debian:trixie-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/flamepanel-agent /usr/local/bin/flamepanel-agent
ENTRYPOINT ["/usr/local/bin/flamepanel-agent"]
