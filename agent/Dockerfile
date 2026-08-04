FROM rust:1.97-slim as builder
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY agent ./agent
WORKDIR /app/agent
RUN cargo build --release

FROM debian:trixie-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/agent/target/release/flamepanel-agent /usr/local/bin/flamepanel-agent
ENTRYPOINT ["/usr/local/bin/flamepanel-agent"]
