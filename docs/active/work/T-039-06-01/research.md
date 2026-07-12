# Research: T-039-06-01

## Ticket boundary

The ticket is `T-039-06-01`, titled `rebuild-cli-and-embedded-wasm`.
It is the first ticket in story `S-039-06`, `rebuild-and-field-report`.
Its only dependency is `T-039-05-03`.
That dependency is complete at the current `HEAD`, commit `399708e`.
The ticket starts in the Research phase.
The ticket asks for a rebuild and verification pass over the completed tree.
It does not describe a production-code change.
The acceptance criterion has one compound requirement:

- build the release WASM plugin;
- build the release CLI with that fresh WASM embedded;
- pass native and WASM formatting, Clippy, and test gates;
- record the commands and results.

The story explicitly places structural changes outside this closing slice.
Structural refactors belong to earlier `S-039` tickets.
New defects found here belong in separate follow-up work.
An unexplained failure or behavior change must not be silently fixed in this ticket.

## Repository state

The workspace is a Rust Cargo workspace with resolver version 2.
The workspace members are all crates under `crates/*`.
The shared package version is `0.4.0-rc.7`.
The shared Rust edition is 2021.
The release profile uses size optimization and link-time optimization.

The current branch is `main`.
The current `HEAD` is `399708e Complete T-039-05-03`.
Recent history contains the production commits for the final structural slice:

- `e7d8cc0 test(core): lock provenance ticket attribution`;
- `a4fdeb7 fix(plugin): reject non-sibling publication temporaries`;
- `af788ef refactor(plugin): centralize atomic publication`.

The ordinary worktree already has two Lisa-owned changes:

- `.lisa/provenance.jsonl` is modified;
- `docs/active/tickets/T-039-06-01.md` is modified.

Those changes reflect the active ticket lifecycle.
They are not ticket-owned source changes for this assignment.
They must be preserved and must not be included in an implementation commit.

## Workspace components

`lisa-core` contains shared data types and scheduling-domain logic.
`lisa-plugin` is the Zellij plugin.
`lisa-cli` is the native `lisa` executable.

The plugin has a native test surface and a WASM production target.
Its production artifact is built for `wasm32-wasip1`.
The expected release artifact path is:

`target/wasm32-wasip1/release/lisa.wasm`.

The CLI binary is built natively.
Its expected release artifact path is:

`target/release/lisa`.

## WASM-to-CLI embedding path

The embedding boundary is implemented in `crates/lisa-cli/build.rs`.
The build script derives the workspace root from `CARGO_MANIFEST_DIR`.
It appends `target/wasm32-wasip1/release/lisa.wasm` for the source.
It appends `lisa.wasm` to Cargo's `OUT_DIR` for the destination.
It emits `cargo:rerun-if-changed` for the release WASM source path.

When the release WASM exists, the build script copies it into `OUT_DIR`.
When it does not exist, the build script writes an empty placeholder.
The placeholder permits ordinary CLI development builds without a plugin build.
It also means build order is material for a release verification pass.

`crates/lisa-cli/src/templates.rs` defines `PLUGIN_WASM`.
The constant uses `include_bytes!` with the copied `OUT_DIR/lisa.wasm` path.
The CLI executable therefore receives the bytes present after the build script runs.
The CLI package does not itself build the plugin as a Cargo dependency.
The plugin must be built first by an outer command or recipe.

## Existing build recipes

The root `Justfile` is the repository's command map.
The `build` recipe runs:

`cargo build -p lisa-plugin --target wasm32-wasip1 --release`.

The `build-cli` recipe depends on `build`.
It then touches the release WASM and runs:

`cargo build -p lisa-cli --release`.

Touching the release WASM makes Cargo reconsider the CLI build script input.
This ordering is the repository's documented embedding workflow.
The ticket acceptance text names the two Cargo build commands directly.
Executing them in the same plugin-first order matches the recipe.
Touching between them retains the recipe's explicit freshness signal.

## Formatting surface

The `fmt-check` recipe runs:

`cargo fmt --all -- --check`.

Formatting is workspace-wide and target-independent.
There is no separate WASM formatter.
The ticket's “native + WASM fmt” language therefore maps to one source-format gate.
That gate covers sources compiled in both configurations.
It is non-mutating because `--check` is present.

## Native Clippy surface

The `lint` recipe invokes native Clippy separately for `lisa-core` and `lisa-cli`.
It invokes plugin Clippy for the WASM target.
Prior admitted work uses a broader native command:

`cargo clippy --workspace --all-targets --all-features -- -D warnings`.

That command covers every native workspace target and denies warnings.
It includes test targets and enabled feature combinations.
It provides a stronger native gate than package-only default-target invocations.

## WASM Clippy surface

The root `lint` recipe uses:

`cargo clippy -p lisa-plugin --target wasm32-wasip1 -- -D warnings`.

This checks the production plugin target with warnings denied.
It is distinct from native workspace Clippy.
The WASM plugin depends on target-specific APIs and must retain this separate pass.

## Test surface

The root `test` recipe runs:

`cargo test --workspace`.

Tests execute natively, including the plugin's test modules.
The workflow guidance also identifies native tests as the supported test mode.
The WASM artifact is compiled and linted rather than executed as a test binary.
The workspace contains unit tests and CLI integration tests.

## Canonical combined check

The default `check` recipe runs two commands:

1. `cargo check -p lisa-plugin --target wasm32-wasip1`;
2. `cargo test --workspace`.

This provides an ordinary WASM type check and repeats the workspace tests.
It does not replace warning-strict Clippy or the release builds.
It remains useful as the repository's canonical developer gate.

## Artifact and evidence constraints

Phase artifacts belong under the private attempt directory:

`.lisa/attempts/T-039-06-01/1/work/`.

They must not be written directly to `docs/active/work/T-039-06-01/`.
Lisa publishes admitted artifacts after lease verification.
The ticket's phase and status fields must not be edited manually.

During implementation, meaningful source changes require `lisa commit-ticket`.
That command must receive exact repository-relative include paths.
Ordinary `git add` and `git commit` are forbidden for ticket work.
Build products under `target/` are generated outputs, not source units.
If all gates pass without source edits, there is no source unit to commit.

## Constraints and assumptions

The checkout may be shared by concurrent Lisa seats.
Broad cleanup of `target/` could disrupt other builds.
The repository-provided touch-based workflow establishes embedding freshness without cleanup.
Cargo's dependency tracking can reuse unchanged compilation outputs.
Success of the named commands is the acceptance boundary, not proof of recompiling every dependency.

The release WASM must exist and be non-empty before the CLI build.
Its byte count and SHA-256 digest can identify the exact plugin artifact.
The CLI's byte count and SHA-256 digest can identify the resulting executable.
Those measurements are supporting evidence, not substitutes for builds and gates.

No current evidence indicates a required source modification.
No current evidence indicates an acceptance criterion beyond local deterministic commands.
The subsequent `T-039-06-02` ticket owns live-seat field reporting.
This ticket must stop after its Review artifact and must not begin that work.
