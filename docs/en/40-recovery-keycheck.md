# 40 · Explicitly authorized encryption-key matching

> Condensed English translation. [Complete Chinese original](../../Doc/40-恢复主密钥匹配验证.md) is retained unchanged. Detailed tables, examples, and historical evidence omitted here still require unabridged translation. Source baseline: `7288d9b199e1b09b529fbaae38303f4c82f38612`.

## Separate from metadata inventory

`keycheck.py` and the internal `fp-restore-keycheck` worker verify a staged master key against supported ciphertext without returning plaintext. Inventory defaults remain unchanged. No production API/UI, kernel startup, migration, task execution, network client, or key generation/rotation is added.

```bash
cargo build --locked -p flame-kernel --no-default-features --features sqlite --bin fp-restore-keycheck
python3 scripts/restore-drill/keycheck.py   --backup /secure/staging/snapshot.db   --assets-root /secure/staging/assets   --key-path secret_encryption.key   --keycheck-bin "$PWD/target/debug/fp-restore-keycheck"   --allow-secret-read   --out /secure/reports/new-unique-keycheck
```

Without `--allow-secret-read`, reject before acquiring the snapshot or reading the key. Never put key values into argv, environment, URLs, manifests, or history. Use private staged roots, canonical relative paths, no-follow directory descriptors, single-link regular key files, owner-only permissions, and a new output directory.

Decode the persisted master as UTF-8, apply the existing Rust trim behavior, and use the remaining text bytes as the original `SecretCipher` does. Do **not** hex/base64-decode it into a different key; base64 in the internal pipe is transport encoding only.

## Fixed scope and budgets

Check only backup-account config JSON, DNS-account credentials JSON, TOTP secret/pending_secret, and assistant API-key settings. Legacy plaintext is only counted and is not matching evidence. Only `enc:v1:` is eligible; unknown versions, missing/non-TEXT/generated/hidden fields, active schema, and incomplete coverage cannot be silently skipped.

Defaults: 512 MiB snapshot and 120-second deadline. Caps: 4 KiB key, 1024 ciphertexts, 64 KiB per item, 1 MiB total ciphertext, 2 MiB worker request, 4 KiB response. Perform coverage/budget preflight before secret reading. No “first N items” claim of full matching is permitted.

## Secret/process boundary

Only after authorization and complete supported scanning with ciphertext present is the staged key read. Send it through anonymous stdin to a trusted worker in a private cwd/cleared environment. The worker reuses existing AEAD validation and discards decrypted values, returning aggregate authentication counts only. It receives no file paths, SQL, database URL, or panel configuration.

Validate exact protocol fields/counts and reject duplicate keys, contradictions, excess output, failures, and unexpected stderr disclosure. Deadlines/interruption reclaim the owned process group/copies; SIGKILL/host failure may leave remnants. No OS sandbox, RAM locking, zeroization, swap protection, or secure-erasure claim is made.

## Results

| Status | Meaning | Exit |
| --- | --- | --- |
| MATCHED_WITHIN_SCOPE | Complete fixed scan, at least one supported ciphertext, all AEAD checks pass | 0 |
| NOT_VERIFIED | Authentication/decoding fails; wrong key and corruption may be indistinguishable | 1 |
| INCONCLUSIVE | No supported ciphertext, gaps, unknown version, or budget limit; key not read | 1 |
| BLOCKED | No permission, unsafe input, worker/snapshot/deadline/cleanup failure | 1 |
| Unavailable report | Arguments/output failed | 2 |

A scoped match does not prove key origin, all secrets, credential validity, JWT continuity, business recovery, or production RTO/RPO. `restore_ready`, `production_auth_verified`, and `kernel_started` remain false. Reports omit key lengths/fingerprints/contents, ciphertext/plaintext, paths, and raw exceptions.

Tests use synthetic material and the real `SecretCipher`/worker; Python orchestration doubles do not replace real cryptographic integration. See the Chinese edition for every limit, exit boundary, and test command. New release packaging includes this tool; older source notes predate that change.
