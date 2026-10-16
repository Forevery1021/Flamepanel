# 30 · Maintenance roadmap

> Condensed English translation. [Complete Chinese original](../../Doc/30-路线图与里程碑.md) is retained unchanged. Detailed tables, examples, and historical evidence omitted here still require unabridged translation. Source baseline: `7288d9b199e1b09b529fbaae38303f4c82f38612`.

## Current direction

FlamePanel remains a lightweight Linux management plane that reuses host software. The current source roadmap prioritizes authorized R-01 recovery-validation slices and a stable engineering baseline; unrelated future product work is not automatically approved.

Four states must remain distinct: code exists, quality gates passed, real-system acceptance passed, and a formal version was released. A milestone heading proves none of them.

## Existing capabilities

The repository already includes read-only IaC export, outbound Agent Edge, MCP/approvals, an optional assistant, and modular agent/application components. Do not schedule these wholesale as unimplemented work based on older plans.

## Maintenance slices

Follow evidence-based cleanup in Doc/35. Preserve public compatibility candidates until an explicit decision; review architecture/toolchain debt independently. Keep tests/toolchains/docs stable without introducing undeclared features or deleting runtime code merely because grep finds no caller.

## Recovery progression

R-01A covers offline SQLite checks; B adds the dedicated migration/HTTP probe; C inventories external assets; D explicitly checks the staged encryption key; E-1 correlates reports. None proves full production recovery. Identity, business reads, startup/OS isolation, external assets, and real RTO/RPO remain separately scoped work; see [Doc/41](./41-recovery-preflight.md).

## Non-goals and release

Do not automatically expand into Kubernetes, eBPF, QUIC, zero-trust networking, preview clusters, or image matrices without a user problem, threat model, operations budget, and architecture decision.

Release is independent from maintenance: final gates, necessary security/performance audits, compatibility/native acceptance, version alignment, then authorized tag/release/image publication. The full Chinese roadmap and CHANGELOG retain historical state; this guide omits detailed milestone tables.
