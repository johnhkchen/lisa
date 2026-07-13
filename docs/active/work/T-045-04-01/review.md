# Review — T-045-04-01 clean-exit-revoke-attempt

## Disposition

Pass.

The successful Codex completion boundary now revokes the completed attempt,
submits a graceful TUI exit, blocks that physical pane during the existing finite
exit grace, rejects the predecessor's exact nonce-bearing claim, and launches a
fresh Codex TUI for the dependent ticket only after the pane is an empty shell.

The source change is committed through Lisa's isolated transaction.
All focused, plugin, and workspace verification is green.
No critical issue blocks completion.

## Commit

```text
41aec7e0de45983e7d0385ceefee532d8ee76412
fix(plugin): cleanly exit Codex at ticket completion
```

The commit contains exactly:

- `crates/lisa-plugin/src/lib.rs`.

No ordinary-index staging or ordinary Git commit was used.
The ticket-owned source path is clean after commit.

## Source changes

### Completion-only release seam

Added private scheduler method:

```rust
State::release_completed_slot_for_ticket
```

It snapshots a process action only when the completed ticket occupies a slot that:

- is not fenced;
- still has a resident session;
- identifies Codex as the resident client.

Every invocation first delegates to existing `release_slot_for_ticket`.
That existing method remains the provider-neutral authority and slot cleanup edge.
It revokes `current_leases[ticket]`.
It clears the slot's ticket and attempt lease.
It removes `SeatAssignmentState` for the pane.
It clears attention state and logs normal release.
It retains the lease high-water record.

Only after that revocation and release does the new helper submit `/exit` through
the resident adapter's `exit_command` contract.
The old TUI therefore has no remaining ticket authority when exit is requested.

### Unassigned bounded exit

The released Codex slot becomes:

```text
ticket_id = None
attempt_lease = None
seat assignment = absent
transition_state = WaitingForExit
has_session = false
last_client = Codex
cooldown = none
```

`WaitingForExit` is the scheduling exclusion.
Both compatible and recycle slot selection require an Idle transition.
The immediate post-completion scheduling call can use other panes but cannot mint
or reserve the next ticket into the exiting pane.

The existing transition evaluator supplies the finite grace.
Its existing `ExitReady` branch for an unassigned pane clears the transition,
resident provider, and session representation and publishes `lisa · idle`.
No new timer, state machine branch, or durable schema was introduced.

After that empty-shell edge, normal scheduling sees no resident provider.
Codex therefore uses the fresh-pane branch and the Lisa-owned launcher.
It does not use `/clear`, a same-process prompt, or ticket-bearing recycle.

### Verified completion integration

`handle_completion_result` calls the new helper only after:

- current completion authority is validated;
- successful exit and commit-ID output are validated;
- durable Done frontmatter is rescanned and verified;
- Confirmed completion is persisted in the completion journal;
- completion activity is logged;
- the thread is marked complete;
- authoritative Done provenance is emitted.

The existing journal and provenance ordering is unchanged.
Completion failures do not exit or release early.
Stale completion authority remains rejected by existing checks.

### Test lifecycle observation

Added test-only event:

```rust
AttemptLifecycleEvent::CleanExitRequested { ticket_id, pane_id }
```

It is not production state or a serialized type.
It makes successful boundary ordering directly assertable alongside:

- `LeaseRevoked`;
- `SlotReleased`.

The new fixture requires that exact order.
Hard fencing and startup recovery do not emit this event.

## Acceptance test

Added:

```text
codex_completion_exits_revokes_and_launches_next_fresh_tui
```

The fixture uses two dependency-ordered Codex tickets and one physical pane.
The pane starts as an empty shell.
The predecessor is scheduled through the production fresh-launch path.
Its startup grace is advanced using exact injected deadlines, not sleep.

The test constructs a real shared `AssignmentClaim` value from:

- predecessor ticket ID;
- predecessor attempt generation;
- scheduler-retained assignment nonce.

It admits that exact claim and proves the predecessor owns the pane.
It then drives durable ticket Done plus successful scheduler cleanup.

The fixture asserts:

1. lease revocation precedes slot release and clean exit observation;
2. predecessor current authority is absent;
3. predecessor high-water history remains;
4. the slot has no ticket, lease, or seat assignment;
5. the slot is unavailable in `WaitingForExit`;
6. the exact old attempt+nonce claim is rejected;
7. the successor is neither minted nor reserved during exit;
8. exit grace publishes an empty shell;
9. the successor then reserves the same physical pane;
10. it receives a distinct immutable assignment path and nonce;
11. its launch script invokes `lisa launch-codex` with that assignment;
12. it enters Starting rather than Owned;
13. the predecessor claim remains rejected after fresh launch.

## Scheduler transcript

The acceptance test emits stable prefixed rows with `--nocapture`:

```text
T0450401|boundary|step=claimed|ticket=T-BOUNDARY-01|pane=10|attempt=1|nonce=<nonce>
T0450401|boundary|step=exit-requested|ticket=T-BOUNDARY-01|pane=10|lease=revoked|late_claim=rejected
T0450401|boundary|step=shell-ready|pane=10|resident=none|next_reserved=false
T0450401|boundary|step=fresh-launch|ticket=T-BOUNDARY-02|pane=10|attempt=1|nonce=<nonce>|state=starting|predecessor_claim=rejected
```

The transcript is backed by exact state assertions.
It is not the only test oracle.

## Real completion-result coverage

Extended existing test:

```text
artifact_completion_publishes_only_after_verified_commit_result
```

After a successful `handle_completion_result`, it now asserts the completed Codex
slot is `WaitingForExit` and no longer publishes a resident live session.
This prevents the new fixture from passing through a helper that production
completion forgot to call.

## Claude behavior

Claude does not match the Codex snapshot predicate.
It continues through generic release unchanged.
Its resident session and `ClearHandshake` policy are preserved.

No adapter interface or Claude implementation changed.
No Claude test was weakened or updated to accommodate this ticket.
The mixed consecutive-reuse fixture still proves both provider mechanisms.

## Lease and fencing behavior

`release_slot_for_ticket` is unchanged.
`revoke_and_fence_attempt` is unchanged.
Timeout, stale, error, reset, audit, and recovery call sites are unchanged.
The hard-silent pane-close boundary remains distinct from graceful completion exit.

Attempt high-water behavior remains unchanged.
Assignment files remain immutable historical evidence.
Authority is denied through current-lease, slot-lease, active-generation, and nonce
admission checks rather than historical-file deletion.

## Verification

Focused acceptance test:

```text
cargo test -p lisa-plugin codex_completion_exits_revokes_and_launches_next_fresh_tui -- --nocapture
1 passed; 0 failed
```

Completion tests:

```text
cargo test -p lisa-plugin completion
22 passed; 0 failed
```

Provider and lease regressions:

```text
cargo test -p lisa-plugin consecutive_reuse
1 passed; 0 failed

cargo test -p lisa-plugin clear_handshake
2 passed; 0 failed

cargo test -p lisa-plugin attempt_lease
1 passed; 0 failed

cargo test -p lisa-plugin split_brain
1 passed; 0 failed
```

Plugin suite:

```text
cargo test -p lisa-plugin
395 passed; 0 failed
```

Formatting and diff validation:

```text
cargo fmt --all -- --check
git diff --check
```

Both passed.

Exact committed workspace suite rerun:

```text
cargo test --workspace --quiet
```

Passed with:

- CLI library: 19 tests;
- CLI binary: 270 tests;
- CLI integration suites: all green;
- core: 200 tests;
- plugin: 395 tests;
- doc tests: green;
- real-Zellij delivery boundary: one existing ignored test requiring the named
  external tools and WASM target.

## Open concerns and limitations

The scheduler still uses its established bounded grace after `/exit`; it does not
consume a new provider-issued shell-ready signal for ordinary completion.
That policy predates this ticket and is covered by existing transition tests.

This is native fixture evidence with inert Zellij host shims.
It validates scheduler state, adapter command selection, queued pane submission,
assignment identity, and launch-script bytes.
It does not claim live installed Codex plus real Zellij proof.
That metered field boundary is explicitly owned by `S-045-05`.

Exactly-one authoritative completion across this boundary is intentionally the
dependent ticket `T-045-04-02`.
This ticket preserves existing completion journal and provenance behavior but does
not absorb that ticket's dedicated single-record/no-double-inject assertion.

No TODO or critical defect remains within this ticket's scope.

## Worktree review

The ticket-owned source file is committed, unstaged, and unmodified.
The commit includes no unrelated path.

The worktree still contains pre-existing or Lisa-owned runtime/materialization state,
including provenance, completion journal, epic/story/ticket files, and admitted work
artifacts. Those paths were excluded from the source commit and not treated as this
ticket's source ownership.

## Final assessment

The acceptance criterion is satisfied at the stated fixture boundary.
Successful Codex completion now owns graceful exit and revocation before pane reuse.
A late exact nonce-bearing predecessor claim cannot regain ownership.
The dependent successor cannot reserve the pane until exit grace establishes an
empty shell, after which it launches as a fresh Codex TUI.

The work is ready for Lisa's lease verification, Review admission, completion
publication, and final completion commit.
This attempt remains on T-045-04-01 until Lisa confirms that commit.
