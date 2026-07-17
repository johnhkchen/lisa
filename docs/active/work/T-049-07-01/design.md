# Design: block triage proposal

## Decision summary

Run triage only after an Operator-owned park is durably established.

Use a one-shot native agent subprocess launched by the plugin through the
existing Zellij RunCommands boundary.

Count each in-flight triage process against global and provider seat limits.

Record a Started transition before launch so restart cannot duplicate spend.

Give the subprocess read-only agent permissions and a hard timeout.

Validate its stdout as a typed proposal before publishing a sidecar.

Project an active sidecar through the shared parking model.

Render summary, recommendation, and prepared steps before the raw reason.

Expose explicit operator `apply` and `dismiss` proposal actions.

Record proposal creation and both operator actions in provenance.

## Option A: ask the blocking reviewer to triage before parking

The live reviewer already occupies a configured agent seat.

It also has the ticket and evidence in context.

Reprompting it would avoid a new provider process.

However, the park would then depend on a surviving live thread.

That recreates the event-triggered failure T-049-05-01 removed.

Waiting for its response would delay the park.

Releasing it first makes reliable reprompt delivery impossible.

Orphaned blocks have no thread to reprompt at all.

This option violates the fail-open timing boundary and is rejected.

## Option B: run an untracked background agent command

The plugin already invokes native host commands for world checks.

A headless Claude or Codex command could return structured JSON.

This is simple and does not consume a physical terminal pane.

Without scheduler accounting, it creates hidden concurrency and spend.

It could exceed both `max_threads` and a provider-specific cap.

It also needs explicit attempt provenance to avoid repeat spend after restart.

An untracked version is rejected.

The host-command mechanism remains useful when wrapped in scheduler accounting.

## Option C: model triage as a synthetic ticket/thread

A synthetic DAG node could reuse interactive adapters and pane lifecycle code.

It would naturally consume a seat and inherit provider observability.

The synthetic work is not a ticket and has no RDSPI phase sequence.

Injecting it into the DAG would affect readiness and all-done semantics.

Using reserved ticket IDs would leak into thread, pane, lease, and artifact APIs.

The interactive lifecycle has much broader failure behavior than one read-only
bounded request needs.

This option is rejected as excessive scheduler coupling.

## Option D: accounted one-shot host runner

Retain the existing ticket DAG unchanged.

Track in-flight triage jobs in a separate plugin map.

Include that map when evaluating global and provider concurrency.

Launch only from a Park provenance row for an Operator remedy.

Append a Started triage row before invoking the native command.

Treat any Started row for that source generation as consumed.

Use the configured ticket route and model for provider choice.

Run the provider once with read-only permissions and a fixed timeout.

Return only candidate JSON to the plugin.

This option satisfies bounded visible spend without polluting the DAG.

Option D is selected.

## Park-first ordering

Both live and orphan park paths keep their current authority sequence.

They write `status: blocked` first.

They append the ordinary Park transition.

They release any live seat and rebuild the DAG.

Only a later triage scheduling boundary may inspect the parked ticket.

Triage launch failure cannot roll back any of those effects.

Triage result handling never changes the canonical disposition.

Disabled configuration simply makes scheduling a no-op.

Timeout, failure, and invalid JSON append outcome provenance only.

No proposal sidecar is written in those cases.

Therefore existing Waiting-on-you output remains byte-for-byte applicable.

## Configuration

Add a `[triage]` table to `.lisa.toml`.

`enabled` defaults to true.

`timeout_secs` defaults to a small bounded value.

The native config validator recognizes only those keys.

The resolved values are serialized into the generated KDL layout.

The plugin parser remains lenient for legacy layouts.

The dashboard/status config summary need not gain extra noise.

Disabled behavior is explicit and fixture-testable.

## Proposal schema

Add a core `triage` module.

`TriageProposal` contains a non-empty summary and recommendation.

Summary is constrained to one plain sentence by validation.

It contains one or more typed prepared steps.

A command step carries display text and an exact shell command.

A file-edit step carries display text, a repository-relative path, exact old
text, and exact replacement text.

`StoredTriageProposal` adds ticket ID, source attempt lease, and state.

State is Pending, Applied, or Dismissed.

Only Pending proposals appear in Waiting on you.

The sidecar name is `triage-proposal.json` in canonical ticket work.

Atomic same-directory replacement avoids partially visible JSON.

## Agent prompt and validation

The runner receives ticket, disposition, and project root paths.

The prompt directs the agent to read those files and cited evidence.

It forbids mutations and asks for the exact proposal JSON schema.

Codex runs with approval never and a read-only sandbox.

Claude runs in print mode with read-oriented tools only.

The native runner captures output outside pipe back-pressure.

It kills the process group at the deadline.

Provider envelopes are reduced to the final assistant result.

The plugin parses the reduced JSON through the core schema.

Malformed, empty, multi-sentence-summary, or path-hostile output is invalid.

## Provenance model

Bump the additive provenance schema version.

Add `TriageTransitionRecord` with a disjoint `triage-transition` record type.

Its state is Started, Proposed, Failed, TimedOut, or Invalid.

It carries source attempt lease, resolved route, timeout, timestamps, and
optional bounded reason.

Started is appended before invocation.

One terminal transition is appended when a result arrives.

Add `ProposalActionRecord` with `proposal-action` record type.

Its action is Proposed, Applied, or Dismissed.

The Proposed row contains the validated proposal payload.

Applied and Dismissed name actor Operator and the source generation.

Mixed-ledger parsing remains backward compatible through disjoint enums.

## Capacity and restart behavior

The plugin scans blocked Operator remedies in ticket order.

It resolves the source generation from the latest Park transition.

It skips a generation with any triage transition already recorded.

It skips a ticket with a Pending proposal sidecar.

It considers normal Running threads plus triage jobs for `max_threads`.

It considers routed provider threads plus routed triage jobs for provider caps.

It may start as many jobs as the configured capacity permits.

Started provenance is the durable in-flight fence.

A plugin crash after Started intentionally fails open without retrying spend.

The raw parked ask remains visible throughout.

## Rendering

Extend `ParkedRemedy` with an optional active proposal.

Both CLI and dashboard consume the same projection.

When a proposal exists, render its summary first.

Render the recommendation next.

Render each prepared step next.

Render the ordinary ask and raw reviewer reason after the proposal.

When absent, retain the existing ask/reason ordering and copy.

World and Agent behavior remains unchanged.

## Explicit operator disposition

Add `lisa proposal apply <ticket>` and `dismiss <ticket>`.

Both require a Pending proposal on a still-blocked Operator-owned ticket.

Apply validates every prepared step before executing any file edit.

File edits require safe repository-relative paths and an exact unique old text.

Commands execute only after this explicit operator invocation.

On success, Apply marks the proposal Applied, appends provenance, and reopens
the ticket for ordinary re-review.

Dismiss marks it Dismissed and appends provenance.

Dismiss leaves the ticket parked and exposes the raw existing ask again.

Neither action changes the Review disposition document.

## T-046-06-03 regression expectation

The fixture supplies the historical bare reason, ticket criteria, and operator
note evidence to the result boundary.

The validated proposal summary must name a criteria-versus-evidence gap.

The recommendation must choose the documented criterion amendments.

Prepared file edits must identify the two stale acceptance sentences.

The test does not require a network provider call.

It injects the deterministic agent result at the same parser/result boundary.

## Rejected automatic behavior

Do not auto-apply a valid proposal.

Do not auto-dismiss an invalid proposal.

Do not convert Operator ownership to Agent ownership.

Do not unblock on proposal creation.

Do not retry a failed or timed-out triage generation.

Do not let triage affect completion sealing or dependency scheduling.
