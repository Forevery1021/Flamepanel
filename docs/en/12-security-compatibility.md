# 12 · Security baseline and compatibility

> Condensed English translation. [Complete Chinese original](../../Doc/12-安全基线与兼容矩阵.md) is retained unchanged. Detailed tables, examples, and historical evidence omitted here still require unabridged translation. Source baseline: `7288d9b199e1b09b529fbaae38303f4c82f38612`.

## Operating-system scope

| Tier | Systems | Boundary |
| --- | --- | --- |
| 1 | Debian 12/13, Ubuntu 22.04/24.04 | Primary native support/regression targets |
| 2 | Rocky/Alma/RHEL/Oracle 9, CentOS Stream 9, Fedora; related RHEL/Debian families | Core panel support; native MySQL/PostgreSQL limitations; some systems lack real-machine evidence |
| 3 | openSUSE, Arch/Manjaro | Native software may require manual installation or be unavailable |
| Docker only | Alpine 3.19+ | Default OpenRC does not satisfy native systemd requirements |

Prebuilt release configuration targets amd64/arm64. A matrix entry is not proof that a specific distribution/architecture has passed real-machine installation. Target acceptance requires installation from scratch, healthy services, UI login, and functional resource reads.

The Chinese baseline recommends Linux 5.10+, systemd 245+, 512 MB minimum memory (1 GB+ recommended), and at least 1 GB disk excluding application images/data. Docker is optional for container management. Validate actual deployment requirements rather than treating these minima as measured performance guarantees.

## Pre-production checklist

- [ ] Run the API as a non-root user; separately review helper/agent/host capabilities.
- [ ] Do not mount Docker socket access by default or expose an agent publicly without a reviewed transport boundary.
- [ ] Restrict dangerous operations by command/path policy; do not interpolate arbitrary shell.
- [ ] Enable HTTPS, strong administrator credentials, expiry/lockout, and revocation behavior.
- [ ] Verify first-frame WebSocket authentication and channel RBAC.
- [ ] Check CSP, `nosniff`, frame restrictions, and referrer policy in actual responses.
- [ ] Use consistent SQLite backups/WAL behavior and rehearse recovery without production side effects.
- [ ] Canonicalize and enforce file roots; reject path traversal.
- [ ] Preserve optimistic-concurrency behavior: conflicting writes fail instead of silently overwriting.
- [ ] Pass dependency audits or record an explicit reviewed exception.
- [ ] Verify release SHA-256 manifests before extraction; signatures remain a separate supply-chain feature.

## Negative cases

Reject `../../etc/passwd`, shell metacharacter injection, unsupported actions, and unauthorized/expired tokens. Missing Docker access must fail explicitly. Concurrent configuration writes must preserve stable conflict semantics.

Agent database/runtime policies allow only exact constant scripts and validated positional arguments. Reject `sh -c id`, even one-character mutations of approved scripts, sudoers paths, and multiline Redis injection. Project units are restricted to approved paths/directives/executables and must reject traversal, shell syntax, execution prefixes, and `LD_*` variables.

## Intentional arbitrary-command features

Interactive terminals and user-authored scheduled tasks are explicit capabilities, not ordinary allowlisted operations. Remote node terminal execution runs as the Agent and bypasses `/action`, so it is off unless the node owner sets `AGENT_TERMINAL=1`. Require both `terminal:execute` and `node:execute`, audit open/close, enforce per-node limits, and close on invalid identity/revoked permission/idle expiry.

## Evidence versus target

The Chinese checklist includes historical aspirational topology statements. Treat it as an acceptance checklist, not proof that every item is implemented or tested. Current code, effective deployment configuration, and real acceptance results take precedence. Full distro-specific and security-policy detail remains in the Chinese baseline and audit report.
