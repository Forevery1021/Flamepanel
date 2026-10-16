# 07 · JWT authentication and RBAC

> Condensed English translation. [Complete Chinese original](../../Doc/07-权限体系设计.md) is retained unchanged. Detailed tables, examples, and historical evidence omitted here still require unabridged translation. Source baseline: `7288d9b199e1b09b529fbaae38303f4c82f38612`.

## Identity and authorization

The panel shares JWT verification, user/role services, and cache invalidation. Login obtains access/refresh credentials; sensitive workflows include initial-password changes, TOTP, revocation, and lockout. Use transport headers for REST and first-frame authentication for WebSockets; never put tokens into URLs.

RBAC associates users with roles and roles with resource/action permissions. Permission names and route behavior are defined by source, not the historical permission count in older documents.

## Route rules

`flame-kernel/src/api/permissions.rs` is authoritative. First matching rule wins. Protected routes without a known mapping are denied; explicitly public endpoints are a separate allowlist. Keep authentication and RBAC in the shared middleware rather than recreating them in handlers.

IaC export combines read permissions for the selected resources and applies a field allowlist. Export permission is not import/deploy permission. Test partial permissions and secret removal rather than relying on an administrator-only happy path.

## Storage and frontend

RBAC seed data must reach the actual backend: memory tests alone do not prove SQLite startup creates the same usable roles/permissions. Regress old-database initialization and idempotent seeding.

Frontend permission-based visibility improves usability but does not replace server enforcement. Handle 403 using shared feedback, and keep role-management UI and backend identifiers consistent.

## Adding a permission

1. Identify the resource/action and existing callers.
2. Update permission seeds and route mappings without widening existing access.
3. Annotate/register OpenAPI and regenerate contracts.
4. Add allowed, unauthenticated, forbidden, revoked, and unknown-route tests.
5. Update locale/UI behavior and relevant documentation.

Keep token/session/2FA format changes separate from comment cleanup. The full Chinese edition contains login diagrams, route examples, permission tables, and database details; this guide does not reproduce those tables.
