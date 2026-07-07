set dotenv-load

# 默认命令
default: dev

# 开发模式（后端热重载，需 cargo-watch）
dev:
    cargo watch -x 'run' --workdir flame-kernel

# 构建前端
build-frontend:
    cd frontend && npm run build

# 完整构建
build: build-frontend
    cargo build --release --package flame-kernel

# 运行
run: build
    ./target/release/flame-kernel

# 清理
clean:
    rm -rf flame-kernel/target frontend/dist data/*.db

# 运行所有测试
test:
    cargo test --package flame-kernel

# 代码检查
check:
    cargo check --package flame-kernel

# Docker 构建
docker-build:
    docker compose build --no-cache

# Docker 启动
docker-up:
    docker compose up -d

# Docker 停止
docker-down:
    docker compose down

# 查看日志
docker-logs:
    docker compose logs -f
