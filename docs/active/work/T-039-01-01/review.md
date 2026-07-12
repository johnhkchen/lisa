# Review: T-039-01-01

## Outcome

The pre-existing test-only all-target/all-feature Clippy baseline is clean.

- Recorded before count: 13 warnings.
- Recorded after count: 0 warnings.
- Native strict Clippy: passed.
- WASM strict Clippy: passed.
- Formatting: passed.
- Workspace tests: 721 passed, 0 failed.
- Non-test product source changed: no.

## Source changes

### `crates/lisa-core/src/dag.rs`

- Modified only code within the existing test module.
- Removed twelve unnecessary temporary `String` allocations from membership assertions.
- Membership probes now use borrowed string literals directly.
- The collections still contain owned ticket IDs.
- Test fixtures, assertion meanings, and assertion order are unchanged.
- Production DAG construction, scheduling, traversal, and query logic are unchanged.

Affected tests include:

- `test_get_blocked_by`;
- `test_get_dependencies`;
- `test_dag_from_depends_on_only_no_blocks`;
- `test_end_to_end_scan_to_dag`.

### `crates/lisa-cli/src/init.rs`

- Modified only code within the existing test module.
- Removed one unnecessary borrow around `format!` output passed to `fs::write`.
- The temporary `.lisa.toml` fixture content remains byte-for-byte equivalent.
- Production initialization planning and file mutation behavior are unchanged.

Affected test:

- `test_plan_init_upserts_missing_config_keys`.

## Commit

- Transaction mechanism: `lisa commit-ticket`.
- Commit: `2395cdb6a708e78132d21e6de68b78e7e3aa1463`.
- Message: `T-039-01-01: clear test-only Clippy debt`.
- Exact committed paths:
  - `crates/lisa-core/src/dag.rs`;
  - `crates/lisa-cli/src/init.rs`.
- Diff size: 2 files, 13 insertions, 13 deletions.
- No ordinary `git add` or `git commit` was used.
- Both ticket-owned source paths are clean after the transaction.

## Verification evidence

### Baseline reproducer

```text
cargo clippy --workspace --all-targets --all-features
```

Before implementation this emitted exactly 13 warnings:

- 12 `clippy::unnecessary_to_owned`;
- 1 `clippy::needless_borrows_for_generic_args`.

After implementation the same command emitted zero warnings and exited successfully.

### Strict native lint gate

```text
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

Result: passed with zero warnings.

### WASM lint gate

```text
cargo clippy -p lisa-plugin --target wasm32-wasip1 -- -D warnings
```

Result: passed with zero warnings.

This uses the repository's established target-specific plugin command and checks the
actual `wasm32-wasip1` compilation target.

### Formatting gate

```text
cargo fmt --all -- --check
```

Result: passed.

### Test gate

```text
cargo test --workspace
```

Result:

- `lisa-cli`: 274 passed;
- `lisa-core`: 155 passed;
- `lisa-plugin`: 292 passed;
- total unit tests: 721 passed;
- failures: 0;
- documentation tests: passed.

## Test coverage assessment

- No new tests were added.
- The edited DAG expressions are executed by existing DAG unit tests.
- The edited CLI expression is executed by its existing init unit test.
- The full workspace run confirms no cross-crate regression.
- Clippy itself is the direct regression detector for the ticket's acceptance condition.
- The WASM gate confirms the native test cleanup did not disturb target-specific linting.
- Additional behavior tests would not add meaningful coverage for ownership-only syntax changes.

## Scope and safety review

- No public API changed.
- No production implementation changed.
- No dependency or feature changed.
- No configuration or persisted format changed.
- No CI, `justfile`, or lint policy changed.
- No lint suppression was introduced.
- No unrelated cleanup was included.
- The exact before/after warning count is reproducible from the recorded command.

## Open concerns

- No known functional concern.
- No test gap requiring follow-up.
- No TODO was introduced.
- The working tree still shows Lisa-managed ticket/work publication entries; these are
  intentionally excluded from the ticket-owned source commit and are handled by Lisa.
- Lisa must perform the completion publication and final completion commit before the
  ticket is considered Done and the seat is released.

## Human review focus

- Confirm every diff hunk remains within a test module.
- Confirm the native warning count is 13 before and 0 after using the recorded command.
- Confirm the commit contains exactly the two declared source paths.
- No further source action is recommended for this ticket.
