# 13 · Troubleshooting

> Condensed English translation. [Complete Chinese original](../../Doc/13-故障排查手册.md) is retained unchanged. Detailed tables, examples, and historical evidence omitted here still require unabridged translation. Source baseline: `7288d9b199e1b09b529fbaae38303f4c82f38612`.

## Diagnose from the failing boundary

Start with service health and logs; distinguish browser/static assets, reverse proxy, API, business storage, and external adapters. Use request IDs to correlate failures. Do not paste tokens, hashes, secret settings, or raw production databases into issue reports.

## Backend cannot start

Inspect systemd status/journal, effective configuration, listening ports, working directory, service UID, database/secret file permissions, disk space, and pinned toolchain/build provenance. Do not start a recovered production database merely to test `/health`: initialization can execute background effects.

## Frontend or API unavailable

Check nginx syntax/service, static asset paths, `/api` proxy routes, cloud security groups, host firewall, TLS, and the browser network console. Frontend HTML loading does not prove API connectivity. Keep security headers rather than disabling CSP to hide a configuration mismatch.

## Database and Docker failures

Verify actual backend selection and SQLite path, directory permissions, migrations, WAL companions, and free space. Never replace consistent snapshots with raw live-file copies. Docker absence/mock fallback is different from business database fallback. Review socket access deliberately; do not grant broad host privileges as a quick fix.

## Privileged operations and app installation

Inspect helper/agent availability, execution mode, allowlisted commands, path policy, package-manager support, and app-store task logs. A root-required operation should fail with a clear boundary, not be silently retried with arbitrary sudo. Check package source/signature/scanner results before diagnosing Compose or native installation.

## WebSockets

Verify proxy upgrade headers, first-frame auth, channel permissions, token renewal/revocation, and idle/backpressure handling. Do not put credentials in query strings to work around missing initial frames.

## Performance, upgrades, and recovery

Inspect shared metrics, process/memory/disk pressure, task queues, and actual workload before tuning. Preserve the canonical build/test profiles. For upgrades, verify checksums/version compatibility, keep previous executables and consistent data backups, and distinguish binary rollback from data recovery.

Use the complete Chinese guide for detailed diagnostic commands, password-reset procedures, and example API sessions. Confirm the current `fpctl` interface before running any account-changing command; this condensed guide intentionally does not guess a reset command.
