# T-034-01-03 Progress — revoke and fence before reschedule

## Status

Implementation is complete and the ticket-owned source change is committed
through Lisa's isolated transaction.

Review remains the only workflow phase after this progress record.

The ticket frontmatter phase and status were not edited.

## Source commit

```text
95bd8efa5360a5c6bdc5084308b068e4835459b7
fix: revoke and fence timed-out attempts
```

The isolated commit contains exactly:

```text
crates/lisa-plugin/src/lib.rs
```

Command used:

```text
cargo run -q -p lisa-cli -- commit-ticket \
  --ticket-id T-034-01-03 \
  --message "fix: revoke and fence timed-out attempts" \
  --include crates/lisa-plugin/src/lib.rs
```

The repository's current CLI package was used so the transaction matched the
workflow's `commit-ticket` implementation.

The source path is clean after the commit.

The ordinary Git index remained empty before and after the transaction.

Unrelated pre-existing working-tree modifications and untracked paths were not
included or changed.

## Step 1 — baseline focused behavior

Completed.

Before editing, the existing tests passed:

```text
cargo test -p lisa-plugin dispatch_mints_and_stamps_strictly_new_attempt_lease
1 passed; 0 failed

cargo test -p lisa-plugin test_check_session_timeouts_expired
1 passed; 0 failed

cargo test -p lisa-plugin test_detect_stale
6 passed; 0 failed
```

This established that later behavior changes came from this ticket.

## Step 2 — split lease history from authority

Completed.

`State::current_leases` now means only currently authorized attempts.

Added `State::lease_high_water` for the latest successfully minted attempt per
ticket in the current plugin process.

Dispatch now passes `lease_high_water[ticket]` to `AttemptLease::mint`.

After successful minting, dispatch stores the exact same lease in:

- `lease_high_water`;
- `current_leases`;
- the physical `AgentSlot`;
- the logical `Thread`.

The insertions remain before provider launch, input, assignment, or thread
creation side effects.

## Step 3 — central release revocation

Completed.

Added `revoke_current_lease`, which removes active authority without removing
the monotonic predecessor.

`release_slot_for_ticket` calls it at method entry.

This establishes the shared invariant that no release caller can make a ticket
reschedulable while its prior lease remains current.

Repeated removal is harmless, which allows the timeout path to revoke earlier
for its stricter ordering contract.

The prior dispatch test now proves that release:

- clears the slot stamp;
- removes current authority;
- causes attempt 1 to fail `is_current`;
- retains attempt 1 only in high-water history.

Redispatch still mints attempt 2 and installs it consistently everywhere.

## Step 4 — terminal fence state

Completed.

Added private `TransitionState::Fenced`.

The state means Lisa closed the terminal pane after a hard-silent attempt.

It is terminal:

- no transition deadline is set;
- no retry timer is armed;
- no transition fallback processes it;
- `find_slot_for_client` excludes it because selection requires `Idle`.

Added private `FenceOutcome` with bounded results:

- `Fenced { pane_id }`;
- `AlreadyFenced { pane_id }`;
- `NoAssignedPane`.

No result creates a retry loop.

## Step 5 — production pane termination

Completed.

Added `close_fenced_pane` as the narrow host boundary.

Production/WASM builds call Zellij's `close_terminal_pane(pane_id)`.

Native unit tests skip the host call and observe scheduler state/order instead.

The deployed target compiled successfully with the real Zellij API.

## Step 6 — ordered revoke-and-fence helper

Completed.

Added `State::revoke_and_fence_attempt`.

Its normal path performs:

1. current lease removal;
2. physical slot lookup;
3. transition to `Fenced`;
4. resident-session/client/cooldown clearing;
5. pane assignment and debounce cleanup;
6. queued Enter removal for the pane;
7. terminal-pane close request;
8. bounded fence logging/result.

The slot's ticket and lease stamps stay present until the subsequent shared
release operation, preserving the explicit fence-before-release boundary.

If the pane is already fenced, the helper returns a named idempotent result.

If no pane is assigned, authority remains revoked, the inconsistency is logged,
and teardown continues without retrying indefinitely.

## Step 7 — fence-aware release

Completed.

For an ordinary slot, release preserves existing behavior:

- resident provider session can remain alive;
- wind-down cooldown is applied;
- the pane receives an idle name;
- the seat assignment is removed.

For a fenced slot, release:

- clears ticket and lease stamps;
- preserves `TransitionState::Fenced`;
- keeps the slot non-resident;
- applies no cooldown;
- skips renaming the closed pane;
- removes its seat assignment.

The fenced pane ID can therefore never be selected for later work.

## Step 8 — strict-order test observation

Completed.

Added test-only `AttemptLifecycleEvent` and a test-only trace on `State`.

The trace records:

- successful lease revocation;
- completed pane fence transition;
- completed slot release.

Production builds contain no trace field or event storage.

## Step 9 — session budget timeout integration

Completed.

`check_session_timeouts` keeps the existing two-part reclaim gate:

- configured global/per-phase budget exceeded;
- silence at least `2 * stuck_threshold_secs`.

Over-budget active sessions still only warn.

Awaiting-human sessions remain exempt.

For hard-silent attempts, the method now calls
`revoke_and_fence_attempt` before `release_slot_for_ticket`.

Existing failed-thread, timed-out provenance, thread removal, alert, and
`SessionTimedOut` activity behavior remains intact.

The method documentation now accurately states that the pane is closed at the
hard-silence boundary.

## Step 10 — pure stale timeout integration

Completed.

`detect_stale_threads` now uses the same revoke/fence helper before release.

It retains its existing `RunOutcome::Failed`, error message, hard-silence
threshold, and awaiting-human exemption.

Both code paths that reclaim on hard silence now share the physical and lease
teardown boundary.

## Step 11 — acceptance scheduler test

Completed by strengthening `test_check_session_timeouts_expired`.

The test builds:

- a real ticket file and DAG;
- attempt 1 in current and high-water state;
- matching thread and slot stamps;
- a running thread beyond budget and hard silence;
- an assigned first pane;
- a second eligible pane for successor dispatch.

After timeout it asserts the exact trace:

```text
LeaseRevoked(T-001)
PaneFenced(T-001, pane 1)
SlotReleased(T-001)
```

It also proves:

- attempt 1 is no longer current;
- attempt 1 remains the high-water predecessor;
- pane 1 is unassigned, unstamped, non-resident, and `Fenced`;
- the old thread is removed;
- the named timeout alert and structured activity remain present.

The test then drives the real scheduler and proves:

- pane 1 remains fenced and unused;
- pane 2 receives the ticket;
- attempt 2 is strictly greater than attempt 1;
- attempt 2 is current;
- high-water, current, slot, and thread all carry attempt 2.

## Step 12 — stale and release regressions

Completed.

`test_detect_stale_threads` now installs a real lease and proves pure stale
reclaim removes current authority, retains high-water history, and leaves the
pane `Fenced`.

The existing release, completion, pane-name, cooldown, Claude reuse, Codex
reuse, and mixed-provider tests all continue to pass, covering ordinary
non-timeout release behavior.

## Step 13 — focused verification

Completed.

Passed:

```text
cargo test -p lisa-plugin dispatch_mints_and_stamps_strictly_new_attempt_lease
1 passed; 0 failed

cargo test -p lisa-plugin test_check_session_timeouts_expired
1 passed; 0 failed

cargo test -p lisa-plugin test_detect_stale_threads
2 passed; 0 failed (substring also matches active-session companion)

cargo test -p lisa-plugin
268 passed; 0 failed
```

Formatting passed:

```text
cargo fmt --all -- --check
```

## Step 14 — broad verification

Completed.

The workspace suite passed:

```text
cargo test --workspace
```

Results:

- Lisa CLI: 270 unit tests passed;
- atomic provider contract: 1 integration test passed;
- Lisa core: 155 unit tests passed;
- Lisa plugin: 268 unit tests passed;
- doc tests: 0 failures.

Total: 694 tests passed, 0 failed.

The repository quick check passed:

```text
just check
```

This checked the deployed WASM plugin and reran repository tests.

The deployed target passed explicitly:

```text
cargo check -p lisa-plugin --target wasm32-wasip1
```

Warning-denied plugin library Clippy passed:

```text
cargo clippy -p lisa-plugin --lib -- -D warnings
```

Whitespace verification passed:

```text
git diff --check
```

## Step 15 — final diff inspection

Completed before commit.

The ticket source diff contained only:

```text
crates/lisa-plugin/src/lib.rs
```

The source diff was reviewed for:

- high-water/current authority confusion;
- timeout sequencing;
- fenced-slot eligibility;
- pending Enter cleanup;
- normal resident-session release behavior;
- frontmatter edits;
- unrelated file overlap.

No ticket frontmatter edit was made.

## Deviations from plan

### Acceptance test reused the existing timeout test name

The plan allowed strengthening or replacing the existing timeout test. The
implementation strengthened `test_check_session_timeouts_expired` rather than
adding another nearly duplicate fixture.

This keeps coverage concentrated and leaves the plugin test count unchanged
while materially expanding the assertions.

### Fence result is represented primarily by slot state in acceptance coverage

The helper returns `FenceOutcome`, but the timeout caller does not branch on the
result because every outcome is bounded and authority is already revoked.

The acceptance test asserts the durable named state `TransitionState::Fenced`
and exact lifecycle trace rather than exposing or storing the transient return
value.

### Closed panes are not automatically replaced

This matches Design. The one-shot slot discovery model has no safe replacement
mechanism. A closed pane permanently reduces scheduler capacity, while a second
eligible pane permits immediate redispatch in the acceptance test.

No automatic pane recreation was added because it would expand this ticket into
layout lifecycle management.

## Remaining work

Only Review remains:

- re-read committed source and test diff;
- summarize acceptance coverage;
- document operational tradeoffs and open concerns;
- write `review.md`.

Lisa owns phase/status transitions, final Done publication, artifact commit,
and seat release after Review.
