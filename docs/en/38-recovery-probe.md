# 38 · Dedicated recovery probe and HTTP acceptance

> Condensed English translation. [Complete Chinese original](../../Doc/38-恢复探针启动隔离.md) is retained unchanged. Detailed tables, examples, and historical evidence omitted here still require unabridged translation. Source baseline: `7288d9b199e1b09b529fbaae38303f4c82f38612`.

## Decision: separate assembly

Normal kernel startup can seed settings, reload WAF, run scheduled tasks/backups/renewals, sample metrics, clean records, scan nodes, handle outbox/notifications, and restore tasks. Visiting only `/health` does not prevent those side effects.

Use the separate `fp-restore-probe`, reusing only migrations, RBAC seeding, and constant-time comparison. It does not construct `FlameKernel`, Services, or RepoFactory; it has no production routes, command runner, Docker/Agent/SMTP/LLM clients, plugins, or production configuration/secret loading.

This is a deliberate assembly boundary, **not an OS sandbox**. Use trusted snapshots and a trusted binary; a filename is not cryptographic provenance.

## Operation

```bash
cargo build --locked -p flame-kernel --no-default-features --features sqlite --bin fp-restore-probe
python3 scripts/restore-drill/drill.py   --backup /secure/export/flamepanel-snapshot.db   --probe-bin "$PWD/target/debug/fp-restore-probe"   --out ./new-unique-probe-report --timeout 120
```

The driver uses a private cwd, cleared environment (PATH/LANG/TMPDIR only), its own process group, and an anonymous stdin pipe for a fresh 32-byte token. The probe creates another private copy, checks integrity/core schema, rejects active schema objects, migrates/seeds only that copy, closes writers, and reopens read-only before binding `127.0.0.1:0`.

No host/port override is accepted; snapshot cap is 512 MiB even if offline mode permits more. The protocol reports only fixed metadata/port/migration status, never tokens or values. The TTL and driver deadline bound work, with a further graceful-stop wait; filesystem/stdin/shutdown are not hard-real-time guarantees. SIGKILL can leave remnants.

## HTTP contract (not production)

| Request | Result |
| --- | --- |
| Authenticated `GET /health` | ready, fixed mode, `production_kernel=false` |
| Authenticated `GET /summary` | counts of users/websites/scheduled_tasks only |
| Missing/wrong/query-only token | 401 |
| Authenticated `POST /summary` | 405 |
| Authenticated production `/api/*` or `/ws/*` | 404 |

No login, sessions, arbitrary SQL, writes, CORS opening, or user-specified query paths. Queries have bounded waits and sanitized errors. Short-lived probe authentication does not prove JWT/password/2FA/RBAC recovery.

## Reports and acceptance

Only successful validation and normal termination set migration/HTTP verification true. `kernel_started=false`; production RTO/RPO stay null. Failure is FAIL, later steps NOT_RUN. Do not register these endpoints in production OpenAPI.

```bash
cargo test --locked -p flame-kernel --no-default-features --features sqlite --test restore_probe
```

Tests use synthetic old schemas and expired/enabled task records, preserve source bytes, reject writes/production routes/OP_* overrides, and verify deadline/error cleanup. Real production startup, external keys/data, business routes, and frontend loading remain unaccepted.

Historical Chinese instructions say probes were not packaged at that slice; the migration release workflow now explicitly includes recovery tools. That packaging change does not widen their execution authority.
