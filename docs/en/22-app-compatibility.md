# 22 · 1Panel, aaPanel, and native compatibility

> Condensed English translation. [Complete Chinese original](../../Doc/22-1Panel与原生软件兼容性.md) is retained unchanged. Detailed tables, examples, and historical evidence omitted here still require unabridged translation. Source baseline: `7288d9b199e1b09b529fbaae38303f4c82f38612`.

## Goal and limits

FlamePanel aims to normalize three package ecosystems, not reproduce every external control-panel behavior. Adapter support, actual lifecycle behavior, and planned compatibility work are different states.

For 1Panel migration, inspect the existing `OnePanelAdapter`, metadata/form mappings, Compose variables, network conventions, dependencies, and hooks. Check aaPanel-specific behavior separately. Preserve the complete Chinese migration example when exact package fields are needed; this English guide does not invent alternate schemas.

## Native software

Use the existing native-installer registry and lifecycle abstraction where available. A custom `install.sh` package is still constrained by execution mode, helper/agent allowlists, paths, and supported distribution packages. Prefer existing database/web-engine/runtime APIs to duplicate installation logic.

## Migration checklist

1. Identify source format and declared installation mode.
2. Validate parser output and field substitutions using fixtures.
3. Review Compose networks, volumes, ports, environment variables, and secret handling.
4. Establish dependencies and supported native packages on the actual distribution.
5. Verify install, inspect, start/stop, upgrade, failure, cancellation, and cleanup semantics.
6. Review source trust/signatures and structured security findings.
7. Record unsupported hooks or lifecycle gaps instead of silently claiming full compatibility.

The Chinese guide contains detailed gap tables and historical milestones. Native-installer abstraction, remote sources, and signing have existing implementations; recheck current code/CHANGELOG before scheduling them as new features. Real distribution/engine validation remains separate from parser tests.
