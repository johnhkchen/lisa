# Research: T-039-01-01

## Ticket scope

- The ticket is `T-039-01-01`, titled `clear-test-only-clippy-debt`.
- Its starting phase is Research.
- Its acceptance criterion names a baseline of 13 Clippy findings.
- The required native command is `cargo clippy --workspace --all-targets --all-features`.
- The required cross-target check is Clippy for the `wasm32-wasip1` plugin build.
- Formatting and the complete native workspace test suite must remain green.
- Product source must not change; only test code is in scope.

## Repository build shape

- The Cargo workspace contains `lisa-core`, `lisa-plugin`, and `lisa-cli`.
- `lisa-core` contains shared ticket, phase, DAG, and configuration types.
- `lisa-plugin` is the Zellij plugin and has a WASM production target.
- `lisa-cli` is the native command-line application.
- Workspace compilation uses Rust edition 2021.
- The normal native test gate is `cargo test --workspace`.
- The repository-level formatting gate is `cargo fmt --all -- --check`.
- The `justfile` lint recipe checks the plugin for WASM and core/CLI natively.
- Its WASM lint command is `cargo clippy -p lisa-plugin --target wasm32-wasip1 -- -D warnings`.

## Baseline reproduction

- The native baseline command was run from the repository root.
- Command: `cargo clippy --workspace --all-targets --all-features`.
- It completed compilation successfully but emitted 13 warnings.
- Twelve warnings originate in `crates/lisa-core/src/dag.rs`.
- One warning originates in `crates/lisa-cli/src/init.rs`.
- `lisa-plugin` emits no warning in this native all-target/all-feature run.
- The twelve core warnings are all `clippy::unnecessary_to_owned`.
- The CLI warning is `clippy::needless_borrows_for_generic_args`.
- Cargo reports `lisa-core`'s library test target as the warning source.
- Cargo reports `lisa-cli`'s binary test target as the warning source.
- This establishes the requested before count as 13.

## DAG test findings

- The relevant code is inside the `dag.rs` test module.
- DAG APIs use owned ticket identifiers internally.
- Test fixtures create identifiers such as `T-001`, `T-002`, and `T-003`.
- Several assertions inspect collections of owned `String` values.
- `Vec<String>::contains` can compare an element through a borrowed `str`.
- The flagged assertions currently allocate a fresh `String` with `.to_string()`.
- Clippy identifies those allocations as unnecessary for membership checks.
- Two findings are in `test_get_blocked_by`.
- Two findings are in `test_get_dependencies`.
- Seven findings are in `test_dag_from_depends_on_only_no_blocks`.
- Two findings are in `test_end_to_end_scan_to_dag`.
- The total across those locations is twelve.
- Other `.to_string()` calls occur in the same test module.
- Calls passed to APIs whose signature requires `&TicketId` are not all flagged.
- The ticket is limited to the emitted findings, not a broad API refactor.

## CLI init test finding

- The relevant test is `test_plan_init_upserts_missing_config_keys`.
- It creates a temporary project directory.
- It writes an intentionally incomplete `.lisa.toml` file.
- The content is built with `format!` so it contains the current Lisa version.
- `std::fs::write` accepts content implementing `AsRef<[u8]>`.
- The current call passes `&format!(...)`.
- The temporary borrow is unnecessary because the owned `String` satisfies the API.
- Clippy recommends passing `format!(...)` directly.
- The assertion behavior after the write is independent of that ownership detail.

## Behavioral boundaries

- The findings are in tests rather than production functions.
- No public signatures need to change.
- No data structures need to change.
- No runtime scheduling behavior needs to change.
- No persisted file format needs to change.
- No new dependency is necessary.
- No feature gate is involved in the warning sites.
- The edits should be semantics-preserving ownership simplifications.
- Existing test names and assertions can remain intact.

## Repository-state constraints

- `docs/active/tickets/T-039-01-01.md` was already modified by Lisa.
- The assignment prohibits manually editing ticket phase or status.
- That pre-existing modification is not ticket-owned source work for this implementation.
- Phase artifacts belong under this attempt's private `work` directory.
- Ticket-owned source changes must use `lisa commit-ticket`.
- Exact repository-relative include paths are required.
- Ordinary `git add` and ordinary `git commit` are prohibited.
- Source files must not remain modified, staged, or untracked at Review completion.

## Verification observations

- Native all-target/all-feature Clippy exercises test code and exposes the debt.
- Adding `-D warnings` to the after-run converts any warning into failure.
- The WASM plugin check has a distinct target and must be run separately.
- Native workspace tests cover all three workspace members.
- Formatting verification detects accidental style drift.
- Because edits only simplify argument expressions, existing tests are sufficient.
- A new test would duplicate behavior already exercised by the edited assertions.

## Research conclusion

- The observed baseline matches the ticket's recorded count exactly.
- The complete warning set is localized to two files and test-only code.
- Twelve sites remove temporary owned strings from membership assertions.
- One site removes an unnecessary borrow around formatted test fixture content.
- Production code, APIs, dependencies, and configuration are outside the change boundary.
