# 04 · Frontend development

> Condensed English translation. [Complete Chinese original](../../Doc/04-前端开发指南.md) is retained unchanged. Detailed tables, examples, and historical evidence omitted here still require unabridged translation. Source baseline: `7288d9b199e1b09b529fbaae38303f4c82f38612`.

## Toolchain and startup

The frontend uses Vue 3, TypeScript, OpenVue, Pinia, VueQuery, and vue-i18n. `.node-version` and `frontend/package.json::packageManager` pin Node/npm; install using `npm ci` and preserve `package-lock.json`.

```bash
cd frontend
npm ci
npm run dev
```

Build configuration is native ESM. Use the existing Vite proxy for `/api` and `/ws`; do not hard-code browser-side localhost endpoints when deploying behind a reverse proxy.

## Components and data flow

Business pages use the `Fp*` wrapper layer rather than directly spreading third-party widgets through every view. Pages assemble; composables/utils contain reusable, testable logic. Follow existing API modules and generated types. Normalize errors through the shared client; use VueQuery for server state and Pinia for application state.

Route metadata drives menus, breadcrumbs, and tabs. Keep permissions consistent with the backend and do not treat hidden buttons as authorization.

## Design and failure states

Use `--fp-*` design tokens, the Flame preset, and the configured material/density system. Preserve keyboard access, focus, semantic controls, accessible labels, and readable contrast. Lists need consistent search/filter/loading/empty/error states; asynchronous UI must account for retry and cancellation.

App-level error boundaries handle fatal layout/router failures. Page-level boundaries preserve navigation after a view fails. Never expose raw translation keys when a locale-loading race occurs.

## Internationalization and tests

Maintain Chinese, English, and Japanese keys and placeholder parity. Do not translate executable strings, API identifiers, or locale key names as part of comment cleanup. Avoid `as any`, ignored types, and nonsemantic E2E selectors.

Each component mount gets a new Pinia, required production plugins, and an explicit unmount. Do not suppress console warnings to hide missing dependencies.

Use the repository's typecheck, lint, locale, coverage, build, and bundle commands via `scripts/ci-local.sh`. Thresholds come from Vitest/bundle configuration, not historical documentation. Component mapping tables, full page templates, and the detailed modernization history remain in the complete Chinese guide and `frontend/design.md`.
