# 23 · Agent communication and Edge

> Condensed English translation. [Complete Chinese original](../../Doc/23-Agent节点通信协议.md) is retained unchanged. Detailed tables, examples, and historical evidence omitted here still require unabridged translation. Source baseline: `7288d9b199e1b09b529fbaae38303f4c82f38612`.

## Role

`flamepanel-agent` is a modular Rust node process. It registers with the panel, sends periodic metrics (normally every 10 seconds), performs allowlisted actions, and exposes sandboxed files. Modules include config, heartbeat, server, executor, install policy, files, PTY, and Edge.

## Configuration and identity

Key settings include `PANEL_URL`, `NODE_NAME`, `NODE_HOST`, `AGENT_PORT`, `AUTH_TOKEN`, `BOOTSTRAP_TOKEN`, `FILE_ROOT`, `AGENT_MODE`, and `AGENT_TERMINAL`. Bootstrap registration credentials and per-agent authentication tokens serve different purposes.

`AGENT_MODE=direct` listens on the configured agent port; `edge` opens an outbound connection to `/ws/agent/edge` without an inbound listener; `both` supports migration with Edge preferred. Invalid modes warn and fall back to direct. See `agent/src/config.rs` for exact defaults.

## Metrics

The heartbeat payload maps to the panel's `HeartbeatRequest`. Retain legacy fields and compatibility with panels that ignore additional snapshot fields. Share CPU sample windows and network counters across heartbeats. First-sample network rates are zero; counter resets/invalid intervals do not create negative rates. CPU discovery may use available parallelism as a startup fallback, not a complete topology assertion.

## Operations and security

`POST /action` dispatches constrained actions. File listing/download/upload use a separately checked root boundary. Authenticate, rate-limit, validate paths/arguments, and fail closed for unsupported commands. Structured execution preserves arguments without shell splitting, but is still subject to allowlists.

Database/runtime installation permits only exact approved constant scripts plus validated positional arguments and restricted paths. Do not replace this with arbitrary `sh -c`, broad sed, or unchecked systemd units.

## Edge and terminal

Edge replays requests into the local authenticated route assembly and tracks node connectivity. Keep frame validation, limits, cancellation, and reconnect behavior. It does not authorize arbitrary browser commands.

Remote PTY is a deliberate arbitrary-shell capability running as the Agent, so it is disabled unless the node owner opts in with `AGENT_TERMINAL=1`. The panel requires `terminal:execute` and `node:execute`, audits sessions, limits concurrency, and closes on revocation/invalid identity/idle timeout.

Exact frame formats, payload examples, endpoint responses, and complete install allowlists remain in the Chinese protocol reference and source. Do not translate executable constants or protocol keys while translating comments.
