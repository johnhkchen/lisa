# Plan — T-048-02-02 ask authoring and auto-recheck

## Implementation principles

- Preserve ticket frontmatter phase/status ownership by Lisa except for the
  product behavior that intentionally reopens other parked fixture tickets.
- Reuse the existing native read-only check runner.
- Keep world-auto eligibility stricter than manual unblock eligibility.
- Keep check failures as expected no-op outcomes, not command failures.
- Use one asynchronous command at the existing scheduler cadence.
- Let open status, DAG rebuild, existing Unpark reconciliation, and ordinary
  seat selection form the complete success path.
- Commit only exact ticket-owned source paths with `lisa commit-ticket`.

## Step 1: extend Review authoring instructions

Modify `crates/lisa-cli/data/rdspi-workflow.md` in the Review section.

Add the full structured block schema while retaining the exact pass schema.

Define:

- the three owner values;
- the honest owner selection rule;
- the externally observable check requirement;
- the one-sentence ask language rule;
- optional exact steps;
- the required bad/good Pages release example.

Mirror the rendered text into `docs/knowledge/rdspi-workflow.md`, preserving the
purpose paragraph and byte equality.

Extend `crates/lisa-cli/src/templates.rs` tests to pin each acceptance phrase
and exact example string.

### Verification

Run:

```text
cargo test -p lisa-cli templates::tests::test_rdspi_workflow_embedded
cargo test -p lisa-cli templates::tests::test_review_disposition_contract_is_injected
```

The first must prove raw/rendered equality. The second must prove the complete
authoring contract is present.

Run `cargo fmt --all -- --check` even though the data files dominate this unit.

### Commit

Use `lisa commit-ticket` with exactly:

- `crates/lisa-cli/data/rdspi-workflow.md`;
- `docs/knowledge/rdspi-workflow.md`;
- `crates/lisa-cli/src/templates.rs`.

Message: `T-048-02-02: teach agents to author actionable blocks`.

## Step 2: add native world-recheck operation

In `crates/lisa-cli/src/unblock.rs`, add `run_world_rechecks`.

Load configuration and scan tickets once.

Build a ticket ID to file path lookup from that scan.

Use `collect_parked_remedies` and retain World remedies with a check.

For each check:

- call `run_check(root, check, CHECK_TIMEOUT)`;
- on Passed, write Open status and record the ID;
- on Failed, TimedOut, or ChangedFiles, continue with no mutation;
- propagate infrastructure/status-write errors.

Avoid using `run_unblock` because it permits operator and checkless cases.

In `crates/lisa-cli/src/main.rs`, add hidden `RecheckWorld` plumbing.

Dispatch to the new function, print reopened IDs line by line, and preserve an
empty successful output for a no-change pass.

### Tests

In `crates/lisa-cli/tests/parked_ux.rs`, add a recheck helper and fixtures.

Passing World fixture:

- create an observable ready marker;
- invoke hidden recheck;
- assert success and exact ticket ID output;
- assert Open and DAG-ready.

Failing World fixture:

- use a nonzero read-only check;
- preserve original ticket bytes;
- invoke hidden recheck;
- assert zero exit with empty output;
- assert bytes unchanged, Blocked, and not ready.

Operator fixture:

- use a passing check with a visible sentinel side effect in scratch if useful;
- invoke hidden recheck;
- assert it remains Blocked and produces no output.

Mutation fixture:

- use `touch must-not-exist` under World owner;
- assert live sentinel absent and Blocked.

Timeout fixture:

- use `sleep` beyond five seconds;
- measure wall time;
- assert the command returns within a conservative upper bound;
- assert Blocked.

If the full five-second black-box timeout materially slows focused iteration,
retain one integration timeout case and rely on the existing 60ms unit test for
process-group precision.

### Verification

Run:

```text
cargo test -p lisa-cli --test parked_ux --no-fail-fast
cargo test -p lisa-cli unblock::tests --no-fail-fast
cargo clippy -p lisa-cli --all-targets -- -D warnings
```

Confirm existing visible `unblock` fixtures remain byte-for-byte passing.

### Commit

Use `lisa commit-ticket` with exactly:

- `crates/lisa-cli/src/unblock.rs`;
- `crates/lisa-cli/src/main.rs`;
- `crates/lisa-cli/tests/parked_ux.rs`.

Message: `T-048-02-02: verify world-owned parks automatically`.

## Step 3: add plugin effect construction and state

In `crates/lisa-plugin/src/lib.rs`, add the process-local in-flight boolean.

Add a pure `build_world_recheck_command` method alongside completion command
construction. Pin argv rather than shell syntax.

Add an eligibility method over `collect_parked_remedies` that requires World
owner and check presence.

Add `request_world_recheck`:

- suppress when already in flight;
- suppress when no eligible remedy;
- build the command;
- set in-flight before launch;
- invoke the host effect with project root cwd and tagged context.

Keep builder failure visible as a warning without setting in-flight.

### Verification

Add pure/native tests for:

- exact binary, subcommand, path, and context;
- missing binary/root rejection;
- World+check eligibility;
- Operator exclusion;
- checkless World exclusion;
- duplicate in-flight suppression.

Run focused plugin tests by their names before lifecycle wiring.

## Step 4: wire startup, cadence, and result handling

Add `handle_world_recheck_result`.

On every result, clear in-flight.

On successful empty output, return without logging or mutation.

On successful nonempty output:

- rebuild DAG;
- reconcile Unpark provenance;
- schedule ready tickets;
- log the reopened IDs for observability.

On command failure, leave durable and scheduling state unchanged and log a
warning.

Integrate startup request after permission grant.

Integrate cadence request into `poll_tick` without adding a timer.

Integrate result attribution into `RunCommandResult` with a unique context key.

### Scheduler fixture

Construct a temporary project containing:

- one blocked Review ticket;
- a canonical World disposition with a check;
- a latest World Park provenance row with `recheck_eligible: true`;
- one available agent slot;
- scheduler paths and timing state required for ordinary scheduling.

Simulate the native success boundary by changing the fixture ticket to Open,
setting in-flight, and delivering a successful result naming the ticket.

Assert:

- the DAG now reads Open;
- exactly one Unpark row follows the Park row;
- the Unpark retains World owner and recheck eligibility;
- the ticket is assigned through `schedule_ready_tickets` on that result pass;
- no operator command or manual unblock method is involved;
- repeated reconciliation does not append another Unpark row.

Construct failure/no-change cases and assert:

- Blocked status remains;
- ledger bytes/record count remain unchanged;
- no seat is assigned;
- in-flight clears for the next cadence.

### Verification

Run focused plugin tests for world recheck.

Run the prior park policy and unpark tests to catch regressions:

```text
cargo test -p lisa-plugin world_recheck --no-fail-fast
cargo test -p lisa-plugin park_instead_of_churn --no-fail-fast
cargo test -p lisa-plugin agent_owned_block --no-fail-fast
```

Run strict plugin lint:

```text
cargo clippy -p lisa-plugin --all-targets -- -D warnings
```

### Commit

Use `lisa commit-ticket` with exactly:

- `crates/lisa-plugin/src/lib.rs`.

Message: `T-048-02-02: recheck world parks on scheduler cadence`.

## Step 5: full verification and cleanup

Run formatting:

```text
cargo fmt --all -- --check
```

Run whitespace validation:

```text
git diff --check
```

Run the complete workspace suite:

```text
cargo test --workspace --no-fail-fast
```

Run workspace checking or strict Clippy as time permits:

```text
cargo check --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

If concurrent uncommitted files cause an unrelated lint failure, identify exact
paths and ownership rather than editing them.

Inspect each ticket commit with `git show --stat` and exact path lists.

Inspect ordinary index and worktree status.

Ticket-owned source must be neither staged, modified, nor untracked.

Lisa-managed journal/ticket/work changes and unrelated ticket artifacts remain
untouched.

## Step 6: record implementation progress

Maintain `progress.md` in the private attempt directory.

Record:

- completed source units;
- exact focused and full test commands/results;
- isolated commit hashes;
- any deviation from this plan and its rationale;
- remaining work until Review.

Do not commit the private progress artifact; Lisa publishes admitted phase
artifacts through its own completion flow.

## Step 7: Review

Write `review.md` in the private attempt directory.

Cover:

- outcome and acceptance criteria;
- files changed;
- instruction/schema behavior;
- native check safety and eligibility;
- startup/cadence behavior;
- Unpark provenance and reseating behavior;
- test coverage and totals;
- commit ownership and repository hygiene;
- open concerns or limitations.

Write `review-disposition.json` with exactly:

```json
{"disposition":"pass","reason":null}
```

only if all ticket-owned source is committed and required verification passes.

If a real blocker remains, write the exact blocking shape required by the
assignment with a nonempty actionable reason.

After both Review artifacts exist, remain on this ticket and stop.
