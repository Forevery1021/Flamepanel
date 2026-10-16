# 33 · Code audit and optimization history

> Condensed English translation. [Complete Chinese original](../../Doc/33-代码审计与优化报告.md) is retained unchanged. Detailed tables, examples, and historical evidence omitted here still require unabridged translation. Source baseline: `7288d9b199e1b09b529fbaae38303f4c82f38612`.

## Scope and provenance

The original audit describes `main@7f111bc` and the `chore/audit-optimization` branch. Its findings and historical validation are not a new security audit of the migration branch.

The review focused on command execution, files/archives, authentication, app-store trust, web-engine configuration, distribution adaptation, and installation/uninstallation. Implemented fixes and deferred work must be checked against current code/CHANGELOG before reclassification.

## Audit areas

- Backend command execution, SQL/data safety, filesystem/archive boundaries.
- App-store package handling, remote-source trust, scanning, and lifecycle.
- Native web-engine render/write/reload and privileged helper policies.
- HTTP validation, authentication/authorization, and error handling.
- Distribution-specific commands, paths, package sources, SELinux/firewalld, and installer rollback.

The Chinese report retains the exact A/B/C/D/E and P-1–P-6 findings, source evidence, and later endpoint reviews. This edition is a condensed audit navigation guide, not a finding-by-finding translated attestation.

## Important current distinctions

MCP, WS session termination, TOTP, release SHA-256 verification, and the shell-execution allowlist gate have existing implementations. Publication signatures remain separate. Do not describe every historical exclusion as still unimplemented, or a historical fix as newly verified.

Debian/Ubuntu are primary targets; RHEL-family native MySQL/PostgreSQL have limitations; openSUSE/Arch require capability-specific review; Alpine native systemd installation is unsupported by default.

## Engineering follow-up

Compare behavior before sharing duplicated logic. Keep secrets, compatibility fallbacks, feature paths, schema types, and RAII guards intact. New shell `-c` sites require explicit policy registration and review through `scripts/check-shell-exec.py`; do not add broad exemptions.

Use [Doc/35](./35-engineering-cleanup.md) for current evidence/status and [testing](./06-testing.md) for final-tree validation. Passing tests does not constitute a timing proof, penetration test, or independent supply-chain audit.
