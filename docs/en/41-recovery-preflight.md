# 41 · Recovery evidence preflight and next acceptance stages

> Condensed English translation. [Complete Chinese original](../../Doc/41-恢复证据预检与后续验收路线.md) is retained unchanged. Detailed tables, examples, and historical evidence omitted here still require unabridged translation. Source baseline: `7288d9b199e1b09b529fbaae38303f4c82f38612`.

## Correlate evidence, not production authority

A drill PASS, successful probe, asset REVIEW_REQUIRED, and scoped key match cannot simply be combined into “production recovery succeeded.” R-01A/B/C/D establish different limited facts. E-1 checks their structure/common snapshot/count/JWT-plan consistency only.

Input reports are unsigned and editable. A forged but structurally consistent report may pass. This tool does not independently verify producer identity, source digest, authenticity, freshness, or production state.

```bash
python3 scripts/restore-drill/preflight.py   --evidence-root /secure/staging/evidence   --drill-report drill/report.json   --inventory-report inventory/inventory.json   --keycheck-report keycheck/keycheck.json   --jwt-strategy preserve   --out /secure/reports/new-unique-preflight
```

Use Linux/Python 3.11+, a private non-symlink root, private single-link regular reports, canonical relative paths, and a nonexistent output directory. Up to three inputs, 128 KiB each; default deadline 30 seconds, maximum 300. Reject links, traversal, repeated paths/JSON keys, unknown schema/fields, NaN/Infinity, boolean counts, negative/contradictory values, and readiness claims.

No database/asset/binary/secret input exists. No recursion, network, subprocess, configuration read, migration/seed, signing, revocation, or key generation occurs. Input bytes remain unchanged. Do not disguise a secret file as report JSON.

## Consistency requirements

1. A dedicated-probe drill has seven passing stages, migration/HTTP/auth/write-denial/normal-stop flags, and no production kernel.
2. Inventory is REVIEW_REQUIRED with complete known scan/resource coverage; required assets are recomputed from observations, not blindly trusted flags.
3. All participating source SHA-256 digests and byte counts agree.
4. When encrypted references exist, key evidence must be MATCHED_WITHIN_SCOPE with matching fixed-field counts, all authenticated, no rejects/unknown versions/gaps.
5. Without known encrypted references, do not fabricate a match; optional key evidence must instead show the same-source zero-reference/no-secret-read case.
6. Preserve/reissue strategy agrees across inputs, but remains only a plan, not session modification.
7. Post-migration/seed probe counts are not forced to equal pre-migration observations.

Incomplete historical schemas block rather than being automatically repaired/mixed with a different snapshot.

## Output

`preflight.json` uses `flamepanel/recovery-preflight/v1`, with a Markdown companion. REVIEW_REQUIRED/exit 0 means consistency within this contract; BLOCKED/exit 1 means missing/unsafe/contradictory evidence; exit 2 means argument/output failure.

Always false: restore readiness, production auth, tool-performed key verification, kernel startup, session modification, key rotation, database opening, asset-secret opening, report authenticity, and freshness. Production RTO/RPO remain unknown. A SCOPED_MATCH_REPORTED check is a producer statement, not local decryption.

## Next stages require separate approval

- **E-2A:** identity-loading ADR/synthetic contracts, including persisted JWT precedence and correct/wrong/expired/revoked/forbidden identities.
- **E-2B:** separately authorized operator material; preserve proves effective identity continuity, reissue requires an explicit revocation/reauthentication decision.
- **E-3:** one actual business read with real permission checks and denial, not an omnipotent admin token.
- **E-4:** startup-assembly and OS-isolation ADR before experimenting. Disable capabilities before assembly; no fallback to ordinary kernel startup when isolation is unavailable.
- **R-01F:** external files/volumes/databases/ACME/TLS/agent material with UID/ACL/mount/timing mappings and real business-ready RTO/RPO definitions.

Review migration/seed, WAF reload, schedulers, backups, renewal, cleanup, node scans, outbox, notifications, and task restoration individually. Do not add privileged containers/host mounts/network exceptions without authorization.

R-02 first inventories current deployment types/locks/cancellation, then chooses one failure-protection slice. R-03 starts with one request/task/log/notification failure chain. C02–C06 cleanup candidates remain pending compatibility decisions. Other node/security/IaC proposals do not become automatic work.

## Validation

```bash
python3 -m unittest discover -s scripts/restore-drill -v
cargo test --locked -p flame-kernel --no-default-features --features sqlite --test restore_preflight
bash scripts/ci-local.sh --keep-going all
```

Synthetic end-to-end evidence must still report production identity as unverified. Use fresh evidence directories; do not retain export variables for repeated full gates. Full Chinese tables retain precise prerequisites/stop conditions for each future slice.
