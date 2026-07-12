# Failure/reclaim state-machine map and invariant test matrix

## Result

The current scheduler has two teardown policies, not one generalized failure
edge:

- assignment/startup failures reach a named seat state and retain enough
  authority for explicit operator reset;
- error, timeout, and stale-thread reclaims remove scheduler ownership so the DAG
  may automatically dispatch a successor.

This document is the before-refactor contract for T-039-03-02. The matrix covers
lease, seat, thread, pane, provenance, and retry teardown for all seven paths.

## Notation

- `L` — `current_leases`; high-water history is stated separately.
- `S` — `seat_assignments[pane_id]`.
- `Th` — `threads[ticket_id]` status and presence.
- `Sl` — `agent_slots` reservation and physical-pane eligibility.
- `P` — `emit_provenance` invocation semantics.
- `R` — bounded retry or rescheduling authority.

“Retained” means the scheduler deliberately leaves the ticket/seat visible for
operator reset. “Released” means the thread is removed and the ticket may appear
in `Dag::get_ready_tickets` again. `lease_high_water` survives every path.

## State-machine overview

```text
Fresh chat delivery
  Delivering(g, retries=0)
    -- deadline --> Delivering(g, retries=1)
    -- deadline --> DeliveryFailed [operator reset]

Reused Codex assignment recovery
  AssignedPendingAck(g0)
    -- deadline/current predecessor --> Recovering(g1, no deadline)
    -- fresh launch submitted --> Recovering(g1, deadline)
    -- deadline or recovery .error --> RecoveryFailed [operator reset]

Initial SessionStart startup
  Starting(g0, relaunches=0)
    -- invalid/missing recovery authority --> StartupFailed [operator reset]
    -- deadline/current predecessor --> ResettingStartup(g1)
    -- exact shell-ready --> Starting(g1, relaunches=1)
    -- missing shell-ready or missing replacement start -->
       StartupFailed + fenced pane [operator reset]

Ordinary adapter error
  Running + current lease + reserved seat
    -- .error --> failed provenance + released seat/thread [automatic retry]

Session timeout
  Running + over budget + hard silent
    -- revoke --> fence --> timed-out provenance --> release --> remove
       [automatic retry]

Stale thread
  Running + hard silent
    -- revoke --> fence --> failed provenance --> release --> remove
       [automatic retry]
```

Grace-readiness Codex startup is intentionally not folded into SessionStart
startup recovery. Its initial `Starting` deadline paces the first delivery, then
uses the bounded delivery path. This preserves the provider distinction already
present in `check_assignment_ack_timeouts_at`.

## Invariant matrix

| Path | Trigger and terminal state | Lease (`L`) | Seat (`S`) | Thread (`Th`) | Pane/slot (`Sl`) | Provenance (`P`) | Retry (`R`) |
|---|---|---|---|---|---|---|---|
| Assignment delivery failure | `Delivering(0) -> Delivering(1) -> DeliveryFailed` | Current generation retained; no mint/revoke; high-water unchanged | Retained as `DeliveryFailed`; never Owned | Marked `Failed`, retained | Ticket and attempt reservation retained; pane not fenced or closed | None; retained thread has no terminal run record yet | Exactly one same-generation chat retry; then operator reset only |
| Assignment recovery failure | `AssignedPendingAck(g0) -> Recovering(g1) -> RecoveryFailed` | Validate g0; mint exactly g1; g1 remains current/high-water at failure | Retained as `RecoveryFailed` | Updated to g1, marked `Failed`, retained | Ticket/g1 reservation retained; abandoned old session remains `WaitingForExit`, not fenced | None; characterized ledger remains absent | One fresh successor attempt only; repeated polls cannot retry; operator reset |
| Startup failure | `Starting(g) -> StartupFailed` when initial recovery authority/reservation is invalid | Existing current lease retained when ticket can still be resolved; no successor from this edge | Retained as `StartupFailed` | Marked `Failed` when reservation resolves; retained | Reservation and physical pane retained; no fence/close | None | No automatic retry; operator repairs/resets |
| Startup recovery failure | `Starting(g0) -> ResettingStartup(g1) -> StartupFailed`, or replacement `Starting(g1, relaunches=1) -> StartupFailed` | g0 revoked before g1 mint; g1 revoked at terminal failure; high-water g1 retained | Retained as `StartupFailed` | Updated to g1, marked `Failed`, retained | Ticket/g1 reservation retained but slot becomes `Fenced`; resident session/client cleared; pane closed | None | At most one same-pane replacement launch; terminal operator reset |
| Error signal | running thread `-- .error -->` no thread/seat | Current lease revoked by release; high-water retained | Removed | Marked `Failed`, provenance emitted, then removed | Reservation/attempt cleared; live resident session preserved with cooldown; pane not fenced | `RunOutcome::Failed`, `fenced=false`, before thread removal | Automatic redispatch allowed; unknown-pane signal is consume/log no-op |
| Session timeout | over-budget plus 2x-threshold silence `-->` no thread/seat | Revoke before fence/release; high-water retained | Removed during fence/release | Marked `Failed`, then removed | Slot fenced, pane closed, reservation/attempt cleared, slot permanently ineligible | `RunOutcome::TimedOut`, fence result passed, before removal | Automatic redispatch on another eligible pane; awaiting/active sessions only warn |
| Stale-thread reclamation | `HealthStatus::Stuck` at 2x threshold `-->` no thread/seat | Revoke before fence/release; high-water retained | Removed during fence/release | Marked `Failed`, then removed | Slot fenced, pane closed, reservation/attempt cleared, slot permanently ineligible | `RunOutcome::Failed`, fence result passed, before removal | Automatic redispatch; pending completion and awaiting-human panes excluded |

## Ordering invariants

1. Startup recovery validates the predecessor as current and high-water before
   revoking it and minting the successor.
2. Startup recovery stamps successor authority into current lease, high-water,
   slot, and thread before waiting for exact shell readiness.
3. Startup recovery failure revokes the successor before leaving the failed
   reservation on a fenced pane.
4. Hard-silence teardown orders `LeaseRevoked`, `PaneFenced`, then
   `SlotReleased`; the native lifecycle trace asserts this sequence.
5. Error-signal release revokes through `release_slot_for_ticket` but never calls
   the pane-fence path.
6. Provenance is invoked while the thread still exists, before thread removal.
7. `release_slot_for_ticket` is an idempotent revocation boundary even when the
   caller already revoked the lease while fencing.
8. No retained terminal seat state is selected by later deadline scans, so an
   unchanged poll cannot create an unbounded retry loop.
9. A redispatched automatic reclaim must mint above retained
   `lease_high_water`; a predecessor cannot become current again.
10. Dashboard alerts are evidence, not execution authority: error alerts exist
    for retained failures and ordinary error reclaim; timeout alerts are specific
    to timeout reclaim.

## Test matrix

| Path/invariant | Primary deterministic test | What is pinned |
|---|---|---|
| Assignment delivery failure | `test_missing_fresh_chat_ack_retries_once_then_fails_without_ownership` | one retry, `DeliveryFailed`, failed retained thread, lease/slot retained, no relaunch, late ack rejected |
| Assignment recovery failure | `assignment_recovery_failure_retains_authority_for_operator_reset` | one successor, `RecoveryFailed`, current/high-water g1, failed retained thread and slot, no provenance, no repeated retry |
| Startup failure/recovery: missing shell proof | `test_missing_shell_readiness_fences_without_relaunch` | g0 revoke/g1 mint, same pane, terminal lease revoke, failed retained thread, fenced pane, no relaunch |
| Startup recovery: replacement start missing | `missing_replacement_start_fences_without_second_relaunch` | exactly one replacement launch, terminal fence/revoke, no second relaunch |
| Error signal | `test_check_error_signals_fails_running_thread` | one-shot signal consumption, thread removal, slot release, resident session preserved, alert |
| Error provenance | `provenance_emitted_on_error_signal` | failed non-authoritative record with current attempt identity |
| Session timeout | `test_check_session_timeouts_expired` | revoke/fence/release order, thread/seat removal, high-water retention, timeout alert, monotonic redispatch |
| Session timeout guards | `test_check_session_timeouts_active_session_deferred`; `test_session_timeout_skips_kill_when_awaiting` | active and human-blocked seats are not killed |
| Stale reclaim | `test_detect_stale_threads`; `test_codex_heartbeat_honest_then_genuine_hang_reclaimed` | genuine hard silence removes thread/releases slot; recent heartbeat remains bound |
| Stale guards | `test_detect_stale_threads_active_session_not_stale`; `test_detect_stale_skips_when_awaiting` | recent activity and human blocking exclude reclaim |
| Lease fence history | `fenced_attempt_and_replacement_publish_one_authoritative_done_record`; `split_brain_timeline_fences_old_attempt_and_admits_one_winner` | fenced history retained and stale predecessor cannot reclaim authority |

## Characterization test added

`crates/lisa-plugin/src/lib.rs` now contains
`assignment_recovery_failure_retains_authority_for_operator_reset`. It is a test-
only before-refactor fixture. It manually establishes the legacy
`AssignedPendingAck` entry state, drives the real recovery and deadline
functions, and asserts all matrix authorities at `RecoveryFailed`.

No production function, enum, constant, retry bound, or consumer behavior was
changed.

## Verification record

Baseline before adding the missing characterization:

```text
cargo test -p lisa-plugin --lib
312 passed; 0 failed; 0 ignored
```

Focused new characterization after formatting:

```text
cargo test -p lisa-plugin --lib \
  assignment_recovery_failure_retains_authority_for_operator_reset
1 passed; 0 failed; 312 filtered out
```

Complete matrix-bearing suite:

```text
cargo test -p lisa-plugin --lib
313 passed; 0 failed; 0 ignored
```

Formatting:

```text
cargo fmt --all -- --check
pass
```

## Implementation status

- Complete: source mapping and seven-path transition map.
- Complete: six-authority invariant matrix.
- Complete: existing-test mapping and baseline execution.
- Complete: missing assignment-recovery characterization fixture.
- Complete: focused and complete native plugin verification.
- Complete: exact source transaction committed through `lisa commit-ticket` as
  `ea0d9af6e7bc1abf86b2c8114341d2eab9981a75`.
- Complete: repository status confirms no ticket-owned source remains staged,
  modified, or untracked; only Lisa-managed publication/provenance paths remain.

## Downstream constraints

T-039-03-02 may introduce explicit transition outcome types, but this matrix
must pass unchanged. In particular it must not make operator-retained failures
automatically reschedulable, fence ordinary error panes, emit premature
provenance for retained failures, collapse timed-out and failed outcomes, or add
retries to any terminal seat state.

T-039-03-03 can consolidate these fixtures around the new named outcomes. Live
provider behavior remains outside this story and is deferred to S-039-06.
