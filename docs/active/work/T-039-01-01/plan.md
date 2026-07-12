# Plan: T-039-01-01

## Step 1: Preserve the baseline evidence

- Run `cargo clippy --workspace --all-targets --all-features`.
- Capture combined output.
- Verify the count is 13 warnings.
- Classify each warning by file, test, and lint name.
- Stop and reassess if product code is implicated.

Verification:

- Twelve `unnecessary_to_owned` warnings in DAG tests.
- One `needless_borrows_for_generic_args` warning in an init test.
- No other warning class or file.

## Step 2: Simplify DAG membership probes

- Edit only the twelve Clippy-reported `contains` arguments.
- Replace borrowed temporary `String` probes with string literals.
- Preserve all fixtures and surrounding assertions.
- Review the diff to ensure production lines are unchanged.

Verification:

- Search the diff for all twelve diagnostic locations.
- Confirm only the test module changed.
- Run `cargo clippy -p lisa-core --all-targets --all-features -- -D warnings`.

## Step 3: Simplify init fixture writing

- Edit only `test_plan_init_upserts_missing_config_keys`.
- Pass `format!(...)` directly to `fs::write`.
- Preserve the fixture's exact TOML content.
- Review the diff to ensure production init logic is unchanged.

Verification:

- Run the specific test if supported by normal filtering.
- Run `cargo clippy -p lisa-cli --all-targets --all-features -- -D warnings`.

## Step 4: Format and inspect

- Run `cargo fmt --all -- --check` first.
- If it fails due to ticket-owned lines, run `cargo fmt --all` and inspect all changes.
- Do not absorb unrelated formatting changes.
- Inspect `git diff` limited to both source paths.

Verification:

- Formatting check exits zero.
- Source diff contains thirteen expression simplifications only.
- No production code line changes.

## Step 5: Verify native lint acceptance

- Run `cargo clippy --workspace --all-targets --all-features`.
- Capture output for the after record.
- Confirm the warning count is zero.
- Run again with `-- -D warnings` as the strict gate.

Verification:

- Both commands exit zero.
- Cargo emits no `warning:` diagnostic.
- Recorded result is before 13, after 0.

## Step 6: Verify behavior

- Run `cargo test --workspace`.
- Record crate and documentation-test summaries.
- Investigate any failure before committing.

Verification:

- Every workspace test target passes.
- No ignored or filtered behavior is introduced by the change.

## Step 7: Verify WASM lint acceptance

- Run the repository-standard plugin command:
  `cargo clippy -p lisa-plugin --target wasm32-wasip1 -- -D warnings`.
- Confirm the target is installed and compilation completes.
- Do not substitute a native plugin check for this command.

Verification:

- Command exits zero.
- No warning is emitted.

## Step 8: Commit the source unit

- Update `progress.md` with completed work and verification results.
- Invoke `lisa commit-ticket` with ticket ID `T-039-01-01`.
- Use a message describing test-only Clippy cleanup.
- Include exactly:
  - `crates/lisa-core/src/dag.rs`;
  - `crates/lisa-cli/src/init.rs`.
- Do not use the ordinary index.

Verification:

- The Lisa transaction succeeds.
- Both source paths are clean afterward.
- The active ticket's Lisa-managed modification remains separate.
- No ticket-owned staged, modified, or untracked source remains.

## Step 9: Review and handoff

- Inspect the committed diff and final status.
- Write `review.md` in the private attempt work directory.
- Summarize changes, exact tests, warning counts, and concerns.
- Do not edit phase or status frontmatter.
- Do not publish artifacts manually.
- Remain on this ticket after Review.

## Testing strategy rationale

- No new tests are needed because behavior does not change.
- Existing assertions directly execute every edited DAG expression.
- The init test directly executes the edited `fs::write` call.
- Full workspace tests protect cross-crate assumptions.
- Strict Clippy commands are the primary regression checks for this ticket.
- The target-specific command protects the plugin's actual deployment target.
