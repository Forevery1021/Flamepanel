# 10 · Deployment and operations

> Condensed English translation. [Complete Chinese original](../../Doc/10-部署运维指南.md) is retained unchanged. Detailed tables, examples, and historical evidence omitted here still require unabridged translation. Source baseline: `7288d9b199e1b09b529fbaae38303f4c82f38612`.

## Choose a deployment mode

Native installation is recommended for supported systemd-based servers. Docker Compose is appropriate where native systemd installation is unavailable, including Alpine. Docker privileges/mounts differ from the native non-root boundary.

```bash
git clone https://github.com/Forevery1021/Flamepanel.git
cd Flamepanel
sudo bash install.sh
# -n: unattended defaults; -t: self-signed HTTPS
```

The installer obtains versioned GitHub Release archives and verifies SHA-256 before unpacking. Check that published assets exist; pushing source alone does not publish binaries. A checksum hosted alongside an artifact detects corruption but is not an independent release signature.

Open the server address, complete setup, and allow 80/443 in the provider security group. Native installation configures nginx, a non-root API service, and a separate allowlisted helper. Preserve the install-state record for precise uninstall rollback.

## Docker

```bash
umask 077
echo "OP_JWT_SECRET=$(openssl rand -hex 32)" > .env
docker compose up -d
```

The default Compose setup does not mount the Docker socket. Do not silently add host access to make container management work. Review volumes, service UID, secrets, TLS, and host capabilities first.

## Configuration and transport

Consult `config/loader.rs`, deployment templates, and the complete Chinese configuration table for supported `OP_*` variables and TOML keys. Persisted key-loading rules can affect the effective value; do not assume changing an environment variable rotates a persisted secret.

Custom reverse proxies must preserve `/api` and `/ws`, WebSocket upgrades, upload limits, and security headers. Retain CSP, `nosniff`, frame protection, and referrer policy. Never weaken CSP with broad `unsafe-inline`/`unsafe-eval` to mask a frontend issue.

## Daily operations

Use systemd status/journal and `fpctl` for health and logs; correlate failures with request IDs. Before an upgrade, preserve both a consistent SQLite snapshot and the independent assets/keys/configuration required by the deployment. Separate executable rollback from database/file/volume recovery.

The privileged helper is a local allowlisted service, not unrestricted panel sudo. Deploy agents with independent authentication/bootstrap credentials and reviewed file/terminal boundaries.

## Uninstall and acceptance

`sudo bash uninstall.sh` preserves data; `--keep-apps` keeps managed applications; `-p` purges. Review flags before execution. Never use production installation/uninstallation as an offline regression test.

Before production: verify non-root service identity, HTTPS/security headers, strong administrator credentials, RBAC denials, WS authentication, firewall/provider rules, consistent backup, independently secured keys, successful isolated recovery, and actual target-system behavior.

The Chinese source retains manual systemd/nginx examples, the full environment table, upgrade/rollback commands, AI-client setup, and topology diagrams. See [migration/releases](./github-migration.md) for the new release workflow.
