# Structure: T-039-06-01

## Change shape

This ticket is structured as a deterministic build-and-verification pass.
No production source file is planned for creation, modification, or deletion.
The only authored files are private RDSPI phase artifacts in the active attempt.
The only other outputs are ignored Cargo build products under `target/`.

## Private attempt artifacts

Directory:

`.lisa/attempts/T-039-06-01/1/work/`

### `research.md`

Created in Research.
Maps the ticket, workspace, embedding pipeline, build recipes, gate surfaces,
repository state, and lifecycle constraints.
Contains observations only and does not prescribe implementation changes.

### `design.md`

Created in Design.
Compares repository recipe use, literal commands, cleanup, isolated targets, and
the combined selected verification approach.
Records why the supported default-target embedding path is retained.

### `structure.md`

Created in Structure.
Defines the file-level evidence layout and the ordering of generated products.
Records that the intended source-change set is empty.

### `plan.md`

Created in Plan.
Sequences preflight, builds, identity capture, format, Clippy, tests, canonical
checks, cleanliness inspection, and Review.
Defines pass criteria and deviation handling.

### `progress.md`

Created during Implement.
This is the primary ticket evidence record.
It will include:

- starting revision and worktree boundary;
- exact release plugin build command and result;
- WASM path, byte length, and SHA-256 digest;
- freshness touch and exact release CLI build command and result;
- CLI path, byte length, and SHA-256 digest;
- exact format command and result;
- exact native Clippy command and result;
- exact WASM Clippy command and result;
- exact workspace test command and result;
- exact canonical `just check` command and result;
- final source-cleanliness assessment;
- deviations, retries, or anomalies;
- remaining work at the point of writing.

### `review.md`

Created in Review.
Summarizes the resulting artifacts and the absence or presence of source edits.
Maps all acceptance clauses to evidence.
Evaluates coverage and limitations.
Names any blocking issue requiring human attention.

## Existing production files inspected but not modified

### `Justfile`

Defines the supported build and verification recipes.
Its `build-cli` dependency order is authoritative for the embedding sequence.
No recipe change is required by this ticket.

### `crates/lisa-cli/build.rs`

Defines the file-copy boundary between plugin output and CLI `OUT_DIR`.
The source remains the workspace's default release WASM path.
The build script remains unchanged.

### `crates/lisa-cli/src/templates.rs`

Defines `PLUGIN_WASM` through `include_bytes!`.
The embedding interface remains unchanged.

### `Cargo.toml`

Defines workspace members and the release profile.
It remains unchanged.

### `crates/lisa-cli/Cargo.toml`

Defines the native `lisa` binary package.
It remains unchanged.

## Generated build outputs

### `target/wasm32-wasip1/release/lisa.wasm`

Produced or refreshed by the release plugin build.
This is the build-script input.
It must exist and contain at least one byte before the CLI build.
It is touched after identity capture to trigger Cargo's rerun tracking.
It is ignored and not committed.

### `target/release/lisa`

Produced or refreshed by the release CLI build.
It contains the compile-time `PLUGIN_WASM` byte slice.
It is ignored and not committed.

### Cargo `OUT_DIR/lisa.wasm`

Produced indirectly by `crates/lisa-cli/build.rs`.
Its exact directory is selected by Cargo under `target/release/build/`.
It is an intermediate generated file.
It is not treated as a repository source or committed artifact.

## Unmodified lifecycle files

### `.lisa/provenance.jsonl`

Lisa has modified this file as part of active execution.
This ticket does not own that mutation.
The implementation must not overwrite, revert, stage, or commit it.

### `docs/active/tickets/T-039-06-01.md`

Lisa has modified this file as part of phase/status tracking.
The assignment explicitly forbids manual phase or status edits.
This ticket does not include it in any source transaction.

## Component boundaries

The plugin release build owns production WASM generation.
The CLI build script owns copying the existing WASM into `OUT_DIR`.
`templates.rs` owns compile-time inclusion of the copied bytes.
The CLI release build owns native linking of the final executable.

The verification phase does not cross these boundaries with new APIs.
It observes them by running their supported Cargo commands in dependency order.

## Execution ordering

Ordering is material at three points.

First, the plugin must build before the CLI.
Otherwise `build.rs` can create and embed an empty placeholder.

Second, the plugin output is touched before the CLI build.
This matches `just build-cli` and activates `cargo:rerun-if-changed` even when the
new plugin bytes are identical to an existing artifact.

Third, artifact identity capture occurs after each corresponding successful build.
This prevents hashes from being attributed to a failed or stale command result.

Formatting and linting may run after builds because they do not supply build inputs.
Workspace tests follow lints so a warning-strict failure is surfaced separately.
`just check` runs last as the canonical combined confirmation.

## Verification boundaries

### Build verification

The release plugin command must exit successfully.
The expected WASM must be non-empty.
The release CLI command must exit successfully after the touch.
The expected native CLI must be non-empty.

### Format verification

`cargo fmt --all -- --check` must exit successfully and make no edits.

### Native lint verification

Workspace Clippy must cover all targets and features with warnings denied.
It must exit successfully.

### WASM lint verification

Plugin Clippy must target `wasm32-wasip1` with warnings denied.
It must exit successfully.

### Test verification

`cargo test --workspace` must exit successfully.
Its summary counts will be recorded when available.

### Canonical verification

`just check` must exit successfully.
This confirms the normal WASM type-check and repeats workspace tests.

### Cleanliness verification

No ticket-owned source path may remain staged, modified, or untracked.
Expected Lisa-owned lifecycle changes may remain visible.
Private attempt artifacts may be untracked or ignored according to Lisa's setup.
Generated `target/` state is not a source cleanliness failure.

## Commit structure

There is no planned ticket-owned source unit, so no implementation commit is planned.
Creating an empty commit would not add durability or evidence.
The phase artifacts are reserved for Lisa's isolated completion publication.

If source unexpectedly changes, each coherent source unit becomes its own exact-path
`lisa commit-ticket` transaction after focused verification.
That contingency is not expected and would require a documented deviation.

## Review handoff structure

Review will lead with acceptance status.
It will list generated artifact identities.
It will list the exact green commands.
It will distinguish source coverage from runtime field coverage.
It will state that live Codex-seat observations are owned by `T-039-06-02`.
It will stop without changing ticket phase/status or starting the dependent ticket.
