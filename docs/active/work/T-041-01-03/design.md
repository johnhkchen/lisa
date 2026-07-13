# Design: level-triggered completion eligibility

## Goals

The design must reconstruct completion obligation from durable admitted facts,
avoid duplicate launch requests once a transaction exists, retain correlation
for unresolved commands, stay pure, and expose no provider-specific branch.

## Durable input options

### Primitive booleans and separate identities

`artifact_admitted: bool`, `disposition_passed: bool`, and independent identity
fields would be compact. It would also permit a supposedly absent artifact to
carry usable authority identities and would discard typed block/invalid
disposition information.

### Raw lease and artifact paths

The core could accept an `AttemptLease`, current lease, and artifact path, then
perform admission itself. This would mix filesystem and scheduler concerns
into the pure module and duplicate the adapter boundary.

### Optional admitted-artifact value plus typed disposition

An `Option<CurrentLeaseArtifactAdmission>` makes the positive authority fact
explicit. The admission carries the attempt and completion identities required
by the request effect. `ReviewDisposition` is consumed directly so only its
typed `Pass` variant authorizes completion.

Decision: use the optional admission plus existing disposition type. The name
states that lease checking already occurred; the absence case represents no
admitted current-attempt artifact without inventing negative reason strings.

## Reconciliation return options

### `Option<EffectCommand>`

This exactly expresses effect or no effect for normal eligibility. It cannot
represent the required correlation-tagged actionable in-flight outcome.

### `Result<Option<EffectCommand>, CompletionRejection>`

An error could carry action-required information, but existing rejection
variants do not describe an unresolved correlated command. Treating expected
reconciliation state as an error also conflates lifecycle observation with an
illegal reducer edge.

### Named reconciliation outcome enum

An enum can distinguish `Effect`, `None`, and
`CommandInFlightActionRequired { correlation }`. The no-action result remains
explicit and the in-flight result is actionable without pretending to execute
or retry anything.

Decision: add `Reconciliation` with these three variants. It is a pure decision
value and keeps the mandatory correlation structurally attached to the only
outcome that needs it.

## Eligibility derivation

Eligibility is the conjunction of an admitted current-lease artifact and the
exact `ReviewDisposition::Pass` verdict. Block and invalid dispositions both
fail closed. The reconciler does not transform their reasons into a rejection,
because the ticket acceptance asks ineligible cases to return no request and
the disposition value remains available to the caller for rendering.

The durable-input check occurs first. This prevents a stale `Eligible` state
from emitting a request after its admission or pass authority disappears. It
also prevents an in-flight state with no longer-valid durable facts from being
reported as currently actionable by this obligation derivation.

## State policy

For eligible durable inputs:

- `Eligible` produces the existing launch effect.
- `Rejected { Retryable }` produces the existing launch effect.
- `Rejected { ActionRequired }` returns `None`.
- `Requested` returns `None` because a transaction is pending.
- `Confirmed` returns `None` because a transaction succeeded.
- `CommandInFlight` returns the correlation-tagged actionable outcome.

This policy interprets “no pending/confirmed transaction exists” to include a
retryable rejected state. It does not automatically retry action-required
rejections.

## Reuse of the reducer

The reconciler could construct `EffectCommand::LaunchCompletion` directly. It
could instead feed a `Request` into `reduce` and extract the effect. Reusing the
reducer keeps request transition semantics in one place and proves that the
reconciliation effect is one the aggregate accepts.

Decision: for requestable states, clone the state and admission identities,
call `reduce`, and extract its accepted effect. The requestability match makes
the expected result exhaustive. If the reducer contract changes, an internal
invariant assertion fails in tests rather than silently creating divergent
request behavior.

The public reconciler borrows durable inputs and state. It needs only clones of
opaque identities when a request is generated and does not consume caller
state.

## Bounded in-flight interpretation

This pure story has no clock and no durable command journal. Adding a deadline
would fabricate infrastructure owned by the later durability story. The
bounded behavior here is therefore decision-bounded: one call returns one
named action-required observation containing the exact correlation, and emits
zero effects. It neither loops nor converts in-flight to requestable state.

The later adapter can render or persist that outcome, and the later durability
story can add deadline and replay convergence without changing the fact that
uncertain in-flight work is never blindly duplicated.

## API shape

Add public values in `completion.rs`:

- `CurrentLeaseArtifactAdmission { attempt_id, completion_id }`
- `DurableCompletionInputs { artifact_admission, disposition }`
- `Reconciliation::{Effect, None, CommandInFlightActionRequired}`
- `reconcile(&DurableCompletionInputs, &CompletionState) -> Reconciliation`

Fields are public aggregate data, matching `Transition`. The admission and
durable inputs derive clone and equality for fixtures. Reconciliation derives
clone and equality for exact outcome assertions.

## Rejected approaches

- Do not add an `Ineligible` lifecycle state; eligibility is derived from
  durable input and can change independently of transaction history.
- Do not add provider variants; evidence normalization is an adapter concern.
- Do not launch or poll commands; effects remain inert data.
- Do not add filesystem parsing; reuse the typed disposition result.
- Do not turn `CommandInFlight` into `Rejected`; reconciliation reports a
  decision without mutating aggregate state.
- Do not invent a deadline; durable timing belongs to the named follow-on.
- Do not retry `ActionRequired` rejections.

## Test design

Unit tests will assert the exact launch effect for admitted pass plus Eligible,
no action for missing admission, no action for block and invalid disposition,
no action for Requested and Confirmed, a fresh effect for retryable rejection,
no action for action-required rejection, and the exact correlation in the
in-flight actionable outcome.

These cases collectively cover the ticket's eligible, ineligible,
already-pending, and in-flight requirements while exercising no Claude- or
Codex-specific inputs.

