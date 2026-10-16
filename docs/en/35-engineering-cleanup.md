# 35 · Engineering cleanup evidence

> Condensed English translation. [Complete Chinese original](../../Doc/35-工程清理清单.md) is retained unchanged. Detailed tables, examples, and historical evidence omitted here still require unabridged translation. Source baseline: `7288d9b199e1b09b529fbaae38303f4c82f38612`.

## Evidence classes

The original review baseline is `2f0b1dacd044f7dedb3d2bf1efe9383b508b4408` (2026-10-10). Static searches are leads, not deletion authority.

- **A:** private placeholder with strong no-use evidence, no macro/feature/export entry.
- **B:** public symbol with no repository caller; external/compatibility consumers still need a decision.
- **C:** duplication/debt; similar code can differ in security, error, platform, or lifecycle semantics.

Preserve schemas, compatibility layers, RAII guards, and feature-only paths. `allow(dead_code)`, TODOs, and file size alone do not establish dead code.

## Candidate status

C01 (`api/ws_auth.rs::_header_ref`) was cleaned up with its dedicated import/exemption. C02–C06 remain public compatibility candidates, including request logging/WS wrappers, app-state constructors/JWT access, certificate convenience methods, and entity state helpers. Do not delete them without checking public contracts and consumers.

D01 concerns backup/DNS credential seal/open policy. Any sharing must preserve empty JSON, legacy plaintext, unavailable keys, corrupt ciphertext, and redaction; do not silently change fallback into fail-closed behavior.

D02 (Agent/kernel constant-time comparison) retains independent implementations with shared test vectors and separate byte boundaries. Functional equivalence tests are not a constant-time proof and do not justify making the Agent depend on the full kernel.

D03's native/runtime registries have different port types and low duplication value. D04 frontend fixtures differ in contracts; if sharing is approved, use stateless factories and fresh Pinia instances. Documentation duplication should be reduced by linking authoritative sources, not deleting safety guidance.

## Debt and acceptance

Architecture/toolchain/build debt is tracked separately. A02 build configuration was fixed in the recorded maintenance work; later A05/A07 and D02 slices have their own evidence. Recheck the Chinese per-item status before scheduling new changes.

A cleanup is complete only after compatibility decisions, scoped diffs, targeted regressions, final-tree CNB gates, and any required actual image build/run. No product feature is implied. This condensed guide omits the full candidate/location table; the retained Chinese document is the detailed evidence register.
