# FlamePanel · Agent working agreement

[简体中文](./AGENTS.zh-CN.md)

Keep this entry under 120 lines. Architecture: Doc/01; workflow: Doc/31; task-specific procedures: `.codebuddy/skills/`. Do not copy historical test counts or gate thresholds into entry documents.

## 0. Find your way

- Product: a self-hosted Linux server panel built with Rust and Vue, licensed under MIT.
- Dependency direction: `api → application → domain ← infrastructure`. Domain code may use serde/chrono and type utilities, but must not perform I/O. Existing debt is in Doc/35; do not expand concrete infrastructure coupling.
- Composition root: `flame-kernel/src/lib.rs::build_services`. `main.rs` selects `config.database.backend`, defaulting to SQLite.
- Routes/permissions: `api/routes.rs`, `api/permissions.rs`. Migrations: `infrastructure/sqlite/mod.rs::run_migrations`. Share live sampling through `MetricsHub`.
- Frontend: Pinia, VueQuery, and i18n. Business pages use `Fp*`; pages compose, while composables/utils contain testable logic.
- Agent modules already include `config / heartbeat / server / executor / install_policy / files / pty / edge`; it is not a single-file program.
- The exact Rust toolchain is in `rust-toolchain.toml`; workspace `rust-version` declares the supported minimum. Node: `.node-version`; npm: `frontend/package.json::packageManager`. Do not float to stable or casually update lockfiles.

## 1. Scope and session discipline

- Establish the goal, allowed files, and prohibited changes first. Record plans against a real issue when provided; never invent a reference.
- Stop after three unsuccessful attempts at the same error. Record failures and attempted fixes; never delete tests or weaken assertions to obtain a pass.
- Search symbols before reading large files. Inspect the relevant sections; do not guess paths, signatures, or APIs.
- Engineering cleanup covers assembly, toolchains, and documentation. List runtime dead-code/duplication candidates in Doc/35 first; do not delete/merge them without approval or introduce features.

## 2. Gates and execution environment

```bash
bash scripts/ci-local.sh quick
bash scripts/ci-local.sh --keep-going all
```

- `scripts/ci-local.sh` defines commands/groups. Rust groups first run `scripts/check-rust-toolchain.sh`; preserve `--locked`, clippy `-D warnings`, and existing test scope.
- Develop/test in CNB cloud-native development, following Doc/31 §5.3. Never overwrite other people's uncommitted work in `/workspace`; use an isolated directory and save evidence before stopping.
- Even lightweight pipelines consume build quota. If zero ordinary cloud builds are requested, do not validate via push, PR, tag, or ordinary build-start. Push CI-skip does not cover PR events.
- Passing all gates does not prove performance, security auditing, cross-architecture release correctness, or real-system acceptance. State what was not executed.

## 3. Code boundaries

1. Handlers use `AppError`, do not expose internal error chains/secrets, and do not instantiate concrete infrastructure.
2. For REST changes, check routes, permissions, 401/403 regressions, OpenAPI annotations/registration, and generated contracts.
3. SQLite migrations must be idempotent. Consider defaults for new columns; test old databases and RBAC seeds.
4. Long tasks use the task center/`TaskSupervisor`, preserving cancellation, deadlines, shutdown, and failure logs.
5. Inject external commands via `SharedCommandRunner` and existing execution policy. Never interpolate untrusted shell arguments or bypass helper/Agent allowlists and path boundaries.
6. Use existing hashing/`SecretCipher` policies and redact logs. Never commit tokens. Compatibility/fallback changes require independent review.
7. Preserve `Fp*`, matching backend permissions, all three locale keys, generated types, and `--fp-*` tokens. Do not add `as any`, ignored types, or nonsemantic E2E selectors.
8. Test real assembly: create a fresh Pinia per mount, install production plugins as needed, and unmount. Do not silence `console.warn` to hide missing dependencies.
9. Do not force-push shared history, add undeclared dependencies, or remove dynamic/schema/RAII/feature/compatibility code to inflate cleanup counts.

## 4. Navigation

| Task | Read first |
| --- | --- |
| Architecture/security boundaries | Doc/01, Doc/35 |
| Issue/delivery workflow | `flamepanel-issue-execution` skill, Doc/31 §1–4 and §9 |
| Backend/API | backend-module/rest-endpoint skills, Doc/05, Doc/07, Doc/api |
| SQLite | sqlite-migration skill, Doc/03 |
| Frontend | frontend-view skill, Doc/04, `frontend/design.md` |
| Quality gates | quality-gates skill, Doc/06, `ci-local.sh` |
| Application packages | app-package skill, Doc/20–22 |
| Deployment/compatibility | Doc/10, Doc/12 |
| Agent/events | Doc/23, Doc/24 |
| Next steps | Doc/30; Doc/32 is historical, not an automatic feature queue |

English guides and their translation status: [docs/en/README.md](./docs/en/README.md). Original Chinese references remain under `Doc/`.

## 5. Delivery

- Follow Doc/31 for branches/commits. Use `Ref: #<issue>` only for a real issue; do not rewrite history.
- Report changed files, implementation, final source baseline, actual commands/results, unresolved items, and recommendations.
- Synchronize API/table/permission/deployment documentation per Doc/31 §7. Record delivery history in CHANGELOG rather than duplicating it across entry points.
- Without permission for external submission/build-triggering actions, deliver a branch/patch and review report. Never claim a push, merge, or release that did not happen.
