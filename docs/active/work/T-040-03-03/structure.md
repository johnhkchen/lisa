# Structure: T-040-03-03

## Change shape

This ticket is a release build and deterministic verification barrier.
No product source file is planned for creation, modification, or deletion.
The authored changes are attempt-private RDSPI and evidence artifacts.
Cargo will refresh ignored generated outputs under `target/`.

## Attempt-private artifact directory

All authored files live under:

`.lisa/attempts/T-040-03-03/1/work/`.

Lisa owns admission and publication to `docs/active/work/T-040-03-03/`.
The agent does not directly edit the shared published location.

## `research.md`

Created during Research.
Maps the ticket and story boundary, current revision, worktree residue, workspace
components, build-script embedding path, gate commands, predecessor regression
locations, and transaction constraints.

It is descriptive and contains no implementation prescription.

## `design.md`

Created during Design.
Compares recipe-only, literal-command, clean-target, isolated-target, and combined
verification approaches.
Selects the ordered default-target rebuild with explicit focused and broad gates.
Defines failure and source-commit policy.

## `structure.md`

Created during Structure.
Defines the evidence files, inspected source boundaries, generated artifacts,
command ordering, ownership rules, and handoff interface.

## `plan.md`

Created during Plan.
Sequences preflight, release builds, identity capture, focused regressions, full
gates, final state inspection, evidence writing, and Review.
Each step has an independent pass condition.

## `progress.md`

Created during Implement.
Tracks actual execution against the plan.
It includes:

- starting commit and worktree boundary;
- completed dependency confirmation;
- exact build command outcomes;
- artifact paths, sizes, and hashes;
- build-script copy comparison;
- focused regression outcomes;
- complete gate outcomes;
- test-suite observations;
- anomalies and retries;
- source transaction disposition;
- remaining Review work.

## `rebuild.md`

Created during Implement as the acceptance-specific rebuild artifact.
It is the downstream field-report handoff and contains:

- release candidate version and Git revision;
- ordered command ledger;
- release WASM identity;
- exact fresh `OUT_DIR` WASM copy identity;
- byte-for-byte equality conclusion;
- release CLI identity;
- named hostile-regression proof;
- formatting, lint, test, and check results;
- expected ignored-test disclosure;
- clean-source conclusion;
- explicit deterministic-versus-live boundary.

This file is evidence, not a generated binary or source module.

## `review.md`

Created during Review.
Summarizes all authored/generated outputs, acceptance mapping, test coverage,
limitations, ownership, and open concerns.
It gives a human reviewer enough context without requiring raw build logs.

## `review-disposition.json`

Created alongside `review.md`.
It has exactly one workflow-approved shape.

On a fully green, anomaly-free result:

`{"disposition":"pass","reason":null}`

If any required result remains red or unexplained, it instead uses `block` with
a non-empty actionable reason.

## Inspected production files

### `Cargo.toml`

Supplies workspace membership, version `0.4.0-rc.7`, and release profile.
It is read-only for this ticket.

### `Justfile`

Supplies the supported `build`, `build-cli`, `check`, `lint`, and formatting
recipes.
Its `build-cli` touch determines the freshness step used here.
It is read-only for this ticket.

### `crates/lisa-cli/build.rs`

Defines the copy from the default release WASM path into Cargo `OUT_DIR`.
It is read-only for this ticket.

### `crates/lisa-cli/src/templates.rs`

Defines the compile-time `include_bytes!` consumer of the `OUT_DIR` WASM.
It is read-only for this ticket.

### `crates/lisa-plugin/src/lib.rs`

Contains both historical regression tests as native test functions.
It is compiled for native tests and production WASM.
It is read-only for this ticket.

### `crates/lisa-cli/src/preownership_status.rs`

Contains the CLI reader/renderer queried by the rc.6 regression.
It is compiled natively and included in the plugin test seam.
It is read-only for this ticket.

## Generated product artifacts

### `target/wasm32-wasip1/release/lisa.wasm`

Produced by the exact release plugin build.
Serves as the CLI build-script source.
Must be non-empty.
Its byte count and SHA-256 identify the plugin passed to embedding.

### `target/release/build/lisa-cli-*/out/lisa.wasm`

Produced by `crates/lisa-cli/build.rs` during the release CLI build.
The newest current-build copy is selected and compared with the release source.
An exact size and SHA-256 match is required.

### `target/release/lisa`

Produced by the native release CLI build.
Its byte count and SHA-256 identify the executable handed to the next ticket.

All three are ignored generated artifacts and are never passed to
`lisa commit-ticket`.

## Existing unrelated worktree paths

`.lisa/provenance.jsonl` and the active ticket file are Lisa lifecycle state.
The two `crates/lisa-plugin/docs/.../review-disposition.json` paths are fixture
residue already present before implementation.

This ticket will not alter, remove, stage, or commit those paths.
They remain visible in final status and are called out as unrelated.

## Execution ordering

The ordering boundary is:

```text
preflight
  -> release WASM build
  -> WASM identity
  -> WASM touch
  -> release CLI build
  -> OUT_DIR copy equality
  -> CLI identity
  -> focused regressions
  -> format and lint gates
  -> workspace tests
  -> canonical just check
  -> final cleanliness
  -> evidence and Review
```

The CLI build cannot precede the release WASM build because the build script has
an empty-placeholder fallback.
The copy comparison cannot precede the CLI build because the current `OUT_DIR`
copy is produced during that build.

## Public and internal interfaces

No Rust public interface changes.
No module boundary changes.
No manifest or dependency changes.
No persisted schema changes.
No command-line interface changes.

The only new interface is documentary: `rebuild.md` provides the exact revision,
artifact identities, and deterministic gate status consumed by `T-040-03-04`.

## Commit structure

Expected meaningful source units: zero.

Therefore the expected number of `lisa commit-ticket` transactions is zero.
This is not a skipped transaction; there is no ticket-owned source diff to
submit.

If implementation unexpectedly changes source, that is a structural deviation.
It must be recorded before any isolated exact-path source transaction occurs.

## Review stop boundary

After `review.md` and `review-disposition.json` exist, no further ticket is
started.
The active ticket frontmatter is not edited.
Lisa owns final publication, Done preparation, completion commit, and seat release.
