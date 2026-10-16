# 02 · REST and WebSocket API

> Condensed English translation. [Complete Chinese original](../../Doc/02-API接口文档.md) is retained unchanged. Detailed tables, examples, and historical evidence omitted here still require unabridged translation. Source baseline: `7288d9b199e1b09b529fbaae38303f4c82f38612`.

## Authoritative contracts

Use [`frontend/openapi.json`](../../frontend/openapi.json), `GET /api/openapi.json`, and the [generated API reference](../../Doc/api/README.md). The panel also exposes an API documentation page at `/api-docs`. This condensed edition does not duplicate the Chinese endpoint examples or the generated schema catalog; their prose translation remains pending.

## Authentication and errors

Authenticate with `POST /api/auth/login`; supply the resulting access token in `Authorization: Bearer <token>`. Two-factor login, refresh, current-user, password-change, and 2FA management routes have separate contracts in the specification. Local `fpctl api <METHOD> <PATH>` signs a short-lived administrator token and still passes through authorization and audit.

Non-2xx errors use `code`, `error`, and `message`. Use `x-request-id` to correlate server logs. Typical statuses are 400 invalid input, 401 missing/expired identity, 403 denied or initial-password change required, 404 missing resource, 409 conflict, 422 validation failure, and 429 rate limiting. Do not infer success from an HTTP connection alone.

Timestamps use RFC 3339 UTC. Pagination uses `page`/`page_size` where declared; consult each endpoint rather than assuming a universal response shape. Field selection is endpoint-specific.

## Resource families

The API covers authentication, setup, users, nodes, websites, Docker, plugins, web engines, settings, databases, app store, files, firewall, audit/system/kernel logs, backups/accounts/jobs, runtimes/projects, scheduled tasks, memos, metrics/overview/inspection/toolbox, tasks, outbox, certificates, DNS providers, WAF, MCP, assistant, and IaC.

IaC export is read-only, permission-filtered, and field-allowlisted; it does not implement import or deployment. Long operations use the task center; request completion and task completion are different states.

## WebSocket boundary

Metrics, logs, and terminal streams authenticate with an initial `{"type":"auth","token":"<access_token>"}` frame plus channel permissions. Never put the token in a query string. Handle expiry, revocation, renewal, slow consumers, and resynchronization. Agent Edge is a separate node protocol, not browser authorization.

## Maintaining contracts

Change handler annotations and permission mappings together. Register paths in `flame-kernel/src/openapi.rs`, regenerate the snapshot/types/reference, and test 401/403 and failure responses. Do not hand-edit generated API documents. Dedicated recovery-probe `/health` and `/summary` are **not production API endpoints**.
