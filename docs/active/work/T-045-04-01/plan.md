# Plan — T-045-04-01 clean-exit-revoke-attempt

## Implementation objective

Move graceful Codex shutdown onto the verified completion boundary, preserve
attempt revocation as the first authority mutation, hold the physical pane until
the existing exit grace proves a clean shell, and prove that a predecessor nonce
cannot claim either before or after the next fresh TUI is launched.

## Step 1 — add the completion exit observation

Modify the test-only `AttemptLifecycleEvent` enum in
`crates/lisa-plugin/src/lib.rs`.

Add `CleanExitRequested { ticket_id, pane_id }`.
Do not change production state or serialized types.

Verification:

- the file compiles in test and non-test configurations;
- existing exhaustive lifecycle assertions are updated only if the new event is
  legitimately emitted in their path;
- hard-fence and startup-recovery traces remain unchanged.

## Step 2 — add successful-completion slot cleanup

Add `State::release_completed_slot_for_ticket` beside generic release.

Implementation sequence:

1. inspect the assigned slot;
2. snapshot a live, non-fenced resident Codex pane;
3. obtain the resident adapter's exit command;
4. delegate to `release_slot_for_ticket`;
5. if Codex was snapshotted, submit `/exit`;
6. set the released slot to unassigned `WaitingForExit`;
7. clear cooldown and publish no live session;
8. append the test lifecycle event;
9. add one scheduler activity event.

Verification:

- authority is absent immediately after the helper;
- the slot cannot satisfy idle-slot selection;
- the helper does not mint or launch;
- generic release unit tests still pass unchanged.

## Step 3 — connect verified completion

Change `handle_completion_result` to call the new helper.

Preserve every preceding durable gate and every following cleanup action.
Do not change completion command execution, journal transitions, modal handling,
provenance, thread completion, or final scheduling.

Verification:

- existing completion success test now observes an exiting Codex slot where it
  previously observed provider-idle residency;
- update only assertions that intentionally describe the new boundary;
- completion failure and stale-authority tests remain unchanged.

## Step 4 — build the two-ticket boundary fixture

Add a native scheduler test in `lib.rs`.

Fixture preparation:

1. create temporary ticket, work, attempt, and signal directories;
2. write predecessor and dependent successor Codex tickets;
3. construct a one-pane, one-thread scheduler with zero wind-down;
4. start with an empty shell;
5. configure a deterministic fixture Lisa binary.

Initial lifecycle:

1. schedule predecessor;
2. assert fresh launch script and `Starting` seat state;
3. advance the exact startup deadline into `Delivering`;
4. construct the exact nonce-bearing claim;
5. admit the claim and require `Owned`.

Verification:

- predecessor owns the only pane;
- successor remains blocked by its dependency;
- the claim identity equals the retained assignment identity.

## Step 5 — exercise completion teardown and late claim

Model durable completion by updating predecessor Done and rebuilding fixture DAG.
Invoke the new successful-completion release and remove the completed thread.

Assert the exact state order:

1. lease revoked;
2. slot released;
3. clean exit requested.

Assert post-boundary state:

- no current predecessor lease;
- predecessor remains in high-water history;
- no slot ticket or attempt lease;
- no seat assignment;
- `WaitingForExit` with no published live session;
- completion-boundary activity is present.

Submit the retained predecessor claim.
Require rejection and no state restoration.

Call scheduling before exit grace.
Require that no successor attempt is minted or reserved.

## Step 6 — advance clean shell and launch the successor

Backdate the completion exit transition past its finite grace.
Call `check_transition_timeouts`.

Assert the pane is:

- `Idle`;
- unassigned;
- without a live session;
- without a resident provider;
- named as an empty Lisa slot.

Schedule ready tickets again.
Assert the successor:

- reserves the same pane only now;
- receives a fresh attempt lease;
- receives a distinct nonce-bearing assignment reference;
- receives a newly written launch script;
- invokes `lisa launch-codex` with the successor assignment path;
- enters `Starting` and not `Owned`;
- is represented as a fresh resident Codex session.

Submit the predecessor claim once more and require rejection.

Print stable transcript rows for the claimed, revoked, exited, shell-ready, and
fresh-launch observations.

## Step 7 — run focused tests

Run the new test with output visible:

```bash
cargo test -p lisa-plugin codex_completion_exits_revokes_and_launches_next_fresh_tui -- --nocapture
```

Run Codex lifecycle regressions:

```bash
cargo test -p lisa-plugin codex_stub_panes_receive_only_fresh_per_ticket_launcher_lines
cargo test -p lisa-plugin consecutive_reuse
cargo test -p lisa-plugin passive_claim
```

Run completion-focused tests:

```bash
cargo test -p lisa-plugin completion
```

Run lease and fencing tests:

```bash
cargo test -p lisa-plugin attempt_lease
cargo test -p lisa-plugin split_brain
cargo test -p lisa-plugin revoke
```

Run Claude-path tests using available name filters discovered from the suite.
At minimum cover its clear-handshake and consecutive-reuse behavior.

## Step 8 — run broad verification

Run formatting validation:

```bash
cargo fmt --all -- --check
```

If formatting fails because of the ticket change, run `cargo fmt --all` and
recheck the exact diff.

Run the plugin suite:

```bash
cargo test -p lisa-plugin
```

Run the workspace suite:

```bash
cargo test --workspace
```

If the full suite exposes unrelated concurrent failures, preserve exact output
and distinguish it from ticket-owned regressions in `progress.md`.

## Step 9 — inspect source ownership and diff

Inspect:

```bash
git diff -- crates/lisa-plugin/src/lib.rs
git status --short
```

Confirm only intended source hunks belong to this ticket.
Do not stage or modify unrelated runtime ledger or planning files.
Do not include private RDSPI artifacts in the source commit.

## Step 10 — commit the meaningful source unit

Commit the scheduler behavior and boundary fixture together because the test is
the executable contract for the behavior:

```bash
lisa commit-ticket \
  --ticket-id T-045-04-01 \
  --message "fix(plugin): cleanly exit Codex at ticket completion" \
  --include crates/lisa-plugin/src/lib.rs
```

Use the repository-resolved `lisa` command available in the environment.
Do not use `git add`, ordinary `git commit`, or a broad include.

After the commit, verify the source file is neither staged nor modified.

## Step 11 — complete implementation tracking

Write `progress.md` in the private attempt directory.
Record:

- implemented source behavior;
- boundary transcript and assertions;
- tests run and their results;
- commit identity;
- deviations from this plan;
- unrelated worktree state left untouched;
- no remaining ticket-owned source changes.

## Step 12 — Review

Inspect the committed diff and test evidence as a reviewer.
Confirm all acceptance facts:

- successful Codex completion requests clean exit;
- predecessor lease/nonce is revoked;
- late exact claim is rejected;
- successor cannot reserve the pane during exit;
- successor launches a fresh TUI after clean shell;
- Claude and lease/fence tests remain green.

Write `review.md` and exact pass/block `review-disposition.json`.
Do not update ticket phase or status.
Do not publish to `docs/active/work`.
Remain on this ticket after Review for Lisa's completion gate.
