# Design: proposal apply records first

## Goals

- Persist operator intent before the first prepared step can mutate the repository.
- Persist a terminal outcome for success and ordinary step failure.
- Identify the successfully applied prefix and the first failed step.
- Announce every step at the moment it is about to execute.
- End the sidecar's Pending state after both success and step failure.
- Reject and hide proposals that do not belong to the latest current Park lease.
- Preserve clean apply, dismiss, dashboard, and triage behavior for matching proposals.

## Apply evidence options

### Option A: reuse one Applied row and enrich it

- A single terminal row cannot exist before mutation and also describe the outcome.
- Writing it first would falsely claim success.
- Rewriting it later would violate the append-only ledger model.
- This option does not satisfy the explicit attempt-row requirement.
- Rejected.

### Option B: use generic triage transition rows

- Triage transitions describe bounded agent first-responder execution.
- Operator application is a distinct lifecycle and actor.
- Reusing triage state would blur agent proposal generation with operator execution.
- Existing ledger readers already distinguish proposal actions.
- Rejected.

### Option C: extend ProposalActionRecord additively

- Add `Attempted` and `Failed` action values.
- Add optional `step_count`, `applied_steps`, `failed_step`, and `failure_reason` fields.
- Attempted rows carry actor, full proposal, and total step count.
- Applied rows carry the ordered descriptions of all applied steps.
- Failed rows carry the successful prefix, failed description, and diagnostic reason.
- Existing Proposed and Dismissed rows keep their current shape and spellings.
- Optional/defaulted fields allow historical ledger rows to deserialize unchanged.
- Chosen because it keeps one coherent proposal-action evidence family.

## Sidecar failure-state options

### Delete the sidecar after failure

- Deletion would remove diagnostic context and erase the proposal that was attempted.
- It would make the ledger the only remaining representation.
- It is unnecessary to unblock triage because projection can filter terminal states.
- Rejected.

### Reuse Dismissed

- Dismissed means an operator intentionally declined the proposal.
- A partially applied proposal has materially different semantics.
- Reusing it would make operator decisions and execution failures indistinguishable.
- Rejected.

### Add Failed

- Failed is terminal and naturally excluded by the existing Pending filter.
- It preserves the proposal and source lease for diagnosis.
- It makes retry/re-triage behavior explicit.
- Chosen.

## Execution control flow

1. Validate ticket, disposition, sidecar identity/state, and current Park correlation.
2. Construct and append an Attempted operator record.
3. Iterate steps in their proposal order.
4. Write one pinned announcement before executing each step.
5. Execute the step and collect its description after success.
6. For file edits, verify the old text occurs exactly once immediately before replace.
7. Stop at the first command, read, precondition, or publication failure.
8. Set sidecar state to Applied or Failed and publish it atomically.
9. Append the corresponding outcome record with step evidence.
10. Reopen only after a successful apply outcome.
11. Return the original execution failure after best-effort durable state/outcome writes.

The attempt append is a hard gate: no prepared step runs if operator intent cannot be recorded.
After step execution, sidecar and outcome are separate durable writes. Both are attempted even if
the first one fails, and a combined error reports publication failures without hiding the step
failure. This does not claim cross-file atomicity, but it prevents an ordinary step failure from
short-circuiting either required write.

## Step result representation

- Use an internal result carrying `applied_steps: Vec<String>`.
- The failure case additionally carries `failed_step` and `reason`.
- Descriptions come from `PreparedStep::description`, not shell text.
- This avoids duplicating arbitrary commands in structured outcome fields.
- The Attempted row still includes the full proposal, so exact commands remain auditable.
- Outcome ordering is the proposal ordering.
- A first-step failure has an empty applied list.
- A clean apply has no failed step or failure reason.

## Operator-visible announcements

- Format a single stable line from the description: `Applying proposal step: {description}`.
- Emit it immediately before command spawn or file read.
- Use a writer-taking internal function so unit tests can capture exact bytes.
- The public CLI path passes standard output.
- Command child stdout/stderr remain inherited as today.
- File edits have no extra output beyond the announcement.

## Correlation source options

### Infer current attempt from filesystem layout

- Published work paths do not encode the active source attempt lease.
- Attempt-private paths are not canonical status/dashboard inputs.
- Rejected.

### Compare with latest triage transition

- Triage transition identifies proposal-generation attempts, not park authority by itself.
- A stale proposal can have a valid triage transition for an older park.
- Rejected.

### Compare with latest parking transition

- Park records already carry the attempt lease that created the durable block.
- The plugin already uses latest Park records to launch triage.
- A latest Unpark explicitly means there is no current Park from that ledger sequence.
- Chosen.

## Lease-aware projection

- Add a core helper that reads the mixed ledger and returns latest current Park leases.
- Ignore malformed lines consistently with the plugin's existing observational scan.
- Replace a ticket's latest entry whenever a newer parking transition is encountered.
- Remove/withhold the lease when the latest transition is not Park.
- Extend `collect_parked_remedies` with an explicit ledger path.
- The remedy still projects from blocked status and canonical disposition.
- The optional proposal projects only when ticket, Pending state, and lease all match.
- Missing ledger evidence yields `proposal: None`, never removal of the remedy itself.
- CLI status/unblock pass `<root>/.lisa/provenance.jsonl`.
- Plugin dashboard, world checks, and triage pass `self.ledger_path`.

## Action-time correlation

- `run_proposal_action` must independently validate the latest Park lease.
- UI filtering is not an authorization boundary.
- Read latest current Park leases from the canonical provenance path.
- Require exact `AttemptLease` equality with `stored.source_attempt_lease`.
- Return the existing “no pending proposal” class of error for a stale sidecar.
- Apply and dismiss both act only on advice attached to the current park.

## Compatibility

- Historical proposal rows deserialize because new fields default.
- Existing action strings remain `proposed`, `applied`, and `dismissed`.
- New strings are `attempted` and `failed` under lowercase serialization.
- Existing StoredTriageProposal JSON remains valid with the expanded enum.
- Matching S-049-07 fixtures gain a real matching Park row; assertions remain intact.
- Other parked remedies do not require a proposal or a Park row to render their ask.
- Failed proposals no longer suppress triage because only Pending can project.

## Failure handling decisions

- Structural sidecar validation happens on read before the attempt row.
- Dynamic file preconditions happen during their step after the attempt row.
- A provenance attempt append failure prevents mutation and returns immediately.
- A command spawn or exit failure produces Failed state and outcome evidence.
- A file read, exact-match, write, or rename failure does the same.
- A sidecar publication failure does not prevent attempting the outcome append.
- An outcome append failure does not prevent attempting the sidecar publication.
- The returned error prioritizes the original step failure, then publication diagnostics.
- Ticket status remains blocked on failure.
- Ticket status reopens only on complete success and durable terminal publications.

## Test strategy

- Core provenance test pins attempted/failed record serialization and round-trip.
- Core triage test confirms Failed sidecar round-trip through write/read.
- Parking test writes a Park row and proves a matching lease projects.
- Parking test changes only the proposal lease and proves it no longer projects.
- Parking test restores the matching lease and confirms current behavior.
- CLI clean-apply test expects Attempted then Applied rows and all step names.
- CLI failure test uses a successful first step and a failing second command.
- It expects both rows, the applied prefix, failed description/reason, and Failed sidecar.
- Writer capture pins both announcement lines and ordering.
- CLI stale-lease test writes a newer Park and proves action is rejected without mutation.
- Plugin triage tests verify a stale proposal does not suppress fresh triage.
- Package tests run before the full workspace test suite.

## Decision summary

Use additive proposal-action evidence, a terminal Failed sidecar, execution-time exact-edit
validation, injected output capture, and a shared latest-Park lease reader feeding both remedy
projection and action validation. This keeps the fix within existing proposal, provenance, and
parking boundaries while making every normal apply attempt observable before mutation.
