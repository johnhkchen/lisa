# Progress — T-045-04-01 clean-exit-revoke-attempt

## Status

Implementation is complete.
The ticket-owned scheduler source change is formatted and fully tested.
The source unit is ready for the required Lisa isolated commit.
Review remains after commit verification.

## Completed phase artifacts

- `research.md` maps lease, claim, release, completion, and process boundaries.
- `design.md` evaluates four options and chooses completion-only Codex cleanup.
- `structure.md` defines the single-file source and fixture change.
- `plan.md` sequences implementation, tests, isolated commit, and Review.

All artifacts were written to the current private attempt work directory.
No phase or status frontmatter was edited by the agent.

## Implemented source unit

Modified:

- `crates/lisa-plugin/src/lib.rs`

No other source file was created, modified, or deleted for this ticket.

## Successful completion boundary

Added:

```rust
State::release_completed_slot_for_ticket
```

The helper snapshots only a live, non-fenced resident Codex pane.
It delegates authority and slot cleanup to `release_slot_for_ticket` first.
That existing method removes the predecessor from `current_leases`.
It clears the slot ticket and attempt lease.
It removes the pane's seat-assignment state.
It preserves `lease_high_water` for history and future monotonic minting.

After generic release, the helper submits the resident adapter's exit command.
For Codex this is `/exit`.
It moves the now-unassigned slot into `TransitionState::WaitingForExit`.
It records the exact transition start time.
It sets `has_session` false, matching the existing recycle transition.
It clears cooldown because the finite exit transition is the scheduling gate.
It retains the resident provider until shell cleanup.

The helper creates no successor ticket reservation.
It mints no attempt.
It writes no assignment or lease marker.
It launches no provider.
It does not alter the completion journal or provenance ordering.

## Completion call-site integration

`State::handle_completion_result` now calls successful-completion release after:

1. command success and commit-ID validation;
2. durable Done frontmatter verification;
3. Confirmed completion-journal persistence;
4. scheduler DAG rebuild;
5. phase and verification activity;
6. thread completion;
7. authoritative Done provenance.

It still removes the completed thread and calls `schedule_ready_tickets` afterward.
Other idle panes may accept work immediately.
The exiting pane cannot because slot selection requires transition state Idle.

## Exit-grace reuse

No new timer or transition state was added.
The existing unassigned `WaitingForExit` path handles the completion exit.
After its finite grace it publishes an empty shell:

- transition Idle;
- no transition timestamp;
- no session;
- no resident provider;
- no seat assignment;
- `lisa · idle` pane title.

Normal scheduling then takes the fresh-pane branch.
The next Codex ticket receives a new assignment and launch script.

## Test-only lifecycle trace

Added:

```rust
AttemptLifecycleEvent::CleanExitRequested { ticket_id, pane_id }
```

This is compiled only in native tests.
It is not production authority or a serialized format.
It makes the safety ordering explicit beside existing lease-revoked and
slot-released observations.

The boundary fixture requires the trace:

1. `LeaseRevoked`;
2. `SlotReleased`;
3. `CleanExitRequested`.

## New acceptance fixture

Added test:

```text
codex_completion_exits_revokes_and_launches_next_fresh_tui
```

The fixture creates two dependency-ordered Codex tickets and one empty pane.
It schedules the predecessor through the real fresh-launch scheduler path.
It advances the injected startup deadline without sleeping.
It builds an exact `AssignmentClaim` from the retained attempt and nonce.
It admits the claim and proves the predecessor seat is Owned.

The fixture then models durable completion cleanup:

- updates the predecessor ticket to Done;
- rebuilds its DAG;
- marks the thread completed;
- calls successful-completion release;
- removes the completed thread.

It proves:

- predecessor current authority is absent;
- predecessor high-water history remains;
- slot ticket, lease, and seat ownership are cleared;
- clean exit is pending on an unassigned pane;
- the exact retained predecessor claim is rejected;
- scheduling cannot mint or reserve the successor during exit;
- the finite grace returns the pane to an empty shell;
- only then can the successor reserve the pane;
- the successor gets a distinct assignment path and nonce;
- the successor launch script invokes the Lisa-owned Codex launcher;
- the successor enters Starting, not Owned;
- the predecessor claim stays rejected after successor launch.

## Scheduler transcript

The focused test prints stable diagnostic rows under `--nocapture`:

```text
T0450401|boundary|step=claimed|ticket=T-BOUNDARY-01|pane=10|attempt=1|nonce=<nonce>
T0450401|boundary|step=exit-requested|ticket=T-BOUNDARY-01|pane=10|lease=revoked|late_claim=rejected
T0450401|boundary|step=shell-ready|pane=10|resident=none|next_reserved=false
T0450401|boundary|step=fresh-launch|ticket=T-BOUNDARY-02|pane=10|attempt=1|nonce=<nonce>|state=starting|predecessor_claim=rejected
```

Native Zellij shims also print their encoded host calls.
The stable prefixed rows isolate the ticket's human-readable transcript.
Exact state assertions enforce the behavior independently of printed text.

## Existing completion assertion

Extended:

```text
artifact_completion_publishes_only_after_verified_commit_result
```

After a real `handle_completion_result` success, it now requires:

- the completed slot has no ticket;
- the slot transition is `WaitingForExit`;
- the old resident session is no longer published live.

This connects the new helper to verified completion rather than testing only a
direct internal call.

## Claude preservation

The completion helper only snapshots `last_client == Some(AgentClient::Codex)`.
Claude delegates to generic release and retains its existing resident-session
and clear-handshake behavior.

No `AgentAdapter` method changed.
No Claude test assertion was changed.
The mixed consecutive-reuse fixture, which exercises both providers, passed.

## Lease and fence preservation

Generic `release_slot_for_ticket` remains unchanged.
Failure, timeout, reset, audit, and stale cleanup callers remain unchanged.
`revoke_and_fence_attempt` remains the terminal hard-silence path.
Startup recovery and shell-readiness flows remain unchanged.

The focused attempt-lease and split-brain fencing tests passed.
The full plugin suite also passed all existing lease and fencing cases.

## Focused verification

Passed:

```text
cargo test -p lisa-plugin codex_completion_exits_revokes_and_launches_next_fresh_tui -- --nocapture
1 passed; 0 failed
```

Passed:

```text
cargo test -p lisa-plugin completion
22 passed; 0 failed
```

Passed:

```text
cargo test -p lisa-plugin consecutive_reuse
1 passed; 0 failed
```

Passed:

```text
cargo test -p lisa-plugin clear_handshake
2 passed; 0 failed
```

Passed:

```text
cargo test -p lisa-plugin attempt_lease
1 passed; 0 failed
```

Passed:

```text
cargo test -p lisa-plugin split_brain
1 passed; 0 failed
```

Passed:

```text
cargo test -p lisa-plugin artifact_completion_publishes_only_after_verified_commit_result
1 passed; 0 failed
```

## Broad verification

Passed:

```text
cargo fmt --all -- --check
```

Passed:

```text
cargo test -p lisa-plugin
395 passed; 0 failed
```

Passed:

```text
cargo test --workspace
```

All workspace unit, integration, and doc-test processes exited successfully.
The workspace output includes the 395-test plugin suite and all CLI/core suites.

Passed:

```text
git diff --check
```

## Plan deviations

The design suggested a fixture that might reuse existing consecutive-ticket
helpers. A dedicated two-ticket constructor was kept inline in the test because
the required dependency edge, fresh empty shell, and exact completion teardown
were narrower than the ten-ticket reuse helper.

The plan listed several possible test-name filters.
Some filters select only the relevant subset available in the current suite.
The full plugin and workspace suites provide the broader regression evidence.

No behavioral deviation from the selected design occurred.

## Worktree ownership

The ticket owns only:

- `crates/lisa-plugin/src/lib.rs`.

Unrelated runtime and materialized planning files were present before work:

- `.lisa/provenance.jsonl`;
- `.lisa/completion-journal.jsonl`;
- epic, story, and later-ticket Markdown files.

Those files were not edited as ticket source and will not be included in the
ticket source commit.

Lisa may mirror admitted phase artifacts to `docs/active/work/T-045-04-01/`.
The agent wrote artifacts only to the private attempt path as assigned.

## Isolated source commit

Committed the exact source path with:

```text
lisa commit-ticket --ticket-id T-045-04-01 --message "fix(plugin): cleanly exit Codex at ticket completion" --include crates/lisa-plugin/src/lib.rs
```

Commit:

```text
41aec7e0de45983e7d0385ceefee532d8ee76412
```

The isolated transaction included only `crates/lisa-plugin/src/lib.rs`.
No ordinary-index staging or ordinary Git commit command was used.
The ticket-owned source file is clean after the commit.
Implementation is complete and proceeds directly to Review.
