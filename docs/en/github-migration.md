# GitHub source migration and binary releases

[中文迁移说明](../github-migration.zh-CN.md)

## Repository roles and history

- CNB source baseline: `yin0410/Flamepanel@7288d9b199e1b09b529fbaae38303f4c82f38612`.
- GitHub target: `Forevery1021/Flamepanel` (already public).
- Development/testing stays in CNB cloud-native development, using a separate owned directory and strict official SSH-host verification.
- GitHub hosts the proposed source branch, review PR, and versioned binary build/release workflows.

The migration branch imports the current CNB source snapshot as a commit on top of existing GitHub history. CNB commit history stays available in the original CNB repository; it is not copied into GitHub. It does not force-push or replace GitHub `main`. Review the large source delta before merging. Existing target changes must be assessed against the CNB baseline; their content is not silently assumed to be part of the new source.

This change does not configure ongoing bidirectional mirroring, move issues/PRs/secrets/packages, or delete CNB. Do not use `git push --mirror` against the existing target.

## English and Chinese documentation

`README.md` and `AGENTS.md` are English defaults. Chinese copies remain in `README.zh-CN.md` and `AGENTS.zh-CN.md`. Original `Doc/` guides and generated API documents remain intact.

`docs/en/` contains 27 **condensed English guides** plus an explicit status index. They are not unabridged translations. Detailed schema/permission tables, code examples, historical evidence, generated API prose, CHANGELOG, design documentation, and auxiliary guides still require further translation/review. Do not report the entire repository as fully translated.

## Binary build matrix

`.github/workflows/release.yml` defines four Linux targets:

| Rust target | Asset suffix | Linking intent |
| --- | --- | --- |
| `x86_64-unknown-linux-gnu` | amd64 | Zig glibc 2.28 baseline |
| `aarch64-unknown-linux-gnu` | arm64 | Zig glibc 2.28 baseline |
| `x86_64-unknown-linux-musl` | amd64-musl | musl |
| `aarch64-unknown-linux-musl` | arm64-musl | musl |

The previous speculative ARMv7 entry is not carried into this matrix. A 32-bit full-feature build requires separate compiler/runtime/dependency verification; it is not relabeled as a supported target without evidence. musl configuration alone does not establish that every produced executable is fully static; inspect actual artifacts before claiming that property.

Rust is read from the checked-in toolchain, Node/npm from their existing files, and dependencies from committed lockfiles. Zig and cargo-zigbuild versions are explicit. Runner images, OS packages, and version-tagged third-party actions are not fully immutable, so this is not a complete reproducible-build or signed-provenance claim.

## Expected assets

For each suffix above:

- `flamepanel-linux-<suffix>.tar.gz`: `flamepanel` (kernel), `fpctl`, `fp-restore-probe`, `fp-restore-keycheck`.
- `flamepanel-agent-linux-<suffix>.tar.gz`: `flamepanel-agent`.

The frontend is packaged and uploaded **once** as `flamepanel-frontend.tar.gz`. Only release-prefixed artifacts are downloaded into the final staging directory; an unpackaged frontend directory or `export-openapi` executable must not become a release asset.

`scripts/release-assets.py` requires exactly these nine nonempty regular archives, rejects missing/unexpected entries and symlinks, and writes `flamepanel-<version>-checksums.txt`. The existing installer archive names remain compatible; its extraction boundary still selects only the expected panel/CLI executables. Extra recovery tools do not automatically become installed services or authorize secret access.

## Version and publication safety

- A `v*` tag triggers the release workflow. It must resolve to valid SemVer matching `flame-kernel/Cargo.toml`.
- Manual workflow input is passed through environment variables, not interpolated into a shell program.
- Manual runs default to the package version, **not the branch name**, and only upload verified build artifacts by default. Setting `create_draft=true` explicitly creates a **draft** release after all builds succeed.
- Normal jobs have read-only repository permission. Only the final release job receives `contents: write`.
- There is no automatic official release merely because this migration branch/PR was pushed.

Do not trigger a real tag/release or publish a draft without a separate release decision. A complete build matrix and a passing checksum unit test are different evidence layers. Checksums detect corruption; independent signing, SBOM, provenance, and distribution-specific smoke tests remain separate work.

## Validation

Run in CNB development against the final synchronized source:

```bash
python3 -m unittest discover -s scripts/tests -p test_release_assets.py
bash scripts/ci-local.sh --keep-going all
```

Use workflow linting and native release/package smoke checks where available, and record their actual outcomes separately. Native tests do not prove all four cross-target builds. See the delivery report for actual commands, failures, and unexecuted checks.

## Credentials

Use CNB/GitHub credentials only in authorized process memory/environment or connector authentication. Never put them in Git remotes, tracked files, logs, workflows, PR descriptions, or release assets. Workflow builds use the ephemeral `GITHUB_TOKEN`, not a committed personal token. Revoke temporary task tokens after completion.
