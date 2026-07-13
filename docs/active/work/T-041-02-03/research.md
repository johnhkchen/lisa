# Research: workspace and WASM green gate

## Ticket boundary

- Ticket: `T-041-02-03`, `workspace-wasm-green-gate`.
- Parent story: `S-041-02`, generated invariant evidence.
- The ticket starts in Research.
- It depends on `T-041-02-01` and `T-041-02-02`.
- Both dependency tickets are complete in the current history.
- This ticket is a settled-tree verification barrier.
- The story explicitly says no production source changes belong to this slice.
- The pure completion contract is consumed read-only.
- Any discovered contract defect is outside this ticket and must block completion.
- The required outputs are verification evidence and RDSPI artifacts.

## Acceptance surface

- `cargo test --workspace` must pass.
- The deterministic livelock regression must be present.
- The generated proptest state-machine suite must be present.
- The release `lisa-plugin` WASM build must succeed.
- The output must stay within the existing WASM size budget.
- Property-test dependencies must remain excluded from the WASM artifact.
- Formatting must be green.
- Native Clippy must be green.
- WASM-target Clippy must be green.

## Workspace organization

- The root `Cargo.toml` defines a resolver-v2 Cargo workspace.
- Workspace members are all crates under `crates/*`.
- `lisa-core` owns shared domain types and pure behavior.
- `lisa-plugin` owns the Zellij WASM adapter and dashboard.
- `lisa-cli` owns native command-line orchestration.
- The release profile uses `opt-level = "s"`.
- The release profile enables LTO.
- Those settings are relevant to release WASM size.

## Deterministic predecessor

- `crates/lisa-core/tests/recorded_livelock_regression.rs` exists.
- It was introduced by commit `e28d712`.
- Its source commit contains only that integration test.
- The test uses the public `lisa_core::completion` API.
- It represents the recorded T-009-01-01 ordering.
- Review artifact observation precedes Review-phase entry.
- Stop, timeout, reload, and confirmation are represented.
- A negative-control edge-triggered model reproduces the old failure.
- The aggregate-backed model converges to one request.
- It converges to one authoritative confirmation.
- It asserts no finish-up prompt over the existing artifact.
- It asserts no re-request.

## Generated predecessor

- `crates/lisa-core/tests/completion_state_machine.rs` exists.
- It was introduced by commit `5c03e6e`.
- That commit also modified `crates/lisa-core/Cargo.toml` and `Cargo.lock`.
- The test uses `proptest` and `proptest-state-machine`.
- It generates sequential arbitrary-order traces.
- It runs 256 cases.
- Each case contains one through 63 transitions.
- The generated vocabulary includes Review observations and phase entry.
- It includes stop, poll, duplicate result, reload, timeout, and recovery.
- The reference model is independent of production `reduce` and `reconcile`.
- The concrete harness exercises the public completion contract.
- It checks passing Review is never stranded.
- It checks blocked Review never completes.
- It checks at most one completion effect is live.
- It checks at most one authoritative Done is accepted.

## Dependency placement

- `crates/lisa-core/Cargo.toml` lists `proptest = "1.10"`.
- It lists `proptest-state-machine = "0.8"`.
- Both entries are under `[dev-dependencies]`.
- Neither appears under `[dependencies]`.
- `lisa-plugin` depends on `lisa-core` as a normal dependency.
- Cargo does not propagate a dependency crate's dev-dependencies to consumers.
- Release compilation of `lisa-plugin` therefore does not link these test tools.
- `Cargo.lock` records the development dependency graph for reproducibility.
- Lockfile presence alone does not imply release linkage.

## Existing verification commands

- The `justfile` defines `test` as `cargo test --workspace`.
- It defines `fmt-check` as `cargo fmt --all -- --check`.
- Its native lint commands cover core and CLI.
- Its plugin lint command targets `wasm32-wasip1`.
- The CI workflow follows the same split.
- CI checks formatting first.
- CI runs warning-strict Clippy for core and CLI.
- CI runs warning-strict plugin Clippy on WASM.
- CI runs the workspace test suite.
- CI checks the plugin on the WASM target.
- The acceptance criterion is stronger than CI because it requires a release build and size observation.

## WASM artifact path and budget

- The release build command is `cargo build -p lisa-plugin --target wasm32-wasip1 --release`.
- Its stable output is `target/wasm32-wasip1/release/lisa.wasm`.
- Prior baseline ticket `T-038-01-01` recorded 1,414,183 bytes.
- That value is a logical byte count from `wc -c`.
- It is the repository's explicit historical WASM baseline.
- Later comparison ticket `T-038-04-02` recorded 1,412,657 bytes.
- The later value was 1,526 bytes below the baseline.
- No newer explicit numeric ceiling was found.
- The conservative existing budget is therefore 1,414,183 bytes.
- Acceptance requires the current output to be at or below that ceiling.

## Artifact identity boundary

- `lisa-cli/build.rs` copies the stable WASM output into its build output.
- `lisa-cli` later embeds that copy.
- This ticket only needs the plugin release artifact itself.
- It does not need to rebuild the native release CLI.
- `wc -c` observes the uncompressed module length.
- A hash can identify the measured output but is not itself a size assertion.
- `file` can confirm the output remains a WebAssembly module.

## Repository state

- Lisa has modified `.lisa/provenance.jsonl`.
- Lisa has advanced the ticket phase from Ready to Research.
- Those are orchestration-owned changes.
- `crates/lisa-plugin/docs/` is an unrelated untracked path.
- It contains a test-like Review disposition fixture.
- It must not be touched, staged, or committed by this ticket.
- The ordinary Git index is empty.
- There are no ticket-owned source modifications at Research time.

## Constraints

- Phase artifacts must be written only under the private attempt work directory.
- Shared `docs/active/work/T-041-02-03/` must not be written directly.
- Ticket phase and status frontmatter must not be edited manually.
- Ordinary `git add` and `git commit` are forbidden for ticket work.
- Meaningful source changes, if any, require exact-path `lisa commit-ticket` calls.
- A verification-only outcome requires no source commit.
- Build products under `target/` are ignored generated outputs.
- Review requires both Markdown and a strict disposition JSON file.

## Environment observations

- The workspace root is `/Users/johnchen/swe/repos/lisa`.
- The host is a macOS development environment.
- The current date is 2026-07-12.
- The requested target is `wasm32-wasip1`.
- Current Git history places both predecessor completion commits immediately before this ticket.
- The current tree therefore represents the intended settled base.

## Research conclusion

- All necessary test and manifest inputs already exist.
- The property dependencies have the correct dev-only boundary.
- The release budget has a documented numeric ceiling.
- The work remaining is a reproducible verification matrix and evidence capture.
- No code edit is indicated by the current tree.
- A failed command or exceeded ceiling would be a blocking finding, not an invitation to broaden scope.
