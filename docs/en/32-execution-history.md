# 32 · Historical v1.x execution plan

> Condensed English translation. [Complete Chinese original](../../Doc/32-v1.x执行计划.md) is retained unchanged. Detailed tables, examples, and historical evidence omitted here still require unabridged translation. Source baseline: `7288d9b199e1b09b529fbaae38303f4c82f38612`.

## Historical status only

This document records the v1.x implementation sequence from the 1.0.0 baseline toward 1.3.0. It is not the current work queue, not a release record, and not permission to start features or ordinary cloud builds. Reconcile old “pending” language with current code and CHANGELOG.

## Recorded work areas

- `fpctl`: JSON-oriented operations and local authenticated API access for scripts/AI.
- W0: structured kernel logs and request-correlated error feedback.
- W1: standardized versioned deployment and upgrade/rollback workflow.
- W2: OpenAPI snapshot/reference consistency plus resource/stability testing.
- M-E: initial read-only IaC scope, with import/deployment explicitly excluded.
- M-AI: MCP access, approvals, and the optional assistant, with security and human-review boundaries.
- Frontend component-library preparation and evidence-based adjustment between waves.

Each wave was intended to be accepted independently and the next plan adjusted using actual CNB results. Historical wave tables, measurements, and adjustment records remain in the Chinese source rather than being reinterpreted as current promises.

## Current interpretation

IaC read-only export, Edge, MCP/approvals, and the optional assistant already have source. Current work follows [roadmap](./30-roadmap.md), [workflow](./31-development-workflow.md), and [cleanup evidence](./35-engineering-cleanup.md).

A successful gate run does not prove long-term stability, security audit, native compatibility, or release completion. Keep actual results tied to a source revision; do not copy historical test counts or resource measurements into a new delivery report.
