#!/usr/bin/env bash
# 全量质量门禁的单一入口（Doc/31 §5.3）。
#
# 开发期在 **CNB 云原生开发环境**（8C/16G，计入 1600 核时/月的开发额度）执行，
# 不消耗云原生构建额度（160 核时/月，只留给大版本 tag 构建）。
# 与 .cnb.yml 的 backend-gates / frontend-gates / e2e-playwright / installer-regression
# 同口径；两边增删检查项时必须同步。
#
# 用法：
#   bash scripts/ci-local.sh [--keep-going] [组 ...]
#   组：installer backend backend-features frontend e2e | all（默认，= 全部）| quick（installer + fmt + clippy + 前端静态检查）
#       perf（压测完整版 ≈45 min）| perf-quick（压测冒烟 ≈5 min）——不含在 all 内，按需显式指定；
#       额外参数经 PERF_ARGS 透传给 scripts/perf/load.py（如 PERF_ARGS="--concurrency 100"）
#   --keep-going  某步失败后继续跑完其余步骤（一次暴露全部问题）
#
# 结果：逐步 PASS/FAIL + 耗时，汇总写入 $CI_REPORT（默认 ci-local-report.txt，已 gitignore）。
# 退出码：0 全部通过；1 有失败；2 用法错误。
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT" || exit 2

KEEP_GOING=0
SEL=()
for arg in "$@"; do
  case "$arg" in
    --keep-going) KEEP_GOING=1 ;;
    -h | --help) sed -n '2,19p' "$0"; exit 0 ;;
    installer | backend | backend-features | frontend | e2e | all | quick | perf | perf-quick) SEL+=("$arg") ;;
    *) echo "未知参数：$arg（--help 查看用法）" >&2; exit 2 ;;
  esac
done
[ ${#SEL[@]} -eq 0 ] && SEL=(all)

REPORT="${CI_REPORT:-$ROOT/ci-local-report.txt}"
export CARGO_TERM_COLOR="${CARGO_TERM_COLOR:-always}" CARGO_INCREMENTAL=0 CI=true HUSKY=0
: > "$REPORT"
FAILED=0
RESULTS=()

step() { # step <名称> <命令...>
  local name="$1"; shift
  if [ "$FAILED" -ne 0 ] && [ "$KEEP_GOING" -eq 0 ]; then
    RESULTS+=("SKIP  ${name}"); return 0
  fi
  printf '\n\033[1;36m▶ %s\033[0m\n' "$name"
  local t0=$SECONDS
  if "$@"; then
    RESULTS+=("$(printf 'PASS  %-44s %4ss' "$name" $((SECONDS - t0)))")
  else
    RESULTS+=("$(printf 'FAIL  %-44s %4ss' "$name" $((SECONDS - t0)))")
    FAILED=1
  fi
}

in_frontend() { (cd frontend && "$@"); }

group_installer() {
  step "installer: offline regression" timeout --signal=TERM --kill-after=10s 120s bash scripts/check-install.sh
  step "installer: api docs up to date" python3 scripts/gen-api-docs.py --check
  step "installer: shell exec gate" python3 scripts/check-shell-exec.py
  step "installer: release assets regression" python3 -m unittest discover -s scripts/tests -p test_release_assets.py
  step "installer: migration invariants" python3 -m unittest discover -s scripts/tests -p test_migration_docs.py
}

group_backend_static() {
  step "backend: rustfmt" cargo fmt --all -- --check
  step "backend: clippy --all-targets -D warnings" cargo clippy --locked --all-targets -- -D warnings
}

group_backend() {
  group_backend_static
  # 含 OpenAPI 快照漂移单测（openapi::tests::frontend_snapshot_is_up_to_date）
  step "backend: cargo test --all" cargo test --locked --all --no-fail-fast
}

group_backend_features() {
  step "backend: check sqlite-only (all targets)" \
    cargo check --locked -p flame-kernel --no-default-features --features sqlite --all-targets
  step "backend: test sqlite-only (lib)" \
    cargo test --locked -p flame-kernel --no-default-features --features sqlite --lib
}

frontend_deps() {
  in_frontend npm run check:toolchain \
    && in_frontend timeout --signal=TERM --kill-after=15s 10m npm ci --no-audit --no-fund
}

group_frontend_static() {
  step "frontend: toolchain + npm ci" frontend_deps
  step "frontend: typecheck" in_frontend npm run typecheck
  step "frontend: lint" in_frontend npm run lint
  step "frontend: i18n keys" in_frontend npm run locale:check
}

group_frontend() {
  group_frontend_static
  # 覆盖率运行即完整单测，不再单独 vitest run
  step "frontend: unit tests + coverage" in_frontend npm run test:coverage
  step "frontend: build" in_frontend npm run build
  step "frontend: bundle size (200 KB gzip)" in_frontend npm run bundle:check
  step "frontend: npm audit (high)" in_frontend npm audit --audit-level=high
}

group_e2e() {
  local target="${CARGO_TARGET_DIR:-$ROOT/target}"
  step "e2e: build backend (debug)" cargo build --locked -p flame-kernel
  [ -d frontend/node_modules ] || step "e2e: npm ci" frontend_deps
  step "e2e: playwright chromium" in_frontend npx playwright install --with-deps chromium
  step "e2e: playwright" env E2E_BACKEND_BIN="$target/debug/flame-kernel" bash -c 'cd frontend && npm run e2e'
}

# 性能 / 稳定性（Doc/32 §8）：release 构建 + 真实内核 + idle/load/soak，报告写 perf-out/
group_perf() {
  local scale="$1"
  step "perf: load driver self-test" python3 -m unittest discover -s scripts/perf -p 'test_*.py'
  # shellcheck disable=SC2086  # PERF_ARGS 需按空白拆分
  # 每个缩放系数单独目录：perf-quick 与 perf 同跑时报告互不覆盖
  step "perf: idle/load/soak (quick=$scale)" env PERF_OUT="$ROOT/perf-out/quick-$scale" \
    bash scripts/perf/run.sh --quick "$scale" ${PERF_ARGS:-}
}

# Rust-dependent groups validate the supported compiler exactly once per run.
# installer/frontend-only invocations remain usable without a Rust toolchain.
for group in "${SEL[@]}"; do
  case "$group" in
    all | quick | backend | backend-features | e2e | perf | perf-quick)
      step "toolchain: pinned Rust" bash scripts/check-rust-toolchain.sh
      break ;;
  esac
done

for g in "${SEL[@]}"; do
  case "$g" in
    all) group_installer; group_backend; group_backend_features; group_frontend; group_e2e ;;
    quick) group_installer; group_backend_static; group_frontend_static ;;
    installer) group_installer ;;
    backend) group_backend ;;
    backend-features) group_backend_features ;;
    frontend) group_frontend ;;
    e2e) group_e2e ;;
    perf) group_perf 1 ;;
    perf-quick) group_perf 0.1 ;;
  esac
done

{
  if git rev-parse --git-dir >/dev/null 2>&1; then
    rev="$(git rev-parse --short HEAD)$(git diff --quiet HEAD || echo +dirty) $(git branch --show-current)"
  else
    rev="$(cat .ci-rev 2>/dev/null || echo unknown)"
  fi
  echo "ci-local  ${rev}  $(date '+%F %T %z')  groups: ${SEL[*]}"
  echo "host: $(nproc) cpu, $(free -g 2>/dev/null | awk '/Mem:/{print $2}') GiB"
  printf '%s\n' "${RESULTS[@]}"
  [ "$FAILED" -eq 0 ] && echo "RESULT: PASS" || echo "RESULT: FAIL"
} | tee "$REPORT"

exit "$FAILED"
