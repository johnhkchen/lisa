# Review: T-034-03-01 deterministic split-brain regression

## Outcome

The T-031-02 split-brain field timeline is now a committed deterministic
scheduler regression.

The test drives a slow old Codex attempt through real timeout reclamation,
redispatches the ticket to a different resident Codex pane, deliberately leaves
the delivered replacement prompt unacknowledged, resumes every relevant old-pane
signal, and completes only through the successor lease.

The regression passes without production scheduler changes.

Ticket-owned source is committed at:

`0ffe40f67551774964cfaf3e229ba5052cee43ea`

## Files changed

### Modified source

- `crates/lisa-plugin/src/lib.rs`

The source change adds one native test:

`split_brain_timeline_fences_old_attempt_and_admits_one_winner`

No production source, public API, serialized schema, hook, CLI contract,
configuration, or scheduler behavior changed.

### Created workflow artifacts

- `docs/active/work/T-034-03-01/research.md`
- `docs/active/work/T-034-03-01/design.md`
- `docs/active/work/T-034-03-01/structure.md`
- `docs/active/work/T-034-03-01/plan.md`
- `docs/active/work/T-034-03-01/progress.md`
- `docs/active/work/T-034-03-01/review.md`

No files were deleted.

The ticket's phase and status frontmatter were not edited by this agent.

## Scenario construction

The regression builds a temporary one-ticket DAG from a real markdown ticket.

The ticket is Codex-routed and begins in Review so the final leg can cross the
actual commit-gated completion boundary.

Two physical pane slots are present:

- pane 1 hosts attempt 1 and begins Owned;
- pane 2 is Idle and contains a resident Codex process.

Attempt 1's start and activity timestamps exceed both the one-second session
budget and the two-second hard-silence threshold.

Attempt 1 also has private staged review bytes before timeout, representing the
slow process's useful but not yet admitted late work.

All files and state are isolated in a `tempfile::TempDir`.

The test does not sleep, invoke Codex, require credentials, depend on Zellij, or
share runtime paths with parallel tests.

## Fence-before-reschedule

The test calls `check_session_timeouts`; it does not manually reproduce timeout
side effects.

It asserts the test-only lifecycle trace exactly:

1. `LeaseRevoked` for T-SPLIT;
2. `PaneFenced` for pane 1;
3. `SlotReleased` for T-SPLIT.

After timeout:

- attempt 1 is absent from current authority;
- its thread is removed;
- pane 1 has no ticket or lease stamp;
- pane 1 has no seat assignment;
- pane 1 remains terminally Fenced;
- timeout provenance is attributable to attempt 1.

The test then calls `schedule_ready_tickets`.

Pane 1 cannot be selected because it is Fenced.

Pane 2 receives attempt 2, and its attempt ID is exactly attempt 1 plus one.

Thread, slot, and `current_leases` all agree on attempt 2.

This proves the ordering and real scheduler selection boundary together.

## Replacement with missed acknowledgement

Because pane 2 contains a resident Codex session, redispatch begins in
`AssignedPendingAck` rather than Owned.

The regression completes the clear handshake with `handle_cleared_signal(2)`.

That action delivers the tagged attempt-2 prompt and arms the acknowledgement
deadline.

The test then deliberately withholds the matching acknowledgement.

It asserts the pane remains pending and `seat_is_owned(2)` is false.

This is the deterministic equivalent of the field prompt-injection miss: Lisa
attempted delivery but received no provider acceptance evidence.

## Old-pane resume rejection

While attempt 2 is pending, the test writes these pane-1 signals:

- heartbeat carrying the attempt-1 lease;
- Codex acknowledgement carrying the attempt-1 generation;
- idle;
- stopped;
- cleared;
- error.

It invokes every corresponding production consumer.

Each file is consumed so it cannot replay.

After replay:

- the replacement thread clock is unchanged;
- the replacement pane clock is unchanged;
- current authority remains attempt 2;
- pane 1 remains Fenced and unassigned;
- pane 2 remains the only reservation;
- pane 2 remains pending and unowned;
- no error is attributed to the replacement;
- no pending completion exists.

The test also submits the stale generation directly to the replacement
acknowledgement boundary and proves it cannot promote pane 2.

Only the exact attempt-2 acknowledgement performs the one transition to Owned.

At that point the count of Owned physical seats is exactly one.

## Artifact attribution

Attempt 1 and attempt 2 use distinct private staging directories and distinct
review sentinel bytes.

Running artifact advancement while only attempt-1 review exists leaves the
canonical work directory empty.

Direct admission with attempt 1 returns an error because attempt 2 is current.

The predecessor bytes remain readable only from attempt-1 staging.

After attempt 2 writes its review and artifact advancement runs again:

- canonical review exists;
- canonical bytes exactly equal attempt-2 bytes;
- canonical bytes do not equal attempt-1 bytes;
- pending completion authority is attempt 2;
- the source remains Artifact.

This is the negative and positive proof for no cross-pane artifact attribution.

## Completion and provenance

A direct attempt-1 completion request returns false and creates no pending
transaction.

The regression temporarily presents attempt 1 as the thread provenance stamp
and proves authoritative Done publication returns false while attempt 2 is
current.

The valid attempt-2 artifact enters the existing pending completion path.

The test marks the fixture ticket durably Done and supplies a valid 40-hex
completion result, matching the existing native transaction-result seam.

It sends the result twice to prove a duplicate callback is inert.

The append-only ledger contains two rows:

1. attempt 1 — TimedOut, fenced, non-authoritative;
2. attempt 2 — Done, unfenced, authoritative.

There is exactly one authoritative Done record, and it carries attempt 2.

The timeout history row is intentionally retained under the T-034-02-04
provenance contract.

Thus the ticket's “one provenance record” safety property is enforced as one
authoritative completion record rather than erasing attributable timeout
history.

## Acceptance mapping

### Slow old attempt

Met by attempt 1's old start/activity clocks and pre-existing private review.

### Timeout

Met through `check_session_timeouts`, including TimedOut provenance.

### Fence before reschedule

Met by exact lifecycle ordering and real redispatch onto pane 2.

### Replacement with missed injection

Met by delivered prompt, armed ack deadline, absent matching ack, and explicit
not-Owned assertion.

### Every resumed signal rejected

Met for native ownership-relevant heartbeat, ack, idle, stopped, cleared, and
error signals; all are consumed without changing successor authority or state.

### No duplicate ownership

Met by the single reservation before ack, zero Owned seats while pending,
exactly one Owned seat after valid ack, and no Owned seats after completion.

### No cross-pane artifact attribution

Met by distinct sentinel content and explicit stale/current admission checks.

### One provenance winner

Met by exactly one authoritative Done row on the successor lease, despite
timeout history and a duplicate completion callback.

### Lease-check regression sensitivity

The scenario has explicit assertions after each independently guarded boundary:

- timeout revocation and fencing;
- monotonic dispatch;
- pending ownership;
- stale acknowledgement;
- stale heartbeat/cross-pane signals;
- stale artifact admission;
- stale completion admission;
- stale authoritative provenance;
- duplicate result handling.

Removing an exercised guard exposes an incorrect state transition, publication,
or ledger count rather than merely changing an incidental log message.

## Test coverage

Passed:

```text
cargo test -p lisa-plugin split_brain_timeline_fences_old_attempt_and_admits_one_winner
cargo test -p lisa-plugin
cargo test --workspace
cargo fmt --all -- --check
cargo check -p lisa-plugin --target wasm32-wasip1
cargo clippy -p lisa-plugin --all-targets -- -D warnings
git diff --check -- crates/lisa-plugin/src/lib.rs
git diff --check -- docs/active/work/T-034-03-01
```

Results:

- focused split-brain regression: 1 passed;
- plugin suite: 273 passed;
- CLI suite: 270 passed;
- core suite: 155 passed;
- atomic provider-contract integration: passed;
- workspace and doc tests: passed;
- WASM target check: passed;
- plugin Clippy with warnings denied: passed;
- formatting and whitespace checks: passed.

## Source commit and repository integrity

The host-installed `/opt/homebrew/bin/lisa` is older and does not expose
`commit-ticket`.

The source was committed with the freshly built repository CLI:

```text
target/debug/lisa commit-ticket \
  --ticket-id T-034-03-01 \
  --message "Test deterministic split-brain fencing" \
  --include crates/lisa-plugin/src/lib.rs
```

Commit `0ffe40f67551774964cfaf3e229ba5052cee43ea` contains exactly the plugin
source path.

`crates/lisa-plugin/src/lib.rs` is clean after the commit.

The ordinary Git index is empty.

No ordinary `git add` or `git commit` command was used.

Unrelated pre-existing modified and untracked paths remain excluded.

## Open concerns and limitations

### Native test versus live pane closure

Native tests observe `TransitionState::Fenced` and lifecycle order but stub the
actual Zellij `close_terminal_pane` host call.

This is the intended deterministic boundary.

T-034-03-02 owns the isolated fresh-loop proof using a newly built binary/WASM.

### Prompt miss representation

The test proves Lisa delivered the tagged prompt and received no matching ack.

It cannot distinguish whether the terminal dropped the keystrokes or the client
failed to accept them; scheduler safety is intentionally based on the missing
ack rather than terminal internals.

### Claude parity

This ticket does not add a Claude path to the scenario.

The parent story assigns unchanged Claude assignment/completion behavior to the
next live-proof ticket.

Existing workspace tests, including consecutive mixed-provider reuse, remain
green.

### Timeout history count

There are two total provenance rows by design, but only one authoritative Done.

A reviewer should preserve this distinction; deleting the timeout row would
violate the prerequisite append-only provenance contract.

## Critical issues

None found.

## Human review focus

A reviewer should verify:

1. the replacement prompt is delivered before the missing-ack window begins;
2. lifecycle ordering prevents pane 1 from being eligible for redispatch;
3. resumed pane-1 signals cannot mutate pane-2 thread or assignment state;
4. predecessor and successor sentinel bytes cannot be confused;
5. final authoritative provenance belongs only to attempt 2;
6. the test remains production-method driven rather than becoming a parallel
   scheduler model.

## Final assessment

The ticket acceptance criterion is satisfied by a committed, deterministic,
composed scheduler regression.

It converts the T-031-02 field failure into durable regression evidence while
preserving the repository's existing lease, commit, and provenance contracts.

No open concern blocks the next live-proof ticket.
