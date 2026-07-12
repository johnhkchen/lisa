# Research: named failure transition outcomes

## Scope and ticket state

T-039-03-02 starts in Research and asks the scheduler to model every failure or
reclaim transition as an explicit named or typed outcome. The change must keep
the invariant matrix from T-039-03-01 unchanged, including E-034 attempt-lease
authority and E-035 recovery behavior.

The relevant production code is concentrated in
`crates/lisa-plugin/src/lib.rs`. No failure transition implementation is in
`lisa-core`; core provides `AttemptLease`, `Thread`, `ThreadStatus`, `Phase`, and
`RunOutcome`, while the plugin owns seats, physical panes, deadlines, signals,
lease mutation, provenance emission, and scheduling.

## Existing state types

`SeatAssignmentState` is scheduler truth for one reserved physical seat. Its
terminal retained states are `DeliveryFailed`, `RecoveryFailed`, and
`StartupFailed`. Those names describe current seat state, not the transition
that produced it.

`TransitionState` independently describes the resident pane/TUI lifecycle.
`Fenced` means a terminal, non-reusable physical pane. A failed seat may remain
on an unfenced pane, so seat failure and pane fencing cannot be conflated.

`FenceOutcome` describes only the bounded result of fencing:
`Fenced`, `AlreadyFenced`, or `NoAssignedPane`. It does not describe the larger
failure/reclaim transition.

`AttemptLifecycleEvent` is test-only. It records safety-critical ordering for
lease revocation, shell interruption/relaunch, fencing, and slot release. It is
not a production transition result and does not cover all seven paths.

`RunOutcome` belongs to durable provenance. It distinguishes `Failed` and
`TimedOut`, but retained assignment/startup failures intentionally emit no
provenance, so it cannot represent the full scheduler transition set.

## Retained failure paths

`fail_assignment_delivery` accepts `Starting`, `ReadyForAssignment`, or
`Delivering`. It changes the seat to `DeliveryFailed`, resolves the ticket from
the slot, marks the retained thread failed, adds an error alert, and logs. It
does not revoke the current lease, fence/release the slot, remove the thread, or
emit provenance. It currently returns `()`; invalid source states are silent
no-ops.

`fail_assignment_recovery` accepts only `Recovering`. It changes the seat to
`RecoveryFailed`, marks the retained thread failed when its ticket reservation
exists, adds an alert, and logs. The successor recovery lease and reservation
remain authoritative for operator reset. It returns `()`.

`fail_startup` accepts only the initial `Starting` shape used when recovery
authority or reservation validation fails. It changes the seat to
`StartupFailed`, marks a resolvable thread failed, alerts, and logs. It retains
the lease, reservation, and physical pane. It returns `()`.

`fail_startup_recovery` accepts `ResettingStartup` or replacement `Starting`
after the one relaunch. It resolves the reservation, changes the seat to
`StartupFailed`, marks the retained thread failed, alerts, revokes the current
successor lease, clears signals and pending input, fences/closes the pane, and
logs. Unlike `fail_startup`, it retains the failed reservation on a fenced pane.
It returns `()`.

## Automatic reclaim paths

`check_error_signals` consumes all `.error` signals. Recovery errors are routed
to retained `fail_assignment_recovery`. An ordinary running-thread error marks
the thread failed, emits `RunOutcome::Failed` with `fenced=false`, releases the
slot (which revokes the lease), removes the thread, alerts, and logs. Unknown
pane errors are logged no-ops. The scan returns `()`.

`check_session_timeouts` first separates active over-budget threads from threads
that are also hard-silent. For each reclaim it marks the thread failed, revokes
and fences the attempt, emits `RunOutcome::TimedOut`, releases the slot, removes
the thread, records a timeout alert, and logs. Pending completion and
awaiting-human states guard reclamation. The scan returns `()`.

`detect_stale_threads` selects running hard-silent threads, excluding pending
completion and awaiting-human panes. It marks each thread failed, revokes and
fences, emits `RunOutcome::Failed`, releases, removes, and logs. It returns `()`.

## Call topology

Assignment delivery failure is reached from ready-delivery errors, grace-paced
delivery errors, retry-delivery errors, and exhausted delivery acknowledgement.

Assignment recovery failure is reached from recovery setup validation, lease
mint failure, recovery launch preparation, recovery `.error`, route mismatch,
and recovery acknowledgement timeout.

Startup failure is reached from missing reservation, missing lease, and stale
lease validation before same-pane recovery can begin.

Startup recovery failure is reached from successor mint/probe failure, launch
artifact or marker failure, missing shell readiness, and missing replacement
process-start evidence.

The three automatic reclaim paths are invoked by the scheduler poll. Their
selection guards and side-effect ordering reside in the scanner methods.

## Existing tests and constraints

T-039-03-01 added
`assignment_recovery_failure_retains_authority_for_operator_reset` and mapped
the complete invariant matrix to native plugin tests. The matrix currently
passes with 313 library tests.

Key tests cover bounded delivery retry, recovery successor retention, missing
shell proof, missing replacement start, error-signal release and provenance,
session-timeout lifecycle ordering and monotonic redispatch, stale reclaim, and
awaiting-human/pending-completion guards.

The production module and its `#[cfg(test)]` module share private access, so a
new internal transition type can be asserted directly without widening the
crate's public API.

## Boundaries and assumptions

The ticket asks to name outcomes, not to change dashboard state, persistence,
retry bounds, logs, or public API. Existing terminal seat states remain the
operator-visible source of retained failure state.

Returning an outcome after mutation is compatible with current call sites:
Rust permits callers to ignore an ordinary returned enum unless it is marked
`must_use`. Tests can capture outcomes from direct helpers or scanner methods.

Scanner methods may process multiple signals or stale threads in one poll, so a
single optional outcome would lose information. Their natural typed return is a
vector in processing order.

No generalized teardown helper currently exists, and the seven paths have
deliberately different authority semantics. Any new type must name those
differences without centralizing or normalizing the mutations themselves.
