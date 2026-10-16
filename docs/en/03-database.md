# 03 · Database design and migrations

> Condensed English translation. [Complete Chinese original](../../Doc/03-数据库设计.md) is retained unchanged. Detailed tables, examples, and historical evidence omitted here still require unabridged translation. Source baseline: `7288d9b199e1b09b529fbaae38303f4c82f38612`.

## Storage selection

SQLite is the production default selected by configuration in `main.rs`. The memory constructor is useful for tests, but does not change that default. Check `infrastructure/factory.rs` and `infrastructure/sqlite/mod.rs` for current pools, migrations, and seeds; older prose paths may predate module splitting.

## Schema families

- Identity: users, permissions, roles, role-permission associations, and two-factor records.
- Managed resources: nodes, websites, web servers, databases, firewall rules, runtimes, projects, certificates, DNS-provider accounts.
- Applications: packages, remote sources, installed apps, and plugins.
- Operations: settings, logs, audit records, backups/accounts/jobs, scheduled tasks, memos, persistent tasks, and outbox events.
- AI access: agent principals, sessions, tool-call audit, and approval-related records.

Exact columns, indexes, defaults, relationships, and historical upgrade behavior remain in the migrations and the complete Chinese tables. This edition intentionally does not invent a second schema definition.

## Migration discipline

`run_migrations` and RBAC seeding must remain idempotent. Adding a column requires considering existing rows, sensible defaults, old databases, and both repository implementations. Do not delete persisted enum values or status columns because static searches find no direct callers.

Use bound SQL parameters, not string interpolation of user values. Read/write pool configuration and WAL behavior belong to the actual SQLite factory, not a handwritten table.

## Backups and secrets

Use the application's consistent snapshot mechanism; a raw copy of a live WAL database is not a backup. A SQLite snapshot is not a complete recovery package: encryption/JWT/bootstrap material, ACME keys, certificate files, application files, volumes, and external databases may be separate.

Restore tests must use private copies, preserve source bytes, and never start normal production services against an untrusted recovered database. See [offline recovery](./37-sqlite-recovery-drill.md), [asset inventory](./39-recovery-assets.md), and [key matching](./40-recovery-keycheck.md).
