# Progress: T-034-03-01 deterministic split-brain regression

## Status

Implementation, verification, and the isolated source commit are complete.

## Completed

- [x] Read project, ticket, and RDSPI workflow guidance.
- [x] Map timeout, fencing, lease, signal, artifact, completion, and provenance paths.
- [x] Write `research.md`.
- [x] Write `design.md` and choose a composed native plugin regression.
- [x] Write `structure.md` with the one-file test boundary.
- [x] Write `plan.md` with focused and broad verification.
- [x] Add `split_brain_timeline_fences_old_attempt_and_admits_one_winner`.
- [x] Drive timeout through `check_session_timeouts`.
- [x] Assert `LeaseRevoked` → `PaneFenced` → `SlotReleased` ordering.
- [x] Redispatch through `schedule_ready_tickets` onto a distinct resident Codex pane.
- [x] Deliver the replacement prompt and deliberately omit its acknowledgement.
- [x] Replay predecessor heartbeat, ack, idle, stopped, cleared, and error files.
- [x] Prove predecessor artifact, completion, and Done provenance rejection.
- [x] Acknowledge the successor and admit only its private review bytes.
- [x] Publish one authoritative successor Done after verified completion.
- [x] Exercise duplicate completion-result suppression.
- [x] Run focused regression.
- [x] Run formatting, plugin, workspace, WASM, Clippy, and diff checks.
- [x] Inspect acceptance and mutation-sensitivity coverage.
- [x] Commit the exact source path through Lisa's isolated transaction.
- [x] Confirm no ticket-owned source residue remains.

## Remaining

None. `review.md` is written and the ticket is ready for Lisa's completion
handling.

## Implementation detail

The regression uses a single temporary Review-phase Codex ticket and two
physical pane slots.

Attempt 1 begins Owned on pane 1 with timestamps beyond both the configured
session budget and hard-silence threshold.

Its private `review.md` is written before timeout to model a slow attempt that
has useful late work.

The real timeout method revokes attempt 1, fences pane 1, emits attributable
TimedOut history, releases the reservation, and removes the thread.

Real scheduling then mints attempt 2 and selects pane 2, because pane 1 remains
terminally Fenced.

Pane 2 already hosts Codex, so the scheduler enters `AssignedPendingAck`.

The test completes the clear handshake to deliver the tagged successor prompt
and arm the acknowledgement deadline, then intentionally supplies no matching
ack before replaying predecessor activity.

The predecessor replay includes every native ownership-relevant signal file:

- heartbeat;
- Codex assignment ack;
- idle;
- stopped;
- cleared;
- error.

All files are consumed, while the successor clocks, lease, reservation,
pending assignment, and unowned state remain unchanged.

Direct negative assertions also cross the artifact, completion, and provenance
lease boundaries.

Only after a matching attempt-2 ack does the replacement become the sole Owned
seat.

Only attempt-2 staged bytes reach the canonical work directory.

The final ledger retains two append-only rows:

1. attempt 1 TimedOut, fenced, non-authoritative;
2. attempt 2 Done, unfenced, authoritative.

Filtering for authoritative Done yields exactly one record.

## Verification results

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

Focused result: 1 passed, 0 failed.

Plugin result: 273 passed, 0 failed.

Workspace result includes 270 CLI tests, the atomic provider-contract
integration test, 155 core tests, 273 plugin tests, and doc tests.

The WASM target check passed.

Plugin Clippy passed with warnings denied.

## Source commit

Committed:

```text
0ffe40f67551774964cfaf3e229ba5052cee43ea
```

Commit subject:

```text
Test deterministic split-brain fencing
```

The installed `/opt/homebrew/bin/lisa` predates the repository's isolated
transaction command and returned `unrecognized subcommand 'commit-ticket'`.

The repository-built CLI was already available from the passing workspace build,
so the transaction used:

```text
target/debug/lisa commit-ticket \
  --ticket-id T-034-03-01 \
  --message "Test deterministic split-brain fencing" \
  --include crates/lisa-plugin/src/lib.rs
```

The commit contains exactly `crates/lisa-plugin/src/lib.rs`.

That source path is clean after the commit.

The ordinary Git index remains empty.

Unrelated pre-existing modified and untracked paths were not included or
changed for this ticket.

## Mutation-sensitivity review

The test has explicit failure points for:

- timeout revocation/fence ordering;
- reuse of a fenced pane;
- non-monotonic successor leases;
- treating pending assignment as ownership;
- stale generation acknowledgement;
- cross-pane liveness/error attribution;
- stale artifact admission;
- stale completion admission;
- stale authoritative provenance;
- duplicate authoritative completion.

## Deviations

The first implementation snapshot asserted pending acknowledgement immediately
after scheduling.

Design review during implementation identified that the resident Codex adapter
was still waiting for its clear handshake at that point, so the tagged prompt
had not yet been delivered.

The test now calls `handle_cleared_signal(2)`, asserts the acknowledgement
deadline is armed, and only then models the missing acknowledgement and old-pane
resume.

This strengthens fidelity to the ticket timeline without changing production
logic.

Clippy initially reported one needless borrow in the new fixture's
`update_ticket_done` call.

The call was corrected and the complete verification matrix was rerun.

The plan spells the transaction as `lisa commit-ticket`; the host-installed
binary did not expose that current subcommand.

Using the freshly built repository CLI is a tooling-path deviation only. It
executes the same isolated transaction implementation and produced the required
single-path commit without touching the ordinary index.
