# 20 · App store and WASM plugins

> Condensed English translation. [Complete Chinese original](../../Doc/20-应用商店与插件系统.md) is retained unchanged. Detailed tables, examples, and historical evidence omitted here still require unabridged translation. Source baseline: `7288d9b199e1b09b529fbaae38303f4c82f38612`.

## Model

The store supports 1Panel, aaPanel, and Flame/x-flame package ecosystems with container, native, and WASM installation modes. Metadata, version information, and typed form fields are normalized through adapters; a parsed package is not automatically safe to install.

Installation follows existing service orchestration, variable mapping, security scanning, task lifecycle, and mode-specific ports. Remote sources support signature verification; retain the Ed25519 trust boundary rather than treating a download URL as proof of authenticity.

## Container and native modes

Container mode renders validated Compose inputs and manages the application lifecycle. Native mode uses registered installers and constrained execution paths. Do not interpolate untrusted form values into shell commands or grant arbitrary root execution to make a package compatible.

Native web-engine configuration is handled through the existing render/validate/write/reload pipeline and a separate privileged helper with path/command allowlists. Cross-engine WAF capability is not identical: rate limiting is specific to Nginx/OpenResty; check real engine behavior before claiming parity.

## WASM boundary

The Wasmtime sandbox applies configured memory, fuel, stack, integrity, and host-capability limits. Lifecycle hooks and allowed imports/exports are contractual; do not describe an arbitrary WASM module as automatically trusted. Built-in WAT examples and plugin metrics use the same declared boundaries.

## Extending the store

Use the [SDK guide](./21-app-sdk.md) and the full Chinese metadata/schema examples. Add parser/variable/scanner/lifecycle regressions, keep task cancellation and failure reporting, and update API contracts and locales together. Preserve source trust, secret redaction, and compatibility semantics.

The detailed Chinese edition retains metadata tables, built-in package lists, plugin examples, database records, endpoints, and web-engine/WAF implementation notes. They are not reproduced here as a second mutable specification.
