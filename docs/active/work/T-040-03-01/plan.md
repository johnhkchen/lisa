# Plan: pin T-039-06-02 as a regression

## Objective

Add and verify one deterministic plugin test proving that an explicit blocking
Review cannot prepare or publish Done, cannot release its assignment, and
cannot unblock downstream work.

## Step 1: establish the source baseline

Inspect the existing Review-disposition tests and current worktree state.

Verification:

- identify the generic block/pass/invalid test;
- identify existing helpers for attempt leases and disposition files;
- confirm `crates/lisa-plugin/src/lib.rs` is clean before editing;
- record unrelated dirty paths and leave them unchanged.

Atomicity: read-only; no commit.

## Step 2: add the historical regression fixture

Modify `crates/lisa-plugin/src/lib.rs` in the test module.

Create a temporary two-ticket repository:

- reviewed ticket already in Review;
- ready dependent with `depends_on` pointing to the reviewed ticket.

Build the real DAG and scheduler state. Configure a temporary work directory
and provenance ledger path.

Add one assigned slot and one running Review thread. Install the current
attempt lease using the existing helper.

Write attempt-private `review.md` and a valid block disposition with an
actionable reason.

Invoke `check_artifact_advances`.

Atomicity: source edit remains uncommitted until verification.

## Step 3: assert the historical failure discriminator

Assert `pending_completions` does not contain the reviewed ticket.

Use a failure message that explains the pre-T-040-01-03 unconditional path
would have inserted the pending completion despite the block disposition.

This assertion is essential. Do not substitute only ticket file or ledger
checks, because the asynchronous command has not produced a result during the
native scheduler call.

Verification: run the filtered test and confirm the assertion passes against
current production code.

## Step 4: assert retained ownership

Check the reviewed thread remains:

- present;
- `Phase::Review`;
- `ThreadStatus::Running`.

Check the slot remains assigned to the reviewed ticket and retains the same
attempt lease. Check the `current_leases` map retains the same lease.

Verification: these assertions pass in the filtered test and would catch any
partial cleanup/refencing behavior.

## Step 5: assert no Done publication or provenance

Read the reviewed ticket file and check both `status: review` and
`phase: review` remain.

Assert the configured provenance ledger was never created. The fixture begins
without a ledger and no other path emits a row, so nonexistence proves there is
no authoritative Done provenance.

Verification: filtered test passes without filesystem residue outside its
temporary directory.

## Step 6: assert downstream safety and actionability

Use `Dag::all_dependencies_done` for the dependent ticket and require false.
Also require the dependent has no runtime thread.

Inspect the activity log for the exact blocking reason and the
`Completion blocked` diagnostic.

Verification: the dependent remains unscheduled and the retained assignment
has an operator-visible explanation.

## Step 7: format and run focused verification

Run repository formatting for the edited Rust source.

Run the specifically named test:

```text
cargo test -p lisa-plugin test_t039_06_02_blocking_review_never_prepares_done
```

Expected result: one test passes with no failures.

Run the neighboring policy tests:

```text
cargo test -p lisa-plugin review_disposition
```

Expected result: existing generic disposition coverage and the new regression
all pass.

If formatting changes unrelated ticket-owned lines only mechanically within
the shared source file, inspect the exact diff before continuing.

## Step 8: run broad verification

Run:

```text
cargo test -p lisa-plugin --lib
cargo test --workspace
cargo check -p lisa-plugin --target wasm32-wasip1
```

Acceptance:

- all plugin library tests pass;
- all workspace tests and doc tests pass;
- plugin compiles for the deployed WASM target;
- no unrelated file is changed by the verification commands.

If a failure is unrelated or environmental, diagnose it and record the exact
evidence in `progress.md`. Do not weaken the regression.

## Step 9: inspect source diff and ownership

Use scoped Git status and diff commands.

Confirm:

- only the intended test was added to the ticket-owned source path;
- no production behavior changed;
- no unrelated dirty path entered the source diff;
- the ordinary index has no ticket-owned staged entry.

Do not clean or modify unrelated Lisa-managed paths.

## Step 10: write implementation progress

Create `progress.md` in the attempt-private work directory.

Record:

- completed test fixture and assertions;
- focused and broad test results;
- formatting result;
- any deviation from this plan;
- exact source transaction command and resulting commit after it runs;
- remaining Lisa-owned artifact publication.

The source unit is ready to commit only after verification succeeds.

## Step 11: commit the source unit

Use the current repository CLI if the installed `lisa` binary lacks the
required subcommand. Invoke only the isolated transaction:

```text
lisa commit-ticket \
  --ticket-id T-040-03-01 \
  --message "Pin blocking Review completion regression" \
  --include crates/lisa-plugin/src/lib.rs
```

Do not use `git add`, `git add -A`, or ordinary `git commit`.

After the command, verify:

- the commit includes exactly `crates/lisa-plugin/src/lib.rs`;
- that path is neither staged, modified, nor untracked;
- unrelated dirty paths remain preserved.

## Step 12: Review handoff

Write `review.md` summarizing:

- the dedicated historical fixture;
- the pre-fix discriminator;
- assignment, provenance, and dependency assertions;
- files changed;
- verification evidence;
- source transaction and cleanliness;
- coverage limitations or open concerns.

Write `review-disposition.json` with exactly:

```json
{"disposition":"pass","reason":null}
```

Use block only if a concrete unresolved implementation or verification issue
remains, and include a nonempty actionable reason.

After both Review artifacts exist, remain on T-040-03-01. Do not edit the
ticket, publish canonical artifacts, invoke completion directly, or begin
another ticket. Lisa owns final admission and completion confirmation.
