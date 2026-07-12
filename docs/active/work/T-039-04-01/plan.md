# Plan: characterize deadline paths

## Implementation strategy

Implement one test-only source unit in `crates/lisa-plugin/src/lib.rs`. Build
fixtures from current state types, assert policy behavior, and avoid production
refactoring. Verify from a narrow shared test prefix through the workspace gate.

## Step 1: establish baseline

Run existing `lisa-plugin` deadline-related tests before editing source.

Verification:

- timeout, health, and stale tests pass on the unmodified production tree;
- environmental or pre-existing failures are recorded before implementation.

This establishes that characterization describes already-passing behavior.

## Step 2: acknowledgement characterization

Create a fixture timestamp and absolute deadline. Install a current lease, slot
reservation, pending-ack seat, and awaiting-human marker.

Invoke the injected check immediately before the deadline and verify no outcome,
no state change, and unchanged current authority.

Invoke exactly at the deadline and verify successor generation, revoked
predecessor authority, `Recovering` seat, `WaitingForExit` slot, and cleared
awaiting-human marker.

## Step 3: transition characterization

Create an expired `WaitingForExit` slot without a ticket, an unexpired matching
slot, and an expired `WaitingForClear` slot with recent activity.

Run `check_transition_timeouts()` and verify the expired exit becomes an idle
empty shell, the unexpired exit remains waiting, and active clear remains
waiting. Avoid any branch that needs native Zellij host linkage.

## Step 4: review characterization

Configure comfortably separated review and wind-down durations. Create running
Review threads past the phase budget: active, quiet/awaiting-human, and quiet
eligible.

Run `check_review_timeouts()` and verify only the eligible ticket enters
`finish_up_sent` and emits `FinishUpPromptSent`. Verify exempt threads remain
running and the awaiting marker remains.

If native pane I/O prevents the combined test, use existing test capture or
split the eligible action into the already-proven safe fixture pattern. Record
the deviation before proceeding.

## Step 5: health characterization

Create a running thread whose activity exceeds the configured health threshold
and mark its pane awaiting-human.

Run `evaluate_health()` and verify the cache and event record
`Healthy -> Stuck`, while the thread and awaiting marker remain. This pins the
intentional non-exemption for observational health.

## Step 6: session characterization

Create globally over-budget threads for active, hard-silent awaiting-human, and
hard-silent reclaimable cases. Install a current attempt and slot for only the
reclaimable case.

Run `check_session_timeouts()` and verify exactly one typed timeout action,
removal and fencing of that ticket, survival of both exemptions, and advisory
warning tracking for both surviving over-budget tickets. Do not assert exact
elapsed seconds because the method samples wall time.

## Step 7: stale characterization

Create an old-phase but recently active thread, a hard-silent awaiting-human
thread, and a hard-silent reclaimable thread. Install a current attempt and slot
for the last case.

Run `detect_stale_threads()` and verify exactly one typed reclaim action and
fencing. Verify the active thread remains despite phase age and the awaiting
thread plus marker remain.

## Step 8: format and narrow verification

Format the modified Rust file, then run:

```text
cargo test -p lisa-plugin characterizes_
```

Verify all six tests are discovered and pass without sleeps or unavailable host
calls. Ensure failure messages identify the relevant policy contract.

## Step 9: regression verification

Run:

```text
cargo test -p lisa-plugin
cargo test --workspace
```

Run `just check` if practical. Verify existing deadline behavior and all
workspace crates remain green.

## Step 10: inspect source diff

Use read-only status and diff commands. Verify only
`crates/lisa-plugin/src/lib.rs` is ticket-owned source, its diff adds tests only,
and machine-owned ticket/provenance paths are excluded. Confirm the ordinary
index has no ticket-owned entry.

## Step 11: commit source

Commit through Lisa's isolated transaction:

```text
lisa commit-ticket \
  --ticket-id T-039-04-01 \
  --message "test(plugin): characterize deadline policies" \
  --include crates/lisa-plugin/src/lib.rs
```

Verify success and that no ticket-owned source remains staged, modified, or
untracked.

## Step 12: artifacts and handoff

Write `progress.md` with completed steps, commands, results, and deviations.
Write `review.md` with the modified file, six policies, coverage, verification,
intentional lack of production changes, retained wall-clock limitations, and
open concerns.

Remain on this ticket after `review.md`. Do not edit ticket phase/status or start
a dependent ticket.

## Atomicity

The six tests form one meaningful ticket unit because acceptance requires the
complete matrix and all edits occupy the same shared file. A partial commit
would not satisfy the characterization bracket and would create unnecessary
conflict for the immediately dependent evaluator ticket.
