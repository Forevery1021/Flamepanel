# 37 · Isolated SQLite recovery drill

> Condensed English translation. [Complete Chinese original](../../Doc/37-SQLite隔离恢复演练.md) is retained unchanged. Detailed tables, examples, and historical evidence omitted here still require unabridged translation. Source baseline: `7288d9b199e1b09b529fbaae38303f4c82f38612`.

## What PASS means

PASS covers snapshot acquisition, integrity/minimum-schema checks, isolated copying, read-only reopening, and fixed-table sampling. It does **not** mean the panel, external services, production authentication, or RTO/RPO have recovered.

Default mode runs no production kernel, migrations, tasks, Agent, SMTP, or external database. Use Linux/Python 3.11+ and trusted standalone snapshots; SQLite files contain sensitive hashes/ciphertext.

```bash
python3 scripts/restore-drill/drill.py   --backup /secure/export/flamepanel-snapshot.db   --out /secure/reports/drill-unique-run
```

The output parent must exist and the output directory must not. `--sha256` checks a digest obtained through a trusted channel; an internally computed digest alone does not authenticate origin. `--backup-time` records declared backup age, not RPO. Default size/time bounds are 512 MiB/120 seconds; check `--help` and the Chinese reference for all limits.

Optional HTTPS acquisition validates certificates, rejects redirects and embedded credentials, and does not use environment proxies. It is not S3/SFTP credential handling. Prefer an existing authorized downloader; never place secrets in a URL or command history.

## Safety

Reject symlinks/nonregular/live SQLite inputs with WAL/SHM/journal companions. Copy through private random directories; do not give the original to SQLite. Use read-only/query-only connections with trusted schema disabled and no extensions or task execution.

Directories are 0700, reports/copies 0600. Refuse output overwrite. Check size/capacity and handle real ENOSPC despite prechecks. Normal/caught failures and interruption clean up copies; SIGKILL/host failure can leave private remnants and do not guarantee secure erasure.

Reports omit paths, complete URLs, credentials, row values, and raw exceptions; metadata remains sensitive.

## Stages and results

`acquire → validate → capacity → restore → reopen → sample`. Sample only fixed columns in users/permissions/panel_settings, at most 20 rows per table. A failed stage makes later stages NOT_RUN and the overall result FAIL.

`storage_rto_seconds` is storage-copy/reopen/sample time, **not production RTO**. Production RTO and RPO stay null. Backup age and file mtime do not prove a data-loss window. Encryption keys, website files, volumes, and external databases are outside this snapshot.

Exit 0: scoped offline PASS; 1: drill failure; 2: arguments/output failure. Use a fresh output directory per run.

## Probe and tests

Only explicit `--probe-bin` enables the additional dedicated probe on another copy; see [Doc/38](./38-recovery-probe.md). Never substitute the normal kernel.

```bash
python3 -m unittest discover -s scripts/restore-drill -v
cargo test --locked -p flame-kernel --no-default-features --features sqlite --test restore_drill
```

Run in CNB's isolated development directory and then run full gates. Fault injection covers space/network/interruption without filling the host disk. Optional evidence variables export synthetic material only and must not persist into repeated full runs.
