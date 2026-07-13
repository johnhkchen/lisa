# Plan: pin the rc.6 pre-ownership evidence boundary

## Objective

Produce an always-on native regression that begins with an unacknowledged
assignment, reaches the real terminal delivery transition, observes one durable
assignment-transition row, and retrieves that row through the implementation
used by `lisa status --ticket`.

## Step 1: preserve the baseline

Confirm the concurrent T-040-03-01 source commit has completed and
`crates/lisa-plugin/src/lib.rs` is clean.

Record unrelated dirty and untracked paths without changing them.

Inspect the existing scheduler provenance test and CLI status tests so the
extraction preserves their contracts.

Verification:

- plugin source path has no uncommitted diff;
- main/status paths have no uncommitted diff;
- ordinary index contains no ticket-owned staged path.

Atomicity: read-only.

## Step 2: extract the CLI ledger report

Create `crates/lisa-cli/src/preownership_status.rs` with the existing
pre-ownership reader, renderer, state-name mapping, and focused tests.

Make the writer-taking function public for deterministic cross-boundary tests.

Remove the moved implementation and tests from `status.rs`.

Update `main.rs` so ticket status mode calls the new module while DAG status
continues to call the old module.

Verification:

- run `cargo fmt --all`;
- run focused CLI module tests;
- run the black-box `preownership_status` integration test;
- compare expected output with the pre-extraction contract.

Atomicity: keep the three CLI paths together with the dependent regression.

## Step 3: add the exact CLI test seam

Inside the plugin native test module, include the extracted CLI report source
using `CARGO_MANIFEST_DIR`.

Keep the inclusion under `#[cfg(test)]` so deployed WASM production code gains
no CLI filesystem/report code.

Verification:

- compile the plugin library tests;
- confirm the included module resolves only core/std dependencies;
- confirm WASM checking does not compile the test seam.

Atomicity: same source unit as Steps 2 and 4.

## Step 4: implement the historical regression

Add `rc6_preownership_delivery_miss_is_durable_and_cli_retrievable` near the
existing pre-ownership provenance tests.

Use `preownership_failure_state` with a Delivering Codex seat and expired
deadline.

Drive `check_assignment_ack_timeouts_at` instead of calling the failure helper
directly.

Assert the returned transition is delivery failure and the seat never becomes
owned.

Read the ledger and assert exactly one physical row.

Decode the typed record and assert ticket, lease, pane, provider, state, exact
reason, schema, and discriminator.

Assert raw JSON has no execution outcome or authority fields.

Call the extracted CLI writer on that same ledger and assert its visible
one-row report contains every required field.

Verification:

```text
cargo test -p lisa-plugin rc6_preownership_delivery_miss_is_durable_and_cli_retrievable
```

The test must fail if no row is appended. This is the key pre-S-040-02
discriminator.

## Step 5: run focused compatibility checks

Run the extracted CLI module tests.

Run the existing CLI black-box fixture test.

Run nearby plugin pre-ownership tests to guard exact-once behavior and later
Done coexistence.

Suggested commands:

```text
cargo test -p lisa-cli preownership_status
cargo test -p lisa-cli --test preownership_status
cargo test -p lisa-plugin preownership
```

Verification criteria:

- all focused tests pass;
- output remains byte-for-byte compatible;
- existing static fixture remains accepted;
- existing delivery/recovery/startup transition coverage remains green.

## Step 6: run broad gates

Run formatting in check mode after any formatter changes.

Run the native workspace suite.

Run plugin checking for `wasm32-wasip1`.

Run Clippy for the native workspace and deployed plugin target if practical
within the repository's established gate behavior.

Commands:

```text
cargo fmt --all -- --check
cargo test --workspace
cargo check -p lisa-plugin --target wasm32-wasip1
cargo clippy --workspace --all-targets -- -D warnings
cargo clippy -p lisa-plugin --target wasm32-wasip1 -- -D warnings
```

Verification criteria:

- no enabled test failures;
- no formatting drift;
- plugin compiles for deployed WASM target;
- no new Clippy warnings.

If a broad pre-existing failure is unrelated, document exact evidence rather
than changing foreign code.

## Step 7: inspect ownership and diff

Run scoped whitespace checks for the four ticket-owned source paths.

Review the complete diff and ensure it contains only:

- mechanical report-module extraction;
- main dispatch redirection;
- the test-only include seam;
- the historical regression.

Confirm unrelated Lisa-managed paths remain untouched.

Write `progress.md` with completed work, commands, results, deviations, and
the exact planned transaction.

## Step 8: commit the meaningful source unit

Use one isolated Lisa transaction because the new regression depends on the
module extraction and all changes jointly preserve one behavior.

```text
lisa commit-ticket \
  --ticket-id T-040-03-02 \
  --message "Pin pre-ownership CLI evidence regression" \
  --include crates/lisa-cli/src/main.rs \
  --include crates/lisa-cli/src/status.rs \
  --include crates/lisa-cli/src/preownership_status.rs \
  --include crates/lisa-plugin/src/lib.rs
```

Do not use ordinary `git add` or `git commit`.

After the transaction, verify:

- returned commit ID exists;
- commit contains exactly the four paths;
- `git show --check` passes;
- every ticket-owned source path is clean and unstaged.

## Step 9: review and disposition

Write `review.md` summarizing:

- the producer-to-consumer regression;
- module extraction and unchanged CLI contract;
- exact source paths and commit;
- focused and broad test results;
- coverage limits and open concerns.

Write `review-disposition.json` with pass only if all ticket-owned changes are
committed and the deterministic regression plus required gates pass.

Use block with a nonempty actionable reason if the dependency-owned production
behavior fails or any unexplained anomaly remains.

Do not change ticket phase/status, publish canonical artifacts, complete the
ticket manually, or start another ticket.
