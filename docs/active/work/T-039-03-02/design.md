# Design: explicit scheduler failure outcomes

## Decision

Add a private `FailureTransitionOutcome` enum to `lisa-plugin` with one variant
for each of the seven characterized scheduler paths:

- `AssignmentDeliveryFailed`;
- `AssignmentRecoveryFailed`;
- `StartupFailed`;
- `StartupRecoveryFailed`;
- `ErrorReclaimed`;
- `SessionTimedOut`;
- `StaleThreadReclaimed`.

Each variant carries the identity needed to tell which transition occurred:
the pane for seat-local failures, and ticket plus pane/fence facts where the
transition has scheduler-wide reclaim effects. The exact payload will stay
minimal and derived from state already used by the transition.

Retained failure helpers will return `Option<FailureTransitionOutcome>`:
`None` means their source-state guard rejected the transition; `Some` means the
named transition ran. Scanner methods will return
`Vec<FailureTransitionOutcome>` because one poll may process multiple paths.

Outcomes are constructed only at the end of the existing mutation sequence.
Therefore receiving an outcome means the transition completed; it does not
grant authority to perform the transition later.

## Alternatives

Reusing terminal seat states does not cover automatic reclaims and cannot
distinguish initial startup failure from exhausted startup recovery.

Using provenance `RunOutcome` cannot cover four retained failures, and error
and stale both map to `Failed` despite distinct triggers.

A coarse `Retained`/`Reclaimed` result captures policy but forces consumers to
reconstruct which path occurred from flags. It does not explicitly name every
transition.

Seven typed variants returned by existing transition boundaries preserve the
current state machines and give tests a direct assertion seam. This is the
chosen approach.

## Payload design

All successful variants include `pane_id`, the physical origin for these paths.

When the slot reservation resolves, variants include `ticket_id`. Delivery,
assignment recovery, and initial startup have a malformed-reservation branch
which still changes the seat and logs a failure. Their ticket identity is
therefore optional.

Startup recovery can return only after resolving a ticket reservation, so its
ticket is present. Automatic reclaims always start from a running thread and
carry a ticket.

Timeout and stale variants also carry `fenced`, preserving the actual bounded
fence result already passed to provenance. Error reclaim is explicitly
non-fenced by definition and needs no redundant flag.

## Authority preservation

The outcome enum is descriptive. It exposes no methods that mutate leases,
seats, threads, panes, provenance, or retries.

Mutation order remains local to each existing function. In particular:

1. retained failures still keep their threads and reservations;
2. initial startup failure still retains lease and pane;
3. startup recovery still revokes and fences but retains the failed seat;
4. ordinary error still emits non-fenced failed provenance before release and
   thread removal;
5. timeout and stale still revoke/fence before provenance, release, and removal;
6. timeout remains `TimedOut`; stale remains `Failed`;
7. all guards and retry bounds remain unchanged.

No common failure executor will be added. The seven paths have intentionally
different scheduler authority and cleanup semantics.

## Call-site handling

Existing orchestration callers need not branch on the returned result. They
already own trigger selection and subsequent control flow. They may discard the
descriptive result while direct tests capture it.

For scanner methods, outcomes are appended immediately after the existing
transition completes. Unknown error signals and over-budget active sessions do
not produce outcomes because no failure/reclaim transition occurred.

Recovery `.error` contributes `AssignmentRecoveryFailed`, not
`ErrorReclaimed`, because it retains scheduler authority and follows the
recovery terminal path.

## Testing decision

Extend representative matrix tests to assert exact returned variants for
bounded delivery, assignment recovery, initial/recovery startup, ordinary
error, timeout, and stale reclaim.

Existing state assertions remain unchanged. This proves the type names paths
without substituting outcomes for state and authority verification.

Run focused tests during development, then the full plugin library suite,
workspace tests, formatting, and the project quick gate.
