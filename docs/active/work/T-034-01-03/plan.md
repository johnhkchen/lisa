# T-034-01-03 Plan — revoke and fence before reschedule

## Implementation strategy

Implement the authority split first, then the fence state/helper, then route
both hard-silence paths through the ordered boundary, and finally strengthen
the scheduler tests.

The source work is one coherent commit because the intermediate forms are not
safe independently: separating registries without release revocation leaves a
behavior gap, while revocation without high-water storage breaks monotonic
redispatch.

## Step 1 — baseline the focused behavior

Run the existing focused tests before editing:

```text
cargo test -p lisa-plugin dispatch_mints_and_stamps_strictly_new_attempt_lease
cargo test -p lisa-plugin test_check_session_timeouts_expired
cargo test -p lisa-plugin test_detect_stale
```

Record any baseline failures before attributing them to this ticket.

Verification criteria:

- existing dispatch monotonicity passes;
- existing timeout teardown passes;
- existing stale-thread coverage passes or its exact available test names are
  identified through the test runner.

## Step 2 — split lease history from authority

In `State`, add `lease_high_water` and narrow `current_leases` documentation to
active authority only.

In `schedule_ready_tickets`:

1. read the predecessor from `lease_high_water`;
2. mint once;
3. insert the lease into `lease_high_water`;
4. insert the same lease into `current_leases`;
5. retain existing slot/thread stamps.

Add `revoke_current_lease` to remove only active authority.

Verification criteria:

- first dispatch writes identical leases to both maps;
- all existing provider dispatch branches compile;
- mint failure still occurs before pane side effects;
- high-water remains the sole predecessor source.

## Step 3 — make release revoke centrally

Call `revoke_current_lease` at the start of
`release_slot_for_ticket`.

Leave high-water state untouched.

Update the prior dispatch test so release proves:

- current authority is absent;
- attempt 1 fails current validation;
- high-water retains attempt 1;
- slot stamp is absent.

Redispatch and prove attempt 2 is installed in both maps and both assignment
stamps.

Verification criteria:

- no release path retains active authority;
- normal resident-session release still uses cooldown and idle rename;
- redispatch remains strictly monotonic.

## Step 4 — add terminal fence state and result

Add `TransitionState::Fenced`.

Add `FenceOutcome` with named terminal results.

Update exhaustive matches so `Fenced` is a no-op and never enters timeout
fallback logic.

Confirm slot selection remains limited to `Idle`.

Verification criteria:

- a fenced slot is not returned by either compatible or recycle selection;
- no timer/deadline is attached to `Fenced`;
- no automatic state transition leaves `Fenced`.

## Step 5 — add host close wrapper

Add a narrow close wrapper that calls `close_terminal_pane` in production and
does nothing in native unit tests.

Keep the conditional compilation local to the wrapper.

Verification criteria:

- native plugin tests can exercise fencing without a Zellij host;
- the WASM target resolves and compiles the real close API;
- the wrapper takes only the physical pane ID.

## Step 6 — implement ordered revoke-and-fence helper

Add `revoke_and_fence_attempt`.

Implement exact ordering:

1. revoke current lease;
2. locate the assigned slot;
3. if already fenced, return its named terminal outcome;
4. mark the slot fenced and non-resident;
5. clear pane-scoped assignments, queued Enter, awaiting, and attention state;
6. request pane close;
7. log the fence;
8. return `Fenced { pane_id }`.

Keep the slot ticket/lease stamps until shared release clears them.

For missing slot state, return `NoAssignedPane` after revocation and log the
condition. Do not retry.

Verification criteria:

- authority is absent for every result;
- a normal fence requests close once;
- a repeated fence does not request another close;
- the slot reaches `Fenced` before release;
- queued input for the closed pane is removed.

## Step 7 — make release fence-aware

Branch cleanup based on whether the matching slot is `Fenced`.

For fenced slots:

- clear ticket and attempt stamps;
- preserve `Fenced`;
- preserve non-resident state;
- do not add cooldown;
- do not rename the closed pane;
- remove seat assignment;
- log bounded fenced release.

For all other slots, retain current release behavior.

Verification criteria:

- fenced pane IDs can never be selected after ticket release;
- ordinary completed sessions remain reusable;
- release remains safe when no slot exists;
- authority revocation is idempotent.

## Step 8 — instrument ordering in tests

Add `AttemptLifecycleEvent` and `State::attempt_lifecycle` under `cfg(test)`.

Record:

- successful active-lease removal;
- completed pane fence state transition;
- completed slot release.

Do not expose the trace publicly or retain it in production.

Verification criteria:

- exact event equality can assert strict order;
- ordinary test state construction remains `Default` compatible;
- production/WASM state layout does not contain the trace.

## Step 9 — route session timeout through the fence

In `check_session_timeouts`, call `revoke_and_fence_attempt` after provenance
capture and before `release_slot_for_ticket`.

Retain existing thread failure, removal, timeout alert, and activity semantics.

Update documentation to state that hard-silent panes are closed.

Verification criteria:

- over-budget but active sessions remain untouched;
- awaiting-human sessions remain exempt;
- hard-silent sessions revoke, fence, release, and remove;
- timeout outcome remains named in `timeout_alerts` and `SessionTimedOut`.

## Step 10 — route pure stale detection through the fence

In `detect_stale_threads`, call the same helper before release.

Retain its existing failed provenance and error activity semantics.

Verification criteria:

- stale detection cannot preserve a live old pane;
- stale detection cannot preserve current lease authority;
- awaiting-human stale exemption remains intact;
- both hard-silence paths share one teardown implementation.

## Step 11 — implement the acceptance scheduler test

Strengthen `test_check_session_timeouts_expired` or replace it with a named test
focused on the acceptance contract.

Fixture setup:

1. create a real open ticket file and DAG;
2. configure a short session timeout and hard-silence threshold;
3. mint attempt 1;
4. store it in high-water and current maps;
5. stamp it on a running hard-silent thread and slot 1;
6. add slot 2 as an eligible idle pane;
7. enable scheduler permissions/discovery.

Timeout assertions:

- lifecycle trace is exactly
  `LeaseRevoked -> PaneFenced -> SlotReleased`;
- attempt 1 does not validate against current authority;
- high-water still stores attempt 1;
- slot 1 is `Fenced`, unassigned, unstamped, and non-resident;
- the old thread is absent;
- the timeout alert contains ticket, elapsed value, and phase;
- the structured `SessionTimedOut` activity remains present.

Redispatch assertions:

1. call `schedule_ready_tickets`;
2. assert slot 1 remains fenced/unselected;
3. assert slot 2 owns the ticket;
4. assert attempt 2 is strictly greater than attempt 1;
5. assert attempt 2 is current and attempt 1 is not;
6. assert current, high-water, slot, and thread all carry attempt 2.

This test directly covers strict ordering, bounded fence state, release, and
monotonic successor dispatch.

## Step 12 — add stale and release regressions

Extend a stale-thread test with a real lease and assert:

- current authority removal;
- high-water retention;
- `TransitionState::Fenced` after reclaim.

Retain or extend ordinary release tests to assert a healthy resident slot does
not become fenced.

Verification criteria:

- the shared helper cannot regress in one hard-silence caller only;
- normal completion reuse semantics remain covered.

## Step 13 — format and run focused tests

Run formatting:

```text
cargo fmt --all
cargo fmt --all -- --check
```

Run focused tests by exact or stable substrings:

```text
cargo test -p lisa-plugin dispatch_mints_and_stamps_strictly_new_attempt_lease
cargo test -p lisa-plugin session_timeout
cargo test -p lisa-plugin stale
cargo test -p lisa-plugin fence
```

Verification criteria:

- all focused tests pass;
- no fixture assumes release retains current authority;
- no native test invokes the Zellij host close command.

## Step 14 — run broad verification

Run:

```text
cargo test --workspace
just check
git diff --check
```

If `just check` duplicates the full suite, record both outcomes because it also
checks the deployed `wasm32-wasip1` target.

Inspect failures before changing unrelated code. Pre-existing unrelated dirty
files are not part of this ticket.

Verification criteria:

- all workspace tests pass;
- WASM plugin check passes with real pane-close code;
- no whitespace errors exist;
- only the intended source file and workflow artifacts are ticket-owned.

## Step 15 — inspect the final source diff

Run:

```text
git diff -- crates/lisa-plugin/src/lib.rs
git status --short
git diff --cached --name-only
```

Review specifically for:

- current/high-water map confusion;
- fence occurring after release;
- a fenced slot accidentally returning to `Idle`;
- pending Enter leakage;
- normal release session-reuse regression;
- ticket frontmatter changes;
- overlap with unrelated working-tree modifications.

Verification criteria:

- no ticket-owned source is staged in the ordinary index;
- no unrelated path is included in the source diff;
- ticket frontmatter phase/status are unchanged.

## Step 16 — commit the source unit through Lisa

Invoke the repository CLI if the installed `lisa` command lacks the current
transaction subcommand.

Use exactly:

```text
lisa commit-ticket \
  --ticket-id T-034-01-03 \
  --message "fix: revoke and fence timed-out attempts" \
  --include crates/lisa-plugin/src/lib.rs
```

Do not use ordinary `git add`, `git commit`, or a broad include.

Verification criteria:

- command returns a commit ID;
- commit contains exactly `crates/lisa-plugin/src/lib.rs`;
- the source path is clean afterward;
- ordinary index remains untouched;
- work artifacts remain for Lisa's final completion transaction.

## Step 17 — write `progress.md`

Record:

- each completed plan step;
- implementation details and invariants;
- any deviations and rationale;
- focused and broad test results;
- source commit ID and exact include path;
- ordinary-index cleanliness;
- remaining work, expected to be Review only.

## Step 18 — perform Review

Read the committed diff and relevant tests again.

Write `review.md` covering:

- outcome and acceptance-criterion evaluation;
- files modified/created/deleted;
- authority/high-water model;
- exact timeout sequence;
- named bounded fence state;
- test coverage and command results;
- source commit transaction;
- compatibility and operational tradeoffs;
- open concerns, especially permanent capacity loss and process-local high-water
  state;
- later S-034-02 surface-gating ownership.

Do not update ticket phase or status. Stop after `review.md`; Lisa owns the
completion transaction and seat release.

## Rollback boundary

The source change is one isolated commit touching one file. If review finds a
critical defect before Lisa completion, amend through another exact-path
`commit-ticket` source unit rather than using the ordinary index.

Removing only the fence call while retaining split maps would leave timeout
processes alive; removing only the high-water map would break attempt
monotonicity. The behavior should be reviewed and reverted as a coherent unit.

## Completion definition

Implementation is complete when:

- every release invalidates active lease authority;
- both hard-silence paths terminate/disqualify their old pane before release;
- the pane ends in `TransitionState::Fenced` with no retry;
- the acceptance scheduler test proves strict lifecycle order;
- redispatch uses another eligible pane and a strictly higher attempt ID;
- focused, workspace, and WASM checks pass;
- the ticket-owned source is committed through Lisa with an exact include;
- `progress.md` and `review.md` are written;
- ticket frontmatter remains untouched.
