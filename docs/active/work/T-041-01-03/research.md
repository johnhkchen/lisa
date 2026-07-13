# Research: level-triggered completion eligibility

## Ticket boundary

T-041-01-03 is the final ticket in the pure completion-reducer story. It adds
durable-input eligibility derivation and reconciliation to the domain module
created by T-041-01-01 and extended by T-041-01-02. The acceptance criterion
requires a pure `reconcile(durable_inputs, state)` decision and focused unit
tests. It does not authorize plugin wiring, filesystem polling, command
execution, journal persistence, or dashboard rendering.

The ticket must remain provider-neutral. Claude and Codex may produce different
adapter evidence later, but this module consumes only admitted domain facts.

## Existing completion module

`crates/lisa-core/src/completion.rs` is public through
`lisa_core::completion`. It has no scheduler, Zellij, WASM, filesystem, or
process imports. Its identities, state, events, effects, reducer, and tests are
all colocated in one file.

The module defines opaque `AttemptId`, `CompletionId`, and `CorrelationId`
newtypes. They retain nominal separation while supporting clone, equality,
hashing, display, and string conversion.

`CompletionState` has five variants:

- `Eligible` represents a requestable aggregate.
- `Requested` means a request was accepted and its effect emitted.
- `CommandInFlight` requires a concrete correlation ID.
- `Rejected` retains a typed reason and retryability.
- `Confirmed` is the terminal successful state.

`EffectCommand` currently has one variant, `LaunchCompletion`, containing the
attempt and completion identities needed by a future adapter. A `Transition`
contains the next state and `Option<EffectCommand>`, limiting one reducer step
to at most one external command request.

## Existing reducer behavior

`reduce` is a pure owned-value function. An `Eligible` request and a retryable
`Rejected` request both enter `Requested` and emit one launch effect.
`Requested` and `CommandInFlight` reject duplicate requests as already pending.
`Confirmed` also refuses a request. A matching command result confirms or
rejects an in-flight aggregate; a mismatched result reports both correlation
identities.

The reducer describes edges. It does not independently notice that the durable
facts still obligate a request after an in-memory edge was lost. Reconciliation
therefore belongs beside, but remains distinct from, reduction.

## Existing disposition contract

`crates/lisa-core/src/disposition.rs` defines `ReviewDisposition`. The parser
returns exactly `Pass`, `Block { reason }`, or `Invalid { reason }`. Missing,
malformed, contradictory, and unknown documents all become `Invalid`.

Only `ReviewDisposition::Pass` grants authority. Block and invalid values are
typed non-passing inputs with operator-visible details. The completion module
can consume this existing type without importing or repeating its parser.

## Durable-input meaning

The acceptance criterion names two positive facts: current-lease artifact
admission and explicit pass disposition. Artifact presence alone is not enough;
the artifact must already have passed the attempt-lease admission boundary.
The pure module cannot inspect a path or compare plugin leases, so its input
must represent the result of that adapter verification.

The admitted artifact also supplies the `AttemptId` and `CompletionId` needed
to build the existing request event/effect. An absent admission means the
durable facts are not eligible, even when the aggregate's in-memory state says
`Eligible`.

## Level-triggered requirement

The Arcade field note records an artifact edge that advanced Review but did
not create a pending completion transaction. A later timeout observed the
artifact yet did not reconstruct the obligation. Edge-triggered handling can
therefore strand an eligible ticket when one transient request edge is lost.

Level-triggered reconciliation recomputes from current durable facts whenever
called. If the admitted artifact and pass verdict still exist and the
aggregate has no pending or confirmed transaction, the obligation remains
observable. Once the adapter applies the emitted request transition and stores
`Requested`, subsequent reconciliation returns no duplicate effect.

## In-flight boundary

`CommandInFlight` already makes correlation mandatory. A reconciler must not
treat an unresolved in-flight command as an absent request and blindly launch
another transaction. It must preserve the correlation in a named outcome so a
future adapter or durability layer can investigate, time-bound, or resolve
that exact command.

The follow-on durability story owns deadlines, journal replay, and idempotent
commit convergence. This ticket can establish the pure bounded policy shape:
an unresolved in-flight state yields one correlation-tagged action-required
decision, not another launch effect or an internal retry loop.

## State-specific constraints

- Ineligible durable inputs return no action for every aggregate state.
- Eligible inputs plus `Eligible` state may emit one request effect.
- Eligible inputs plus retryable `Rejected` state may emit one fresh request.
- Action-required `Rejected` state cannot be retried automatically.
- `Requested` represents a pending request and emits nothing.
- `Confirmed` represents a completed transaction and emits nothing.
- `CommandInFlight` emits no launch and retains its correlation in an
  actionable reconciliation result.

## Provider boundary

The completion core has no `AgentClient`, Claude, or Codex value. Adding one
would contradict the story's contract boundary. Provider adapters may differ
in how they discover and admit artifacts; after admission both feed the same
durable input structure and reconciliation function.

## Verification surface

Colocated unit tests can cover eligible, ineligible, already-requested,
confirmed, retryable rejection, action-required rejection, and in-flight
reconciliation without a runtime. Exact effect equality proves identity
preservation. A provider-name source scan is unnecessary if the production
API contains only completion and disposition types, but the tests and public
types should likewise contain no provider branch.

Focused verification is `cargo test -p lisa-core`. Workspace tests protect
downstream compilation, formatting protects the single source unit, and the
plugin WASM check verifies that the new core API remains target-compatible.

## Repository state and ownership

The active ticket and `.lisa/provenance.jsonl` are orchestration-owned modified
files. `crates/lisa-plugin/docs/` is unrelated untracked content. The only
anticipated ticket-owned source path is
`crates/lisa-core/src/completion.rs`; it must be committed through
`lisa commit-ticket` with that exact include path.

