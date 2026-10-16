# 31 · Development workflow

> Condensed English translation. [Complete Chinese original](../../Doc/31-开发流程.md) is retained unchanged. Detailed tables, examples, and historical evidence omitted here still require unabridged translation. Source baseline: `7288d9b199e1b09b529fbaae38303f4c82f38612`.

## Delivery sequence

Confirm goal/prohibited changes → inspect existing implementation → plan files/risks → implement a small vertical slice → targeted regression → full CNB gates → review package → separately authorized merge/publication.

Use real issue references only. Do not mix features, refactors, and dependency upgrades. Serialize changes within the same conflict domain. Record out-of-scope findings without opportunistically implementing them.

## Planning and review

A task plan records background, source baseline, allowed/prohibited files, compatibility/data/protocol risks, success/failure acceptance, commands, lockfile differences, and unresolved work. Prefer paths and symbols over drifting line numbers.

Branches use `fix|refactor|docs|chore/<issue-or-scope>` from main. Do not push directly to main or force-push shared history. The existing project convention uses Conventional Commits with Chinese descriptions. Include `Ref` only for a genuine issue.

Review scope, contracts, permissions, state/migrations, test authenticity, and evidence. Fixes must not weaken assertions, suppress missing-plugin warnings, or bypass security boundaries. Stop after three unsuccessful attempts at the same error.

## Canonical gates

```bash
bash scripts/ci-local.sh --help
bash scripts/ci-local.sh quick
bash scripts/ci-local.sh --keep-going all
```

Preserve the exact Rust toolchain, `--locked`, feature subsets, npm lockfile, coverage, and bundle limits. `all` includes real-SQLite Chromium E2E, but not performance/soak, cargo-audit, full release architectures, or real-system/multi-node acceptance.

## CNB development protocol

Use `scripts/cnb-workspace.sh` with the token only in authorized process memory/environment; never put it in files, Git URLs, or logs.

1. Start or reuse a ready workspace. Validate the `cnb.space` RSA host key against the pinned official SHA-256 fingerprint before SSH, using only RSA/SHA-2 signatures.
2. Use private temporary known-hosts files. Fail on unknown hosts/keys, empty scans, or fingerprint mismatch; do not fall back to TOFU, `accept-new`, SHA-1, or disabled strict checking.
3. Synchronize the **final tree**, including uncommitted files, into an owned isolated directory. Never overwrite `/workspace`, which may contain someone else's restored work.
4. Maintain a foreground SSH connection or `hold` for long jobs. A detached process alone does not keep the workspace alive. Do not permanently increase idle timeouts.
5. Save revision/dirty state, source digest, Rust/Node/npm versions, CPU/memory, step results/times, full logs, and reports.
6. Download evidence before stopping. Confirm stop success; a directory or started process is not completed validation.

Official fingerprint source: https://docs.cnb.cool/zh/workspaces/fingerprint.html. Rotation requires independent verification/review, not an environment bypass. Offline SSH-policy regression: `python3 scripts/tests/test_cnb_workspace.py`.

## Quota and external actions

CNB development and ordinary build pipelines use different quotas. Lightweight builds still cost quota. When zero ordinary builds are requested, do not validate through build-start, push, PR, or tags. Push CI-skip does not suppress PR events. GitHub configuration does not establish that Actions ran.

Push/PR/tag/release are external actions, not prerequisites for testing. Never claim a merge/publication without evidence. Keep credentials, private keys, dependencies, and generated build directories out of commits.

## Synchronization and release

API changes update annotations, permissions, snapshot, types, and API reference. Table changes update migration tests/database notes; deployment changes update templates/operator docs. Record history in CHANGELOG, current direction in Doc/30, and historical execution in Doc/32.

Release requires separate authorization and evidence. Toolchain upgrades are separate slices covering manifests, Docker/NPC/CI, locks where necessary, feature/release targets, and actual validation. Read the task-specific skills for module/migration/endpoint/frontend/app-package procedures.
