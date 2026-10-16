# 21 · App package and plugin SDK

> Condensed English translation. [Complete Chinese original](../../Doc/21-应用商店SDK开发指南.md) is retained unchanged. Detailed tables, examples, and historical evidence omitted here still require unabridged translation. Source baseline: `7288d9b199e1b09b529fbaae38303f4c82f38612`.

## Package lifecycle

Packages are recognized, imported, inspected, installed, and upgraded through existing adapters. Choose the supported ecosystem/installation mode deliberately; do not assume 1Panel/aaPanel hooks or native installers are interchangeable.

The complete Chinese source defines directory layouts, `app.json`, form fields, Compose substitutions, x-flame extensions, and concrete examples. Preserve machine-readable field names and validate against actual parser code instead of translating schema identifiers.

## Authoring rules

- Declare metadata, supported modes, version, dependencies, and typed input constraints.
- Keep Compose variable mapping explicit and avoid exposing secrets in logs.
- Follow network conventions and test conflicts, missing dependencies, and existing resources.
- Pass structural/security scanning before installation; do not use unsupported shell workarounds.
- Treat source/signature policy separately from package parsing and schema validity.

## Native and WASM integration

Native installation uses registered lifecycle ports and existing command policies. A generic install script does not grant unrestricted host access.

WASM plugins must use supported export signatures, lifecycle hooks, and host capabilities. Observe sandbox memory/fuel/stack and integrity limits. The full guide contains Rust/WAT examples, host API details, plugin-market scope, and API integration examples; those examples remain pending unabridged translation.

## Validation

Test the existing parser, schema, form substitutions, security scanning, successful install, rejection paths, upgrades, and cleanup with isolated fixtures. Integrate through documented authenticated API contracts, preserving permissions, task semantics, and error envelopes. Do not weaken scanner rules just to accept a package.

See [app store](./20-app-store.md), [ecosystem compatibility](./22-app-compatibility.md), and the corresponding app-package skill.
