# Plan: Gate completion on explicit pass

## Step 1: introduce the Review authorization helper

Import `parse_review_disposition` and `ReviewDisposition` from `lisa-core`.

Add `State::request_review_completion` adjacent to the lower-level completion
transaction method.

Implement lease-aware admission of `review-disposition.json`.
Refuse and log admission errors or missing evidence.
Parse only the successfully admitted canonical file.

Match the typed outcome exhaustively:

- Pass delegates to `request_completion`;
- Block logs its actionable reason and returns false;
- Invalid logs its diagnostic reason and returns false.

Verification: run `cargo check -p lisa-plugin` after the helper compiles.

## Step 2: route every automated Review edge through the helper

Replace the direct call in `check_artifact_advances` when Review's next phase is
Done.

Replace the same-cycle Review catch-up call in `check_idle_signals` after
Implement transitions to Review.

Replace the direct Review branch in `check_idle_signals` when its next phase is
Done.

Replace `auto_complete_review`'s direct transaction request.

Preserve the source enum and current attempt lease at each call site.
Do not alter manual operator or observed-Done requests.

Verification: search all `request_completion` calls and classify each remaining
direct call by manual, reconciliation, helper delegation, or focused test.

## Step 3: add the block/pass/invalid artifact regression

Create a table-driven test around `check_artifact_advances`.
For each case, construct a fresh temporary two-ticket DAG where the dependent
names the Review ticket in `depends_on`.

Install a running Review thread, assigned slot, and current lease.
Write `review.md` and the case-specific `review-disposition.json` in the
attempt-private work directory.

Drive the real polling method.

For block, verify retained thread/assignment/lease, no pending request, Review
on disk, dependent prerequisites incomplete, and visible actionable reason.

For pass, verify one pending request with unchanged Artifact source/authority,
Review retained until transaction result, and canonical disposition admission.

For invalid, verify the same safe retained state as block and a visible refusal
diagnostic.

Verification: run the new test by exact name.

## Step 4: lock the stopped-session site

Update the positive `auto_complete_review` test to configure a work directory
and write a passing disposition under the installed current attempt.

Add a block test that writes an actionable refusal, calls
`auto_complete_review`, and asserts no pending transaction while thread and slot
remain assigned and the warning contains the block reason.

The artifact-poll table already covers invalid input at the shared helper; the
stopped test establishes that this named caller uses that helper.

Verification: run plugin tests filtered by `auto_complete_review`.

## Step 5: update all existing positive fixtures

Search for every `review.md` write and every call to `check_artifact_advances`,
`handle_stopped_signal`, or `auto_complete_review` that expects a pending
completion.

Add the exact passing document to the corresponding current attempt directory.
Do not add dispositions to tests whose expected behavior is refusal or whose
focus is a lower-level direct `request_completion` call.

Pay special attention to:

- Codex full artifact progression;
- Codex stopped-session completion;
- split-brain replacement completion;
- verified artifact completion publication;
- attempt admission and canonical publication regression fixtures.

Verification: run `cargo test -p lisa-plugin` and address only failures caused
by the new explicit contract.

## Step 6: inspect behavior and formatting

Run `cargo fmt --all` to format the source unit.

Inspect the scoped diff for:

- a sole `Pass` branch delegating to `request_completion` in the Review helper;
- complete Block and Invalid logging;
- no default-to-pass condition;
- no changes to manual completion authority;
- exact use of current attempt artifact admission.

Run `git diff --check -- crates/lisa-plugin/src/lib.rs`.

## Step 7: run verification suite

Run focused tests first:

```text
cargo test -p lisa-plugin review_disposition
cargo test -p lisa-plugin auto_complete_review
```

Run the complete plugin suite:

```text
cargo test -p lisa-plugin
```

Run the workspace suite:

```text
cargo test --workspace
```

Run the WASM check:

```text
cargo check -p lisa-plugin --target wasm32-wasip1
```

If a failure is unrelated and belongs to concurrent work, record the evidence
without modifying its files. Any ticket-owned failure must be resolved before
commit.

## Step 8: commit the meaningful source unit

Review `git status --short` and preserve all existing Lisa-managed or
other-ticket changes.

Commit only the plugin source unit through the isolated transaction:

```text
lisa commit-ticket \
  --ticket-id T-040-01-03 \
  --message "Gate review completion on explicit pass" \
  --include crates/lisa-plugin/src/lib.rs
```

Do not use ordinary `git add`, `git add -A`, or `git commit`.

After commit, inspect the created commit and confirm the source path has no
ordinary-index staged entry and no remaining worktree modification.

## Step 9: write implementation progress

Create `progress.md` in the private attempt work directory.
Record the helper behavior, all routed callers, test coverage, verification
results, commit identity, and any deviation from this plan.

The progress artifact is not part of the ticket-owned source commit.

## Step 10: Review and disposition

Re-read the acceptance criterion and inspect the committed diff.
Verify block, pass, and invalid outcomes against every required invariant.

Write `review.md` with change inventory, test evidence, coverage gaps, open
concerns, and ownership hygiene.

Write exactly one machine-readable outcome:

```json
{"disposition":"pass","reason":null}
```

Use a block disposition instead only if a concrete unresolved issue prevents
safe completion.

After both Review artifacts exist, remain on T-040-01-03 and stop. Do not edit
the ticket phase/status or publish Done manually.
