# 39 · Recovery assets and external key inventory

> Condensed English translation. [Complete Chinese original](../../Doc/39-恢复资产与外部密钥清单.md) is retained unchanged. Detailed tables, examples, and historical evidence omitted here still require unabridged translation. Source baseline: `7288d9b199e1b09b529fbaae38303f4c82f38612`.

## Inventory, not readiness

A SQLite snapshot is not a full recovery package. `inventory.py` reads declared asset **metadata**, not secret contents, and does not back up, restore, generate/rotate keys, start services, or verify production identity.

Default encryption/JWT/bootstrap paths depend on **CWD/data**. Final ACME/certificate paths depend on the **actual database parent**. Check WorkingDirectory, EnvironmentFile, overrides, and historical versions; do not assume all keys are beside SQLite.

| Material | Separate boundary |
| --- | --- |
| SQLite snapshot | Consistent exported state, not a raw live-file copy |
| Encryption master | `data/secret_encryption.key`; not the JWT key; replacement cannot decrypt old data |
| JWT signing material | Persisted loading can override TOML/OP_JWT_SECRET |
| Bootstrap and agent tokens | Registration token differs from per-agent AUTH_TOKEN |
| ACME/certificate keys | Independent files/account continuity; certificate rows do not contain all files |
| Ingress TLS/configuration | May exist outside the certificate table |
| Website/project/app files, volumes | Database counts do not establish content completeness |
| External databases/agent configuration | Not replaced by the panel database |

## Inputs

```bash
python3 scripts/restore-drill/inventory.py   --backup /secure/staging/snapshot.db   --assets-root /secure/staging/assets   --manifest /secure/staging/recovery-assets.json   --out ./new-unique-inventory-report
```

Use a private, owner-readable/traversable, non-symlink staging root (normally 0700), not a production directory. The manifest is bounded to 64 KiB and only allows schema/jwt_strategy/assets with role/path entries. Reject duplicate roles/paths, embedded secrets, absolute/traversal/control-character paths, symlinks at every component, unsafe/nonregular files, hard links, devices, and FIFOs.

Sensitive files normally use 0600/0400. Output must not exist; directory/report modes are 0700/0600. No recursion, network, archive extraction, or reading host paths from database values. A directory's metadata check does not validate its contents, ACLs, service UID, or snapshot consistency.

## Required roles and scope

Deployment configuration is always required. Observed encrypted fields/coverage gaps require encryption material; preserve strategy requires JWT signing material. Nodes, certificates, websites/projects/apps, and external databases conservatively require corresponding declared material. Every declared missing asset blocks, including optional items left in an unedited example manifest.

Choose `jwt_strategy=preserve` or `reissue` explicitly. Reissue records a plan only; it does not generate keys, revoke tokens, or modify sessions.

The fixed encrypted-field inventory covers backup account JSON, DNS account JSON, TOTP secret/pending_secret, and assistant API key settings. Non-TEXT/missing fields and unknown enc versions are coverage gaps, not proof of no secrets. Only aggregate resource counts/source categories are returned.

## Results

`BLOCKED`/exit 1 means unsafe/missing/failed material. `REVIEW_REQUIRED`/exit 0 means metadata has no blocking issue, **not PASS or startup permission**. Exit 2 is argument/output failure.

`restore_ready`, `key_match_verified`, `production_auth_verified`, and `secret_contents_read` stay false. Secret assets are opened/fstat'ed, not read/decoded/hashed/fingerprinted/copied. A wrong but nonempty key can be PRESENT_UNVERIFIED; an empty directory cannot prove backup completeness.

Use [explicit keycheck](./40-recovery-keycheck.md) only with separate authorization. Full production/external recovery and RTO/RPO remain pending. The Chinese source retains exact role-policy and code-provenance matrices.
