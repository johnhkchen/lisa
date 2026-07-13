# Plan — T-045-03-03 delivered awaiting claim

## Objective

Implement a real passive claim-wait transition for a delivered assignment in a live
Codex TUI.

The old acknowledgement deadline must not trigger a duplicate prompt.
The passive wait must accept current-attempt ownership evidence and end at a finite,
named, operator-actionable `ClaimTimedOut` state when no evidence arrives.

Preserve Claude and non-live delivery retry behavior.

## Step 1 — establish baseline

Run the most relevant current tests before editing:

```text
cargo test -p lisa-plugin delivered_assignment_becomes_owned_on_exact_claim_without_hook
cargo test -p lisa-plugin matching_hook_accelerates_pending_claim_ownership
cargo test -p lisa-plugin current_artifact_is_bounded_fallback_and_stale_evidence_is_ignored
cargo test -p lisa-plugin bounded_fresh_delivery
```

Record whether each baseline passes.

Inspect `git status --short` and retain the initial unrelated-worktree inventory.
Do not stage or alter Lisa-managed files.

Verification:

- predecessor evidence behavior is green;
- old Codex delivery retry behavior is reproduced before replacement;
- no ticket-owned source is dirty before implementation.

## Step 2 — add durable terminal vocabulary

Edit `crates/lisa-core/src/provenance.rs`.

Add `AssignmentState::ClaimTimedOut`.

If direct enum serialization coverage exists, extend it to assert
`"claim-timed-out"`.
Otherwise rely on scheduler provenance regression plus existing serde policy.

Run:

```text
cargo test -p lisa-core provenance
```

Verification:

- core compiles;
- old provenance records still round-trip;
- the new enum spelling is kebab-case.

## Step 3 — add dashboard projection vocabulary

Edit `crates/lisa-plugin/src/ui.rs`.

Add:

- `DeliveredAwaitingClaim`;
- `ClaimTimedOut`.

Map labels exactly:

- `delivered-awaiting-claim`;
- `claim-timed-out`.

Map the passive state yellow and the terminal state red.

Do not add elapsed-time inference or signal inspection to UI code.

Verification:

- exhaustive UI matching compiles once scheduler projection is added;
- existing labels remain unchanged.

## Step 4 — add private scheduler states and typed outcome

Edit `crates/lisa-plugin/src/lib.rs`.

Add `SeatAssignmentState::DeliveredAwaitingClaim` with generation and absolute
claim deadline.

Add terminal `SeatAssignmentState::ClaimTimedOut`.

Add `FailureTransitionOutcome::AssignmentClaimTimedOut`.

Extend `active_assignment_generation` with the passive state.

Extend `to_ui_state` with both projections.

Compile before adding behavior to find every exhaustive match:

```text
cargo check -p lisa-plugin
```

Verification:

- every state consumer makes an explicit choice;
- `ClaimTimedOut` is not treated as active ownership evidence;
- no behavior has yet been inferred by the UI.

## Step 5 — define live current Codex eligibility

Add a private read-only helper that validates the addressed delivery against the slot.

Require:

1. matching pane;
2. retained ticket ID;
3. `has_session` true;
4. `last_client == Codex`;
5. retained attempt lease;
6. lease ticket equals slot ticket;
7. lease attempt equals state generation;
8. exact current lease authority.

Add a focused unit assertion through the acceptance fixture rather than exposing the
helper publicly.

Verification:

- current live Codex qualifies;
- Claude does not qualify;
- stale or missing lease does not qualify;
- missing session does not qualify.

## Step 6 — implement passive transition

In `check_assignment_ack_timeouts_at`, include the passive state's claim deadline in
candidate extraction.

Before the existing `Delivering` retry branch, match a live current Codex delivery.

On its first expired deadline:

1. compute a new absolute deadline from injected `now` with
   `assignment_ack_deadline`;
2. insert `DeliveredAwaitingClaim` with the same generation;
3. log the passive transition;
4. perform no call to `deliver_assignment_to_pane` or `send_line_to_pane`;
5. push no failure outcome.

Leave the existing retry branch reachable for Claude and non-live deliveries.

Verification:

- the new deadline is later than the transition time;
- retry count one is not created for live Codex;
- current lease and reservation are unchanged;
- no session launch is created;
- no duplicate delivery activity is logged.

## Step 7 — implement terminal claim timeout

Add `fail_assignment_claim_wait` following existing retained-failure helpers.

It must:

1. accept only `DeliveredAwaitingClaim`;
2. insert `ClaimTimedOut` first;
3. resolve the ticket reservation;
4. mark the thread failed;
5. emit `AssignmentState::ClaimTimedOut` provenance;
6. add one error alert;
7. log an actionable pane inspection/reset message;
8. return `AssignmentClaimTimedOut`;
9. retain lease, slot, and thread records;
10. send no pane input.

Wire passive deadline expiry to this helper.

Verification:

- terminal state is exact-once;
- repeated timeout checks return no outcome;
- late ownership evidence is rejected;
- durable state is `claim-timed-out`, not `delivery-failed`.

## Step 8 — add acceptance regression

Add a scheduler test named along the ticket language, for example:

```text
live_codex_slow_claim_waits_without_reinjection_then_times_out_actionably
```

Fixture sequence:

1. create a Codex ticket and slot;
2. schedule it and advance the fresh launch into `Delivering`;
3. confirm the slot is live/current Codex;
4. confirm no claim or hook file exists;
5. capture delivery log count, session-launch count, and queued sends;
6. expire the first delivery deadline;
7. assert `DeliveredAwaitingClaim` and unchanged generation;
8. assert no new delivery log, launch, queued Enter, or failure outcome;
9. assert dashboard label is `DeliveredAwaitingClaim`;
10. expire the passive deadline;
11. assert typed `AssignmentClaimTimedOut` outcome;
12. assert terminal scheduler and dashboard states;
13. assert failed thread, retained reservation/lease, one alert, and actionable log;
14. assert provenance `ClaimTimedOut` and reason;
15. assert no `DeliveryFailed` record or duplicate timeout mutation.

The send assertion should observe actual scheduler send-side state, not only text.

## Step 9 — protect ownership while passive

Add or extend a focused test that constructs the passive state for the exact current
attempt, writes a valid claim, consumes it, and observes `Owned`.

Confirm:

- no hook is required;
- the claim is one-shot;
- activity is bumped;
- the later passive deadline is inert;
- no terminal alert or provenance row appears.

The existing hook/artifact tests continue to protect their paths because
`active_assignment_generation` is their common admission gate.
If practical, add a direct assertion for one supplemental path from the new state.

## Step 10 — update superseded Codex regressions

Run the plugin suite and identify failures that encode the old live-Codex retry path.

For each failure:

- update expectations only if the fixture has a live current Codex session;
- preserve non-live delivery failure characterization;
- preserve Claude retry behavior;
- keep startup grace behavior unchanged through initial `Delivering`;
- do not broadly rename unrelated tests.

Run focused filters after each adjustment:

```text
cargo test -p lisa-plugin claim
cargo test -p lisa-plugin delivery
cargo test -p lisa-plugin codex
cargo test -p lisa-plugin provenance
```

Verification:

- no test hides a duplicate send;
- previous claim/hook/artifact evidence tests remain green;
- Claude behavior remains explicitly covered.

## Step 11 — format and full verification

Run:

```text
cargo fmt --all
cargo fmt --all -- --check
cargo test -p lisa-core
cargo test -p lisa-plugin
cargo test --workspace
cargo check -p lisa-plugin --target wasm32-wasip1
git diff --check
```

If the repository's normal quick check adds coverage without redundant failure, run:

```text
just check
```

Verification criteria:

- every enabled test passes;
- WASM plugin check passes;
- formatting and whitespace checks pass;
- no ticket-owned warnings or TODOs remain;
- only the three planned source files are changed.

## Step 12 — progress and isolated commit

Write `progress.md` before committing.

Document:

- completed steps;
- exact semantic transition;
- test results;
- any deviations;
- intended exact include set.

Commit the meaningful source unit only with:

```text
lisa commit-ticket \
  --ticket-id T-045-03-03 \
  --message "feat(plugin): await delivered Codex assignment claims" \
  --include crates/lisa-core/src/provenance.rs \
  --include crates/lisa-plugin/src/lib.rs \
  --include crates/lisa-plugin/src/ui.rs
```

Do not use ordinary `git add`, `git commit`, or a broad include.

After commit, verify:

```text
git show --stat --oneline HEAD
git show --check HEAD
git status --short
```

Ticket-owned source must be clean.
Pre-existing Lisa-managed worktree entries may remain and must not be included.

## Step 13 — review

Inspect the committed diff and test evidence.

Review specifically for:

- a pane write hidden in the passive transition;
- a late-evidence resurrection path;
- accidental Claude behavior change;
- missing exhaustive state projection;
- false `DeliveryFailed` provenance;
- unbounded deadline behavior;
- lease or reservation release on timeout;
- duplicate terminal evidence.

Write:

- `review.md`;
- `review-disposition.json`.

Use pass only if all acceptance behavior and verification are complete.
Remain on T-045-03-03 after Review.
