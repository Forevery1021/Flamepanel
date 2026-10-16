# 06 · Testing and quality gates

> Condensed English translation. [Complete Chinese original](../../Doc/06-测试指南.md) is retained unchanged. Detailed tables, examples, and historical evidence omitted here still require unabridged translation. Source baseline: `7288d9b199e1b09b529fbaae38303f4c82f38612`.

## Sources of truth

Use `scripts/ci-local.sh` for current groups/commands, `frontend/vitest.config.ts` for coverage thresholds, and `frontend/scripts/check-bundle-size.mjs` for bundle budgets. Historical test counts and old green reports do not establish the current baseline.

```bash
bash scripts/ci-local.sh quick
bash scripts/ci-local.sh --keep-going all
```

Run these in an isolated CNB cloud-native development directory, not by triggering ordinary CNB build pipelines. Synchronize the final working tree, including uncommitted changes, and keep a live SSH connection for long-running jobs.

## Test layers

- Rust unit tests cover pure rules, state transitions, parsers, and safety checks.
- Integration tests exercise actual service/repository assembly, old SQLite schemas, migrations/seeds, authentication, authorization, and failures.
- Frontend tests install production-equivalent plugins with a fresh Pinia per mount and clean up on unmount.
- Playwright exercises the real SQLite backend and built frontend. An in-memory substitute misses production initialization and RBAC defects.
- Release asset regression tests validate metadata, exact artifact selection, manifest hashes, and unsafe inputs without publishing.

Coverage execution already runs the complete frontend unit suite; avoid running the same suite twice merely for separate log labels. Never weaken assertions, thresholds, or feature scope to obtain a green result.

## Evidence

Report final revision/dirty state and source digest, exact tool versions, resources, actual commands, exit codes, durations, and PASS/FAIL/SKIP per step. Save logs and reports before stopping the workspace. Baseline results cannot be relabeled as final-tree results.

`all` is not a performance/security/cross-architecture/real-system acceptance certificate. Performance/soak tools under `scripts/perf/`, cargo-audit, release matrices, native installation, and external multi-node recovery need explicit runs and real resources. State unexecuted checks.

## Failure handling

Test 401/403, malformed input, conflicts, missing capabilities, cancellation, timeouts, restart interruption, and sensitive-data redaction. Use bounded fault injection for disk/network errors rather than filling the host disk. Stop after three unsuccessful attempts at the same error and record the unresolved cause.
