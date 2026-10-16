# 36 · Product proposals and scope boundaries

> Condensed English translation. [Complete Chinese original](../../Doc/36-后续产品路线规划.md) is retained unchanged. Detailed tables, examples, and historical evidence omitted here still require unabridged translation. Source baseline: `7288d9b199e1b09b529fbaae38303f4c82f38612`.

## Positioning

Continue as a lightweight Linux operations panel reusing host software, not a Kubernetes console or another container runtime. The proposal was based on `1ca4497239742f4746b98b361cde7d9cf6e3775a` on 2026-10-12, with later recovery slices documented separately.

A proposal does not authorize implementation, pushes, ordinary cloud builds, or releases. Do not prefill issue numbers where no real issue exists.

## Candidate sequence

- **P0:** trustworthy maintenance baseline, compatibility decisions, toolchains, tests, and evidence.
- **P1:** recoverable, explainable operations; first verify existing recovery/deployment behavior.
- **P2:** reliable nodes and safe operations, dependent on task/lifecycle evidence.
- **P3:** declarative/supply-chain capabilities only after architecture decisions.

A read-only IaC drift proposal would first define schema versions, stable IDs, secret references, omitted-field semantics, and permission filtering; comparison must not execute import/deploy. App-upgrade preflight must distinguish executable rollback from database/volume recovery. Signing/provenance/SBOM policy is a separate supply-chain ADR.

Defer eBPF, QUIC, zero-trust networking, image matrices, PR preview clusters, and self-built Kubernetes management until there is a concrete user problem, threat model, operations cost, and design decision.

## Starting a slice

Record the user problem and end-to-end success path, existing source/API/permission/test evidence, actual gap, allowed/prohibited files, compatibility/recovery boundaries, positive/negative/lifecycle cases, environment budget, final-tree evidence, and separately authorized external actions.

## Authorized recovery work versus pending work

R-01A–D and E-1 have implementations and synthetic acceptance paths. They do not establish production identity, full business reads, isolated normal startup, external recovery, or RTO/RPO. E-2 onward requires its own inputs/resources/security decisions. See [Doc/41](./41-recovery-preflight.md) for prerequisites and stop conditions. Public cleanup candidates C02–C06 remain until a compatibility decision.

The detailed product comparisons and per-slice proposal tables remain in the Chinese source, not a new implementation commitment.
