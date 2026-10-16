# 05 · Backend development

> Condensed English translation. [Complete Chinese original](../../Doc/05-后端开发指南.md) is retained unchanged. Detailed tables, examples, and historical evidence omitted here still require unabridged translation. Source baseline: `7288d9b199e1b09b529fbaae38303f4c82f38612`.

## Baseline

Start with [architecture](./01-architecture.md). Use the exact Rust toolchain in `rust-toolchain.toml`; both workspace packages inherit `rust-version`, with edition 2021. Preserve `Cargo.lock` and `--locked`.

```bash
cargo run --locked -p flame-kernel --bin flame-kernel
```

## Implement a use case vertically

1. Locate existing routes, service methods, domain types, repository traits, and callers before adding code.
2. Define pure domain rules/ports. Implement both storage backends where required.
3. Compose services and concrete adapters in `lib.rs::build_services`, not handlers.
4. Add the API route, permission mapping, input validation, and OpenAPI registration together.
5. Add success, denial, invalid input, conflict, and lifecycle regressions; update affected documents/contracts.

## Errors, commands, and persistence

Handlers return the shared `AppError`; use the project's JSON extractor and validation conventions. Log internal details privately rather than leaking SQL, paths, credentials, or provider errors.

Inject `SharedCommandRunner`. Never concatenate user-controlled shell arguments; helper and agent install policies enforce specific command/path boundaries. Parameterize SQL. Keep migrations and RBAC seed behavior compatible with old SQLite databases.

Do not infer production defaults from the memory constructor. SQLite and InMemory test paths both matter; production E2E must use real SQLite.

## Runtime and observability

Long-running work belongs in the task center/`TaskSupervisor`, with deadlines, cancellation, shutdown, and failure evidence. Use existing request/task IDs and tracing. Domain events decouple side effects; an in-process broadcast is not a transactional outbox guarantee.

A handler must not independently recreate JWT/auth caches or concrete infrastructure. New code must not increase known boundary debt. Preserve feature-gated builds and compatibility re-exports.

## Validation and release

Run the canonical gates in CNB development. They include formatting, clippy with warnings denied, locked tests, SQLite-only checks/tests, frontend gates, and real-backend E2E. Release builds, dependency security audits, performance tests, and real-system installation acceptance are separate evidence layers.

The complete Chinese guide retains full code templates, naming examples, pagination patterns, and historical refactoring details. Consult the relevant `.codebuddy/skills/` procedure before implementing a new module, migration, or endpoint.
