<div align="center">

# 🔥 FlamePanel

**An open-source, self-hosted Linux server management panel**

Rust kernel · Vue 3 · Static frontend + SQLite · MIT

[![Rust](https://img.shields.io/badge/Rust-1.98.1-orange)](https://www.rust-lang.org/)
[![Axum](https://img.shields.io/badge/Axum-0.8-red)](https://docs.rs/axum)
[![Vue](https://img.shields.io/badge/Vue-3.5-brightgreen)](https://vuejs.org/)
[![License](https://img.shields.io/badge/license-MIT-blue)](./LICENSE)

**English** | [简体中文](./README.zh-CN.md)

[Quick start](#quick-start) · [Features](#features) · [Supported systems](#supported-systems) · [Documentation](#documentation) · [Contributing](#contributing)

</div>

---

## What is FlamePanel?

FlamePanel is a browser-based alternative to panels such as 1Panel and aaPanel. Manage websites, containers, databases, certificates, backups, and firewalls on your own servers.

- **Lightweight:** a Rust kernel, static frontend, and SQLite, with no external database requirement. Resource targets and measured acceptance results are recorded separately.
- **No lock-in:** the panel is a management plane. **Your sites and containers keep running when the panel stops or is removed** (`uninstall.sh --keep-apps`).
- **Native privilege separation:** the panel runs as a non-root user with systemd sandboxing. A separate local privileged helper performs allowlisted web-engine operations. RBAC, audit logs, and login lockout are built in.

## Quick start

### Native installation

Debian 12+ and Ubuntu 22.04+ are recommended. RHEL 9-family systems have limitations; see [Supported systems](#supported-systems).

```bash
git clone https://github.com/Forevery1021/Flamepanel.git
cd Flamepanel
sudo bash install.sh
# -n: non-interactive defaults; -t: self-signed HTTPS, redirecting port 80 to 443
```

Open `http://<server-ip>` and complete the six-step administrator setup wizard. Allow ports **80/443 in your cloud security group**.

> The installer downloads versioned GitHub Release assets. Source migration or a successful CI run does not itself create a published release. Confirm that the required assets exist before installation.

<details>
<summary><b>What does the installer change?</b></summary>

- Installs distribution-specific `curl`, `openssl`, and `nginx` packages; creates the `flamepanel` system user and installs to `/opt/flamepanel`.
- Configures nginx for static assets and `/api` and `/ws` proxying, security headers/CSP, and a 2 GB upload limit. The backend listens on `127.0.0.1` only.
- Handles Debian's default site, nginx layout differences on Alpine/Arch, and SELinux/firewalld on RHEL-family systems.
- Records system changes in `/opt/flamepanel/.install-state` so `uninstall.sh` can roll them back precisely.

See the [deployment guide](./docs/en/10-deployment.md) and the [complete Chinese reference](./Doc/10-部署运维指南.md).
</details>

### Docker

Docker Compose is available across distributions, including Alpine:

```bash
umask 077
echo "OP_JWT_SECRET=$(openssl rand -hex 32)" > .env
docker compose up -d
# http://<server-ip>:8080; docker.sock is not mounted by default
```

**Uninstall:** `sudo bash uninstall.sh` preserves data; `--keep-apps` keeps managed applications running; `-p` purges data.

## Features

| Area | Capabilities |
| --- | --- |
| Monitoring | Live CPU, memory, disk, network, and process metrics; historical charts; Prometheus `/metrics` |
| Docker | Containers, images, networks, volumes, and Compose; convert `docker run` to Compose and adopt existing containers |
| Websites | Native Nginx, Apache, OpenLiteSpeed, OpenResty, and Caddy installation/switching; rewrite presets for WordPress, Laravel, and SPAs |
| WAF | Rules across five engines: IP/UA allowlists and denylists, blocked paths, SQL injection/XSS patterns; rate limiting on Nginx/OpenResty; hit logs in the panel |
| Certificates | ACME HTTP-01/DNS-01 with Cloudflare, Alibaba Cloud, Tencent Cloud, and Huawei Cloud; self-signed/uploaded certificates and renewal |
| Databases | MySQL, MariaDB, PostgreSQL, and Redis installation and database/user/configuration management |
| App store | 1Panel, aaPanel, and x-flame package formats; remote sources, Ed25519 signatures, and structured pre-install security scanning |
| Project deployment | PHP, Node, Python, Go, and Java runtimes; push-to-deploy, Dockerfile builds, and health-checked zero-downtime switching |
| Backups | Panel snapshots and scheduled application backups to S3, SFTP, or local storage; AES-256-GCM credential encryption |
| Files and terminal | Sandboxed uploads/downloads, editing, archives, recycle bin, and audited xterm.js sessions |
| Firewall | Unified ufw, firewalld, and iptables management |
| Multiple nodes | Lightweight Rust agent with bootstrap registration, heartbeats, and allowlisted commands |
| Automation | JSON-oriented `fpctl`, MCP with human approvals, and an optional assistant disabled by default |
| Configuration export | Read-only IaC YAML with resource permissions and field allowlists; **no import/deploy support** |
| Interface | Command palette (⌘K), onboarding, OKLCH themes, three material levels, and Chinese/English/Japanese locales |

## Supported systems

| Tier | Distributions | Notes |
| --- | --- | --- |
| Tier 1 | Debian 12/13; Ubuntu 22.04/24.04 | Primary support and regression targets; consult actual acceptance evidence |
| Tier 2 | Rocky, Alma, RHEL, Oracle Linux 9; CentOS Stream 9; Fedora; openEuler, Anolis, OpenCloudOS, UOS, and related systems | Panel/core functionality; known native MySQL/PostgreSQL limitations |
| Tier 3 | openSUSE; Arch/Manjaro | Panel can run; some native software requires manual setup |
| Docker only | Alpine (no systemd by default) | Use Docker Compose |

Native installation requires systemd. Release configuration targets amd64/arm64 with GNU and musl variants. Configured build targets are **not proof of completed cross-platform testing**. See [security and compatibility](./docs/en/12-security-compatibility.md) and [Chinese distribution notes](./Doc/33-代码审计与优化报告.md#4-发行版适配与支持程度).

## Architecture

```text
Vue + Pinia + VueQuery → /api, /ws → Axum handlers → Services → Domain ports
                                       │                          ↑
                               JWT / RBAC / audit         SQLite / Docker / I/O
                                       ├─ Local privileged helper (allowlisted)
                                       └─ Lightweight agent (HTTP / optional outbound Edge)
```

Composition root: `flame-kernel/src/lib.rs::build_services`. Backend selection: `main.rs` (SQLite by default). Permissions: `api/permissions.rs`. Live sampling: `MetricsHub`. Frontend bootstrap: `frontend/src/main.ts`.

| Directory | Responsibility |
| --- | --- |
| `flame-kernel/` | Domain entities/ports, services, API, adapters, and task lifecycle |
| `frontend/` | Page composition, business composables, Fp components, and generated API types |
| `agent/` | Modular node process with independent command, file, and token security boundaries |
| `scripts/` | Quality gates, toolchain checks, offline installer tests, and CNB workspace helpers |

The native non-root boundary does **not** automatically apply to Docker deployments. See [architecture](./docs/en/01-architecture.md) for startup, boundary debt, and features, and [cleanup evidence](./docs/en/35-engineering-cleanup.md) before removing apparent dead code.

## Documentation

Start with the [English documentation index](./docs/en/README.md). It explicitly distinguishes translated editions from condensed guides and work still pending. All original [Chinese documentation](./Doc/README.md) is retained.

| Task | Guide |
| --- | --- |
| Install, upgrade, back up, uninstall | [Deployment](./docs/en/10-deployment.md), [troubleshooting](./docs/en/13-troubleshooting.md) |
| Review security and compatibility | [Security](./docs/en/12-security-compatibility.md), [audit](./docs/en/33-code-audit.md) |
| Understand the code | [Architecture](./docs/en/01-architecture.md), [API](./docs/en/02-api.md), [database](./docs/en/03-database.md), [permissions](./docs/en/07-permissions.md) |
| Develop and test | [Backend](./docs/en/05-backend.md), [frontend](./docs/en/04-frontend.md), [testing](./docs/en/06-testing.md), [workflow](./docs/en/31-development-workflow.md) |
| Build app packages | [App store](./docs/en/20-app-store.md), [SDK](./docs/en/21-app-sdk.md), [compatibility](./docs/en/22-app-compatibility.md) |
| Work with nodes/events/AI | [Agent](./docs/en/23-agent-protocol.md), [events](./docs/en/24-events.md), [MCP](./docs/en/34-mcp.md) |
| Review maintenance and recovery | [Roadmap](./docs/en/30-roadmap.md), [recovery evidence](./docs/en/41-recovery-preflight.md) |
| Migrate and build binaries | [GitHub migration and releases](./docs/en/github-migration.md) |

Release history: [CHANGELOG.md (Chinese)](./CHANGELOG.md).

## Contributing

Use Rust **1.98.1** from `rust-toolchain.toml`, Node from `.node-version`, and npm from `frontend/package.json`. Do not switch to pnpm or upgrade lockfiles as part of cleanup.

```bash
# Terminal 1, repository root: SQLite by default
cargo run --locked -p flame-kernel --bin flame-kernel
# Terminal 2
cd frontend && npm ci && npm run dev
```

Run complete validation in the **CNB cloud-native development workspace**:

```bash
bash scripts/ci-local.sh --keep-going all
```

The scripts/configuration define gate groups and thresholds. `all` does not replace performance testing, Rust dependency auditing, cross-architecture release builds, or real-system acceptance. When ordinary CNB build usage is prohibited, do not test through push, PR, tag, or ordinary build-start events.

Read [AGENTS.md](./AGENTS.md) and the [development workflow](./docs/en/31-development-workflow.md). Feature work, runtime deletion/refactoring, merge, and publication require their own scope decisions.

## License

[MIT](./LICENSE)
