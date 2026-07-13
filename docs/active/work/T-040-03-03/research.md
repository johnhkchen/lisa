# Research: T-040-03-03

## Ticket boundary

The ticket is `T-040-03-03`, titled `rc-rebuild-and-green-gate`.
It belongs to story `S-040-03`, `hostile-path-regression-and-field-report`.
Its dependencies are `T-040-03-01` and `T-040-03-02`.
Both dependencies are complete at the current repository revision.
The ticket begins in Research and requires all remaining RDSPI phases.

The acceptance criterion is a release rebuild and verification obligation.
It requires:

- a release `lisa-plugin` build for `wasm32-wasip1`;
- a release `lisa-cli` build after the plugin build;
- the fresh plugin WASM to pass through `crates/lisa-cli/build.rs`;
- workspace formatting to pass;
- native Clippy to pass;
- WASM-target Clippy to pass;
- native workspace tests to pass;
- both newly landed hostile-path regressions to be present and passing;
- the results to be recorded in a rebuild artifact.

The ticket does not request a production behavior change.
It is a barrier between deterministic regression work and the live field report.
The dependent `T-040-03-04` owns the authorized live Codex-seat observation.

## Story boundary

Story `S-040-03` separates deterministic proof from live observation.
The two predecessor tickets pin distinct historical failures.
This ticket consumes their finished tree and certifies a settled release candidate.
The next ticket must use the exact CLI and WASM rebuilt here.

The story's honest boundary forbids treating the outer running Lisa loop as proof
of the newly embedded scheduler.
This ticket does not launch a metered provider seat.
It does not fix any anomaly found by the gate.
An unexplained red gate is completion-blocking evidence.

## Current revision and repository state

The current branch is `main`.
The current `HEAD` is the completion commit for `T-040-03-02`.
Recent history contains:

- the `T-040-03-01` blocking-Review regression source commit;
- the completion commit for `T-040-03-01`;
- the `T-040-03-02` pre-ownership regression source commit;
- the completion commit for `T-040-03-02`.

The worktree has Lisa-managed lifecycle changes:

- `.lisa/provenance.jsonl` is modified;
- `docs/active/tickets/T-040-03-03.md` is modified;
- two untracked fixture outputs exist below `crates/lisa-plugin/docs/`.

The untracked fixture paths are generated residue from plugin tests.
They are not source owned by this ticket.
They must not be staged, committed, deleted, or otherwise adopted.
The ordinary Git index initially contains no staged path.

## Workspace layout

The repository is a Cargo workspace containing `lisa-core`, `lisa-plugin`, and
`lisa-cli`.
The workspace version is `0.4.0-rc.7` and the Rust edition is 2021.
The release profile uses size optimization and link-time optimization.

`lisa-core` contains shared domain types and parsing logic.
`lisa-plugin` is the Zellij WASM scheduler and also has native unit tests.
`lisa-cli` is the native executable and embeds the plugin at compile time.

The production plugin target is `wasm32-wasip1`.
Its release output is:

`target/wasm32-wasip1/release/lisa.wasm`.

The native release CLI output is:

`target/release/lisa`.

## Embedding boundary

`crates/lisa-cli/build.rs` walks from `CARGO_MANIFEST_DIR` to the workspace root.
It reads `target/wasm32-wasip1/release/lisa.wasm`.
It copies those bytes to `OUT_DIR/lisa.wasm`.
It emits `cargo:rerun-if-changed` for the release WASM source.

If the release WASM is missing, the build script writes an empty placeholder.
Therefore build ordering is material to this acceptance criterion.
The plugin must be built successfully before the CLI build begins.

`crates/lisa-cli/src/templates.rs` uses `include_bytes!` for the `OUT_DIR` copy.
The CLI therefore embeds the bytes copied by the build script at compile time.
A matching byte count and SHA-256 between the release WASM and the fresh
`OUT_DIR` copy provides deterministic evidence at this boundary.

## Repository build recipes

The root `Justfile` defines `build` as the release WASM build.
Its `build-cli` recipe depends on `build`, touches the release WASM, and then
builds `lisa-cli --release`.
The touch activates Cargo's build-script invalidation without changing bytes.

The ticket names both Cargo build commands directly.
Running them in plugin-first order and reproducing the touch matches both the
literal criterion and the documented repository workflow.

## Verification surfaces

Workspace formatting is checked by:

`cargo fmt --all -- --check`.

There is one source formatting surface, not a target-specific formatter.

Broad native warning-strict linting is available through:

`cargo clippy --workspace --all-targets --all-features -- -D warnings`.

Production-target WASM linting is distinct:

`cargo clippy -p lisa-plugin --target wasm32-wasip1 -- -D warnings`.

Native workspace behavior is exercised by:

`cargo test --workspace`.

The canonical `just check` additionally combines a WASM `cargo check` with a
repeat of the native workspace test suite.

## Predecessor regression 1

`T-040-03-01` added:

`test_t039_06_02_blocking_review_never_prepares_done`.

It lives in `crates/lisa-plugin/src/lib.rs` tests.
It drives the real artifact-advance scheduler consumer.
It asserts that a valid blocking Review disposition retains assignment and lease,
does not prepare a completion transaction, emits no Done provenance, and leaves
dependent tickets blocked.

The predecessor review states that its pending-completion assertion would fail
against the pre-S-040-01 unconditional Review-to-Done path.

## Predecessor regression 2

`T-040-03-02` added:

`rc6_preownership_delivery_miss_is_durable_and_cli_retrievable`.

It also lives in the plugin native test module.
It drives the production bounded assignment-ack timeout path.
It requires a durable `AssignmentTransition` failure row and queries that same
physical ledger through the implementation used by `lisa status --ticket`.

The CLI evidence implementation now lives in
`crates/lisa-cli/src/preownership_status.rs`.
The plugin test includes that source under a test-only module boundary.
The predecessor review states that the physical-row assertion would fail on the
pre-S-040-02 scheduler because no row would exist.

## Artifact and transaction constraints

All phase artifacts must be written to:

`.lisa/attempts/T-040-03-03/1/work/`.

They must not be authored directly in `docs/active/work/T-040-03-03/`.
The ticket frontmatter phase and status must not be edited manually.

Meaningful ticket-owned source changes require exact-path `lisa commit-ticket`.
Ordinary `git add`, broad staging, and ordinary `git commit` are forbidden.
Generated Cargo products are not source units.
If the tree passes without source changes, no source transaction is required.

## Research conclusion

The existing tree already contains both required regressions and the supported
release embedding pipeline.
The remaining work is deterministic evidence production.
No current observation establishes a need for a source modification.
The verification must preserve unrelated lifecycle and fixture residue and stop
after this ticket's Review artifacts are complete.
