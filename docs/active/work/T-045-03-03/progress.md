# Progress — T-045-03-03 delivered awaiting claim

## Status

Implementation is complete.

The live current Codex delivery timeout now transitions into a real
`DeliveredAwaitingClaim` scheduler state without sending another pane line.

The passive wait has one finite deadline.
If no current-attempt ownership evidence arrives, it transitions to the named retained
terminal state `ClaimTimedOut`.

The terminal state is operator-visible, durable in pre-ownership provenance, and
retrievable through the CLI status formatter.

All focused, package, workspace, formatting, whitespace, and WASM checks pass.

## Baseline

Before source edits, these focused predecessor tests passed:

```text
cargo test -p lisa-plugin delivered_assignment_becomes_owned_on_exact_claim_without_hook
cargo test -p lisa-plugin matching_hook_accelerates_pending_claim_ownership
cargo test -p lisa-plugin current_artifact_is_bounded_fallback_and_stale_evidence_is_ignored
cargo test -p lisa-plugin test_bounded_fresh_delivery_retries_once_then_fails_actionably
```

Each ran one test and passed.

The final test reproduced the old behavior:

- first delivery deadline triggered a chat retry;
- the retry deadline produced `DeliveryFailed`.

## Scheduler states added

`SeatAssignmentState` now contains:

```text
DeliveredAwaitingClaim {
    generation,
    claim_deadline,
}
```

This is an unowned active state.
Its generation participates in the same evidence admission helper as `Delivering`.

`SeatAssignmentState` also contains:

```text
ClaimTimedOut
```

This is a terminal retained state.
It carries no active generation and cannot accept late ownership evidence.

`FailureTransitionOutcome` now contains the typed completed outcome:

```text
AssignmentClaimTimedOut { pane_id, ticket_id }
```

## Live current Codex boundary

Added `is_live_codex_delivery`.

It requires:

- the addressed physical slot;
- a ticket reservation;
- `has_session == true`;
- `last_client == Codex`;
- a retained attempt lease;
- matching lease ticket and slot ticket;
- matching lease attempt and delivery generation;
- exact current lease authority.

Only this boundary replaces the active retry with passive waiting.

Claude and missing/stale/non-live delivery paths retain the prior retry/failure branches.

## Passive transition

`check_assignment_ack_timeouts_at` now includes `claim_deadline` in its existing generic
absolute-deadline evaluation.

On the first expired live current Codex `Delivering` deadline, the scheduler:

1. preserves the exact attempt generation;
2. computes a new finite deadline with `assignment_ack_deadline(now)`;
3. inserts `DeliveredAwaitingClaim`;
4. logs that the assignment is delivered and awaiting claim;
5. explicitly says the prompt is not being re-injected;
6. returns no failure outcome;
7. performs no pane send.

No retry count one is created on this path.
No attempt, process, lease, or reservation changes.

## Ownership evidence

`active_assignment_generation` recognizes `DeliveredAwaitingClaim`.

Consequently the predecessor evidence hierarchy remains intact:

1. an exact pane-routed claim owns;
2. a matching `UserPromptSubmit` hook can accelerate ownership;
3. an admitted exact-current-attempt artifact can establish bounded fallback ownership;
4. stale attempts remain rejected by existing lease checks.

The exact-claim regression now advances from `Delivering` into
`DeliveredAwaitingClaim` before publishing the valid claim.
It proves the state is visible on the dashboard and that the claim alone still reaches
`Owned` with no hook file.

The supplemental hook regression from the new passive state also passes.

## Terminal transition

Added `fail_assignment_claim_wait`.

The helper accepts only `DeliveredAwaitingClaim` and first inserts `ClaimTimedOut` as
the exact-once guard.

It then:

- resolves the retained ticket;
- marks the logical thread failed;
- emits `AssignmentState::ClaimTimedOut` provenance;
- adds one existing error alert;
- tells the operator to inspect the pane and reset the ticket;
- returns `AssignmentClaimTimedOut`.

It retains:

- the ticket reservation;
- the current attempt lease;
- the slot attempt lease;
- the thread record;
- the live pane session for inspection.

It performs no pane send, retry, relaunch, release, or automatic redispatch.

Repeated timeout evaluation is inert.
A late exact hook/claim cannot resurrect the terminal state.

## Durable and UI vocabulary

`lisa_core::provenance::AssignmentState` now includes `ClaimTimedOut`.

The existing kebab-case serde contract writes:

```text
claim-timed-out
```

The plugin UI now projects:

- `DeliveredAwaitingClaim` as yellow `delivered-awaiting-claim`;
- `ClaimTimedOut` as red `claim-timed-out`.

These labels are projections of real scheduler states.
No dashboard inference was added.

## Plan deviation

The Structure artifact identified three source files.

The first complete plugin-test compile surfaced an exhaustive match in:

`crates/lisa-cli/src/preownership_status.rs`

The CLI status formatter maps every `AssignmentState` to a stable operator name.
The new provenance variant therefore required a fourth source file.

The deviation is necessary and minimal:

- add `ClaimTimedOut => "claim-timed-out"`;
- add one direct formatter test.

No CLI command shape, parser, configuration, claim producer, or external behavior was
otherwise changed.

## Acceptance regression

Added:

```text
live_codex_slow_claim_waits_without_reinjection_then_times_out_actionably
```

It drives the production scheduler methods through:

```text
Starting
  -> Delivering
  -> DeliveredAwaitingClaim
  -> ClaimTimedOut
```

The fixture proves:

- the slot is a live current Codex session;
- claim and hook files are absent;
- the old delivery deadline returns no failure outcome;
- the generation remains exact;
- the new claim deadline is finite and later;
- delivery-log count does not increase;
- pending-Enter count does not increase;
- session-launch count does not increase;
- the dashboard shows `DeliveredAwaitingClaim`;
- passive expiry returns `AssignmentClaimTimedOut`;
- terminal state is `ClaimTimedOut`;
- the thread is failed but reservation and leases remain;
- the operator log says inspect and reset;
- no delivery-failure error is emitted;
- one provenance row uses `ClaimTimedOut` and the exact reason;
- later timeout checks add no second record.

## Historical regressions updated

Five tests encoded the old live-Codex retry behavior.

They now expect passive claim waiting where their fixtures satisfy the live current
Codex predicate:

- prompt miss;
- dropped hook event;
- bounded fresh delivery;
- ownership evidence during the second window;
- ten-ticket/two-pane consecutive reuse.

Non-live and Claude delivery-failure tests remain unchanged and green.

## Verification results

Focused plugin suite after implementation:

```text
cargo test -p lisa-plugin
```

Result:

- 393 passed before the CLI formatter test was added;
- 0 failed;
- 0 ignored.

Final repository quick check:

```text
just check
```

Result:

- `cargo check -p lisa-plugin --target wasm32-wasip1` passed;
- `cargo test --workspace` passed;
- plugin total 394 passed;
- core total 200 passed plus integration tests;
- CLI binary total 270 passed;
- CLI library total 19 passed;
- all enabled integration and doc tests passed;
- the environment-gated real Zellij test remained ignored as expected.

Additional checks passed:

```text
cargo fmt --all -- --check
cargo test -p lisa-core
cargo test -p lisa-cli preownership_status
cargo test --workspace
cargo check -p lisa-plugin --target wasm32-wasip1
git diff --check
```

## Ticket-owned source

The exact source include set is:

- `crates/lisa-cli/src/preownership_status.rs`;
- `crates/lisa-core/src/provenance.rs`;
- `crates/lisa-plugin/src/lib.rs`;
- `crates/lisa-plugin/src/ui.rs`.

These four files form one semantic state-machine unit.

No ordinary Git staging or commit command has been used.
The ordinary index is empty.

The next action is one isolated `lisa commit-ticket` call with those exact paths.
