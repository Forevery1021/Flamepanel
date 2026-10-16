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
    find frontend/src -name "*.js" -not -name "*.d.ts" -delete
    find frontend/src -name "*.js.map" -delete

# 运行所有测试
test:
    cargo test --package flame-kernel

# 代码检查
check:
    cargo check --package flame-kernel

# 代码风格检查（前端 ESLint + 后端 Clippy）
lint:
    cd frontend && npm run lint
    cargo clippy --package flame-kernel -- -D warnings

# 后端格式检查（CI 必查：cargo fmt --check）
fmt:
    cargo fmt --all -- --check

# 后端格式化
fmt-fix:
    cargo fmt --all

# 前端类型检查
typecheck:
    cd frontend && npx vue-tsc --noEmit

# 前端单元测试（vitest）
test-unit:
    cd frontend && npm run test:unit

# 全量验证（发版前跑）：格式 + 测试 + lint + 类型检查 + 构建
check-full: fmt test lint typecheck test-unit build

# 打包发行资产（release-assets/，自动读取版本号）
release:
    ./scripts/package-release.sh release-assets

# 指定版本打包发行资产
release-v VERSION:
    ./scripts/package-release.sh release-assets {{VERSION}}

# 校验发行资产（SHA256 一致性）
release-verify:
    cd release-assets && sha256sum -c flamepanel-*-checksums.txt

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

# 门禁单一入口（Doc/31 §5.3）：just ci [组…]，组 = installer backend backend-features frontend e2e all quick
ci *GROUPS="quick":
    bash scripts/ci-local.sh {{GROUPS}}

# 性能 / 稳定性压测（Doc/32 §8）：just perf [--quick 0.1] [load.py 参数…]，报告在 perf-out/
perf *ARGS:
    bash scripts/perf/run.sh {{ARGS}}

# CNB 云原生开发环境：just ws start|sync|bg|log|stop|ssh <参数>（需 CNB_TOKEN 环境变量）
ws *ARGS:
    bash scripts/cnb-workspace.sh {{ARGS}}

# 重新生成 OpenAPI 快照（T-05：与 CI 漂移门禁同源）
openapi:
    cargo run -q -p flame-kernel --bin export-openapi > frontend/openapi.json

# 由 OpenAPI 快照生成 Markdown API 文档到 Doc/api/（Doc/32 §6；CI 以 --check 校验）
api-docs:
    python3 scripts/gen-api-docs.py

# 一步刷新：快照 + 前端类型 + Markdown 文档
api-refresh: openapi api-docs
    cd frontend && npm run openapi:generate

# 版本号单源（T-06）：一次修改 Cargo.toml × 2 + package.json + 刷新 Cargo.lock
bump VERSION:
    ./scripts/bump-version.sh {{VERSION}}

