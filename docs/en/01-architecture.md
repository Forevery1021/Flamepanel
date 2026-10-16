# 01 · Architecture and code navigation

> Condensed English translation. [Complete Chinese original](../../Doc/01-架构设计.md) is retained unchanged. Detailed tables, examples, and historical evidence omitted here still require unabridged translation. Source baseline: `7288d9b199e1b09b529fbaae38303f4c82f38612`.

## System boundary

```text
Browser → Vue pages/composables → HTTP/WebSocket
nginx → non-root flame-kernel
         ├─ SQLite: business state and audit
         ├─ Docker, web engines, files, runtime adapters
         ├─ local privileged helper: Unix socket + allowlists
         └─ remote lightweight agent: HTTP / optional outbound Edge
```

The panel manages existing software; it does not replace Docker, database engines, or web servers. Docker socket access changes the privilege boundary and must not be described as a low-privilege deployment.

## Layers and startup

The intended dependency direction is `api → application → domain ← infrastructure`. Domain code may use serialization/time/type utilities, but not I/O.

| Location | Responsibility |
| --- | --- |
| `flame-kernel/src/main.rs` | CLI dispatch, logs, configuration, backend selection, startup |
| `flame-kernel/src/lib.rs` | `new_with_backend`, `build_services`, repository/port/service injection |
| `api/routes.rs` | Route aggregation; handlers call services and return `AppError` |
| `application/` | Use cases, tasks, and domain events |
| `domain/entity/`, `domain/repository.rs` | Entities, repository interfaces, pure rules |
| `infrastructure/factory.rs` | InMemory/SQLite factories and I/O adapters |

Production selects `config.database.backend`, defaulting to SQLite. `OP_DATABASE_BACKEND=memory` explicitly selects memory. `FlameKernel::new()` is a convenience memory entry, not evidence of production defaults. A Docker mock fallback does not switch business storage from SQLite to memory.

Some services still reference concrete infrastructure. Do not expand that coupling; boundary refactoring requires separate review.

## Invariants

- `api/permissions.rs` is the permission mapping source. First match wins; unknown protected routes are denied.
- Share JWT and user/role cache invalidation. Authenticate WebSockets in the first frame, recheck permissions, and support in-band renewal. Never put access tokens in URLs.
- Log internal error chains; do not expose them to clients.
- OpenAPI annotations → `openapi.rs` → `frontend/openapi.json` → generated references/types. Schema-only DTOs are not automatically dead code.
- Inject `SharedCommandRunner` through domain execution ports. `application/execution_mode.rs` is a compatibility re-export, not duplicate implementation.
- Helper path/protocol allowlists must not become arbitrary panel sudo.
- Preserve `SecretCipher` legacy plaintext/fallback behavior unless a separate security decision changes it. Agent tokens, commands, and files are independent security boundaries.
- `TaskSupervisor` owns cancellation/shutdown. Broadcast plus subscriber persistence does not prove business-transaction/outbox atomicity.
- `MetricsHub` shares collection/cache/broadcast; history is persisted separately. Subscribers must handle lag and closure.

## Frontend, agent, and features

`frontend/src/main.ts` installs production plugins. Pages compose `Fp*` components; business logic belongs in composables/utils. The agent is already modular, including Edge and terminal support; do not add the full kernel as an agent dependency.

The default kernel features are SQLite, Docker, WASM, and email. Keep the SQLite-only feature subset valid. Rust, Node, npm, and lockfiles are authoritative; do not copy stale dependency tables into guides.

For a change: locate the route/permission/contract, follow it through service and repository/port, update tests and affected documentation, then validate the final tree in CNB. See [workflow](./31-development-workflow.md) and [cleanup evidence](./35-engineering-cleanup.md).
