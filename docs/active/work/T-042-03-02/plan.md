# Plan: operator modal durable confirmation

## Step 1: introduce modal outcome vocabulary

Add the private operator modal outcome enum in `lib.rs`.

Add the public UI outcome enum in `ui.rs`.

Use Pending, Accepted, and Rejected variants.

Carry ticket and correlation in every variant.

Carry named kind and detail in Rejected.

Add the optional outcome field to both modal structs.

Update explicit modal constructors and test literals with None.

Verification:

- `cargo check -p lisa-plugin --tests` reaches later behavior errors rather
  than missing-field errors;
- default modal construction still yields no outcome.

## Step 2: project named rejections into the open operator modal

Add a matching helper on `State` for modal rejection display.

Match only open MarkDone modals for the same selected/outcome ticket.

Call it from `log_completion_rejection` after exact kind/detail mapping.

Preserve the existing correlated Activity event unchanged.

Ensure an immediate `AlreadyPending` reducer rejection reaches the helper.

Ensure blocked disposition, blocked dependencies, and launch failure use the
same helper automatically.

Verification:

- existing named completion activity tests continue passing;
- new state test observes named kind, exact correlation, and detail in the
  modal;
- unrelated ticket rejections do not replace current modal feedback.

## Step 3: keep accepted-for-processing requests visibly pending

Update `mark_ticket_done` without creating a second dispatch path.

Call `dispatch_completion(OperatorRequested)` exactly once.

When it returns true, read the pending transaction's completion generation.

Set the matching modal to Pending with that correlation.

When it returns false, retain any Rejected state set by the projection helper.

Never close the MarkDone modal in this action.

Verification:

- the current operator authority assertions still pass;
- Enter leaves the modal open;
- outcome is Pending and matches the pending transaction key;
- no additional completion effect is launched.

## Step 4: define outcome-specific keyboard behavior

Branch on MarkDone outcome before list navigation.

While Pending, ignore Enter, Esc, q, and navigation.

This prevents duplicate submission and premature loss of unresolved feedback.

After Accepted or Rejected, let Enter, Esc, or q close explicitly.

Retain pre-submission cancellation behavior.

Retain ResetTicket close-after-action behavior.

Retain QuitConfirm's dedicated handling.

Verification:

- Pending cannot be closed or resubmitted by Enter;
- terminal feedback closes only from explicit acknowledgement;
- existing ResetTicket modal tests pass.

## Step 5: project durable acceptance

Add a matching helper for Accepted state.

Call it from `handle_completion_result` only for operator pending source.

Place it after the persisted Confirmed journal transition.

Use the stable completion generation string as correlation.

Do not announce Accepted before durable Done verification.

Do not announce Accepted if confirmation persistence fails.

Leave unresolved failures in Pending when the result handler intentionally
retains pending state.

Verification:

- successful operator completion shows Accepted with the original correlation;
- thread/slot release and provenance remain unchanged;
- invalid result shows Rejected rather than Accepted;
- result persistence failure leaves Pending.

## Step 6: render status modal

Add modal text wrapping for correlation and detail.

Render Pending, Accepted, and Rejected with distinct labels/colors.

Render ticket and full correlation for every state.

Render exact rejection kind and full detail for Rejected.

Render a waiting footer for Pending.

Render explicit close controls for terminal outcomes.

Dispatch MarkDone outcome state to this renderer before the selection layout.

Verification:

- renderer test finds the ticket and correlation in every outcome;
- renderer test finds `already-pending` and its detail;
- long values wrap within the configured modal width;
- existing modal title tests still pass.

## Step 7: prove persistence across a poll

Extend the key-handler operator completion test.

Open the modal and submit the selected Review ticket.

Capture the pending correlation.

Assert the modal remains open and Pending.

Invoke `poll_tick` once.

Assert the modal is still open and the exact outcome is unchanged.

This test exercises the real periodic work boundary without a WASM host.

Verification:

- poll does not clear modal state;
- poll does not launch a duplicate operator effect;
- ticket remains Review while completion is unresolved.

## Step 8: prove already-pending and result rejection feedback

Build an operator pending transaction for a selectable Review ticket.

Open MarkDone and submit the same ticket again.

Assert Rejected AlreadyPending.

Assert full correlation and named detail render.

For a launched operator request, send a failing command result.

Assert Rejected LaunchFailed with stderr detail and the original correlation.

Assert the modal remains open until explicit acknowledgement.

Verification:

- no silent close occurs in either rejection timing;
- no duplicate launch occurs for already-pending;
- Activity still contains the same named correlated event.

## Step 9: prove successful acceptance feedback

Use a durable-journal fixture or adapt the operator retry regression.

Open and submit through the modal.

Update the fixture ticket to Done as the command would.

Feed a successful commit-shaped result.

Assert the modal remains open with Accepted and the exact request correlation.

Assert Enter or Esc explicitly closes it.

Retain assertions for slot release, dependent readiness, and single
provenance.

Verification:

- Accepted is only visible after durable confirmation;
- accepted feedback survives the result-triggered DAG rebuild;
- dismissal is operator-driven.

## Step 10: focused verification

Run formatter:

```text
cargo fmt --all -- --check
```

Apply formatting if needed with `cargo fmt --all`.

Run MarkDone state tests:

```text
cargo test -p lisa-plugin test_mark_done -- --nocapture
```

Run operator result tests:

```text
cargo test -p lisa-plugin operator_completion -- --nocapture
```

Run modal renderer tests:

```text
cargo test -p lisa-plugin modal -- --nocapture
```

Run named rejection tests:

```text
cargo test -p lisa-plugin completion_rejections -- --nocapture
```

Correct implementation defects rather than weakening assertions.

## Step 11: broad verification

Run the complete plugin tests:

```text
cargo test -p lisa-plugin --lib
```

Run the workspace tests:

```text
cargo test --workspace
```

If concurrent unrelated edits make a workspace failure unrelated, record the
exact test and evidence in progress/review.

Check the diff for accidental core, ticket, or shared artifact changes.

Check ordinary index state without modifying it.

## Step 12: commit the meaningful source unit

Commit the coupled plugin state and UI change through Lisa:

```text
lisa commit-ticket \
  --ticket-id T-042-03-02 \
  --message "fix(plugin): keep operator completion outcome visible" \
  --include crates/lisa-plugin/src/lib.rs \
  --include crates/lisa-plugin/src/ui.rs
```

Do not include unrelated dirty paths.

Do not use ordinary `git add` or `git commit`.

After commit, verify both ticket-owned source paths are clean.

## Step 13: implementation and review artifacts

Write `progress.md` with completed steps, tests, commit, and deviations.

Write `review.md` with file summary, behavior, test coverage, and concerns.

Write valid `review-disposition.json`.

Use pass only if source is committed and the acceptance criterion is covered.

Remain on T-042-03-02 after Review.

Do not edit ticket phase/status or publish shared artifacts.
