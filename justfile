set dotenv-load

# 默认命令
default: dev

# 开发模式（后端热重载）
dev:
    cargo watch -x 'run' --workdir backend

# 构建前端
build-frontend:
    cd frontend && npm run build

# 完整构建
build: build-frontend
    cargo build --release --package ops-panel-backend

# 运行
run: build
    ./target/release/ops-panel

# 清理
clean:
    rm -rf backend/target frontend/dist data/*.db

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