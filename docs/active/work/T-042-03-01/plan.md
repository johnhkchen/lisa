# Plan: operator-requested authority emission

## Step 1: make operator provenance explicit

Add `OperatorRequestSource` beside completion source types.
Start with the `MarkDoneKey` variant.
Replace the unit Manual completion source with OperatorRequested carrying it.
Replace the ambiguous Manual adapter input with OperatorRequested carrying it.
Remove the optional authority field from the operator input.

Verification:

- Rust exhaustive matching identifies every affected branch.
- The operator input type has no AttemptLease or CompletionAuthority field.
- Other adapter input variants are untouched.

## Step 2: isolate canonical disposition evaluation

Extract canonical `review-disposition.json` parsing into a State helper.
Return the existing typed CompletionRejection on Block or Invalid.
Retain the authored block reason.
Retain invalid-disposition context.

Refactor `admit_passing_review` to call this helper after artifact admission.
Do not change current-lease checks or staged-to-canonical publication.

Verification:

- Existing automatic Review gate tests continue to pass.
- Missing and invalid canonical evidence remain fail-closed.
- Active-attempt admission still requires an exact current lease.

## Step 3: normalize OperatorRequested in the adapter

Map OperatorRequested to CompletionAuthority::Operator unconditionally.
Map its provenance to CompletionSource::OperatorRequested(source).
Give it no review lease.
Before reducing, evaluate the canonical Review disposition.
Log any rejection with the already-derived operator correlation.
Return without an effect on rejection.

Continue mapping operator authority to the stable reducer AttemptId `operator`.
Do not alter the pure reducer event or effect shape.

Verification:

- The dispatch match contains no operator route to Attempt authority.
- A blocked operator request produces no effect.
- The rejection contains ticket, named kind, correlation, and detail.

## Step 4: constrain executor and result authority

Update operator checks to require an OperatorRequested source.
Keep current-lease validation for Attempt authority.
Keep all missing/mismatched authority rejection behavior.
Keep dependency validation in the sole executor.
Update result validation to recognize the new source variant.

Verification:

- Scheduler sources cannot pair with Operator authority.
- Operator sources cannot pair with Attempt authority through typed dispatch.
- Unmet dependencies produce no pending completion or launch.

## Step 5: simplify `[d]one` construction

Remove `self.threads` lookup from `mark_ticket_done`.
Always dispatch OperatorRequested with MarkDoneKey source.
Do not alter modal eligibility or key handling.
Do not alter modal closing behavior in this ticket.

Verification:

- An active thread has no effect on emitted authority.
- No live thread is required to emit the request.
- The same source is retained in both cases.

## Step 6: revise active and orphaned tests

Update the active-thread test with canonical Pass evidence.
Retain an installed attempt lease and running thread.
Assert pending authority equals Operator.
Assert pending source equals OperatorRequested(MarkDoneKey).
Assert the effect's AttemptId equals `operator`, never the installed attempt ID.
Retain thread, slot, and ticket Review assertions.

Update the orphaned test with canonical Pass evidence.
Assert the same authority, source, and effect identity.
Retain the no-thread setup.

Verification:

- Both focused tests pass independently.
- Their expected effect identities are identical.

## Step 7: add gate regression

Build a native fixture containing an active blocked Review.
Write canonical Block disposition with an actionable reason.
Install an attempt and thread to prove they are not borrowed.
Invoke `mark_ticket_done`.
Assert no pending completion and no launched effect.
Assert correlated DispositionBlocked activity.

Build a second fixture ticket with an unfinished dependency.
Write canonical Pass disposition for the dependent Review.
Invoke `mark_ticket_done` without a thread.
Assert no pending completion and no launched effect.
Assert correlated DependencyBlocked activity.

Verification:

- The test name directly describes both acceptance gates.
- Each refusal is independently asserted by typed event kind.
- Neither refusal mutates ticket frontmatter.

## Step 8: format and targeted verification

Run `cargo fmt --all -- --check` after formatting if required.
Run the three focused mark-done tests.
Run existing Review-disposition completion tests.
Run lisa-plugin native tests if targeted verification is green.

Verification commands:

```text
cargo fmt --all
cargo test -p lisa-plugin test_mark_done -- --nocapture
cargo test -p lisa-plugin operator_requested -- --nocapture
cargo test -p lisa-plugin test_auto_complete_review_block_retains_assignment_with_visible_reason
cargo test -p lisa-plugin
```

## Step 9: broader verification

Run `cargo test --workspace` when the focused suite passes.
Run `cargo check -p lisa-plugin --target wasm32-wasip1` or `just check`
if available within the ticket window.
Record exact outcomes and any unrelated failures in progress and review.

Verification criteria:

- Native workspace compiles and tests pass.
- WASM target compiles if invoked.
- No new dependency or size impact is introduced.

## Step 10: ticket-scoped commit

Inspect the diff for only intended source changes.
Confirm unrelated working-tree changes are untouched.
Commit the meaningful source unit with:

```text
lisa commit-ticket --ticket-id T-042-03-01 \
  --message "fix(plugin): emit explicit operator completion requests" \
  --include crates/lisa-plugin/src/lib.rs
```

Do not use ordinary git add or git commit.
Confirm the ticket-owned source file is no longer modified or untracked.

## Step 11: implementation and review artifacts

Write `progress.md` with completed steps, tests, and deviations.
Write `review.md` with source changes, behavior, coverage, and concerns.
Write a valid `review-disposition.json`.
Use pass only when source is committed and verification supports completion.
Remain on this ticket after Review artifacts are present.

## Completion checklist

- OperatorRequested is an explicit typed adapter event.
- It carries MarkDoneKey source.
- It cannot carry Attempt authority.
- Active and orphaned Reviews emit identical operator identity.
- Blocking disposition refuses the request.
- Missing/invalid disposition fails closed.
- Unmet dependencies refuse the request.
- The sole executor remains the effect boundary.
- Ticket-owned source is committed through Lisa.
- No ticket-owned source changes remain in the working tree.
