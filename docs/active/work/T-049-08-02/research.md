# Research: proposal apply records first

## Ticket and workflow position

- Ticket `T-049-08-02` starts in Research and is a stable-0.4.4 blocker.
- The work is limited to proposal application durability and proposal/park correlation.
- The parent story promises that every operator apply is recorded, including partial failures.
- The same story promises that advice belongs to the park against which it was prepared.
- Phase artifacts belong in this attempt-private directory.
- Lisa, rather than this attempt, advances ticket phase/status and publishes admitted artifacts.
- Ticket-owned source changes must be committed with `lisa commit-ticket` and exact paths.
- The ordinary index already contains Lisa-managed changes and must remain untouched.

## Proposal model

- `crates/lisa-core/src/triage.rs` owns proposal and sidecar data types.
- `TriageProposal` carries a summary, recommendation, and ordered prepared steps.
- A prepared step is either a shell command or an exact-text file edit.
- Each step has a human-readable description exposed by `PreparedStep::description`.
- `PreparedStep::display` combines that description with the command or edited path.
- Commands are validated only for visible, nonempty text.
- File edits require a safe relative path and nonempty, different old/new text.
- `TriageProposal::validate` requires at least one step.
- `StoredTriageProposal` adds ticket identity, source attempt lease, and state.
- Sidecar validation already requires the lease ticket to equal the proposal ticket.
- Sidecar validation already requires a positive source attempt id.
- Current states are Pending, Applied, and Dismissed.
- There is no failed terminal state, so an unsuccessful apply cannot leave Pending cleanly.
- Sidecars are written atomically through a temporary file and rename.
- The sidecar filename is `triage-proposal.json` beneath the ticket work directory.

## Proposal provenance

- `crates/lisa-core/src/provenance.rs` owns the mixed JSONL ledger vocabulary.
- `ProposalActionRecord` is an untagged ledger member distinguished by record fields.
- It contains schema/seal, ticket, source lease, action, actor, optional proposal, and time.
- Current proposal actions are Proposed, Applied, and Dismissed.
- Proposed rows are agent-authored and include the full proposal.
- Applied and Dismissed rows are operator-authored and omit the proposal.
- There is no apply-attempt action.
- There is no failed apply action.
- There is no step count or step outcome data.
- `append_proposal_action_record` uses the common append-only JSONL writer.
- The writer creates the parent and appends one serialized row plus newline.
- Existing mixed-ledger deserialization relies on additive, distinguishable record shapes.
- Additive optional fields preserve older serialized rows.
- New action variants preserve existing Proposed/Applied/Dismissed spellings.
- Provenance tests construct proposal records directly, so new required fields affect fixtures.

## CLI action path

- `crates/lisa-cli/src/main.rs` routes `lisa proposal apply|dismiss` to `proposal.rs`.
- `run_proposal_action` loads resolved config and scans the ticket board.
- It requires the ticket to have blocked status.
- It parses the canonical review disposition.
- It requires an operator-owned block before allowing either operator action.
- It reads the proposal sidecar from the resolved work directory.
- Current sidecar filtering checks ticket id and Pending state only.
- It does not compare the sidecar source lease with the active Park ledger row.
- Apply currently calls `validate_apply` for all steps before execution.
- `validate_apply` validates step syntax and exact-text file-edit preconditions.
- Apply then calls `apply_steps` for the full ordered list.
- Commands run through `/bin/sh -c` in the repository root.
- Command output is inherited, but the command itself is not announced.
- File edits reread the destination and use `replacen(old, new, 1)`.
- The file-edit execution path does not recheck that old text still occurs once.
- An earlier command can therefore invalidate a later edit after prevalidation.
- In that case `replacen` can publish unchanged bytes and report success.
- A nonzero command returns immediately from `apply_steps`.
- The caller therefore never reaches provenance or sidecar publication on that failure.
- Successful execution sets the sidecar to Applied in memory.
- Dismiss sets it to Dismissed without step execution.
- A single terminal action row is built only after the match completes successfully.
- That row is appended before the sidecar is written.
- Apply then reopens the ticket after ledger and sidecar writes succeed.
- Dismiss leaves the original ticket blocked.
- Existing unit tests cover one clean file-edit apply and dismiss.
- They assert terminal ledger strings but not row order or step evidence.
- Tests do not exercise multiple steps or a mid-list failure.
- Tests do not capture the operator-visible step announcements.

## Park provenance and proposal projection

- `ParkingTransitionRecord` in provenance carries the exact `AttemptLease`.
- Park and Unpark records share the record shape and differ by `record_type`.
- The plugin records Park rows when a blocked Review attempt becomes durable.
- The plugin can reconstruct the latest parking transition per ticket from the ledger.
- `State::latest_parking_transitions` currently implements that scan privately.
- The latest transition is authoritative evidence for which attempt owns the current park.
- Ticket status remains scheduling authority, so projection still begins with blocked tickets.
- `crates/lisa-core/src/parking.rs` owns `collect_parked_remedies`.
- It parses each blocked ticket's canonical Review disposition.
- It projects operator/world/agent owner, ask, reason, check, and optional proposal.
- The proposal subprojection reads the sidecar and checks ticket id plus Pending only.
- `collect_parked_remedies` currently receives tickets and work directory only.
- It has no ledger path and therefore cannot discover the current Park lease.
- CLI status uses this projection for “Waiting on you.”
- CLI unblock and world recheck also use the projection for canonical remedies.
- The plugin dashboard uses the same projection for waiting items.
- The plugin triage scheduler also uses it to decide whether proposal generation is suppressed.
- `request_operator_triage` separately reads latest Park and triage transitions.
- It skips a remedy whenever `remedy.proposal.is_some()`.
- A stale Pending sidecar therefore suppresses fresh triage today.
- A stale Pending sidecar also renders in CLI/plugin operator advice today.
- The action CLI can apply that same stale sidecar today.

## Existing test boundaries

- `parking.rs` tests structured, legacy, excluded, and proposal remedies.
- The proposal fixture currently creates no Park ledger record.
- Its “matching” concept presently means only matching ticket and Pending state.
- Plugin triage tests create a real Park row in their shared state fixture.
- They assert proposal publication and subsequent projection.
- Status and unblock call sites will need any projection signature change.
- Plugin dashboard and triage call sites will need the same change.
- The existing S-049-07 behavior must remain: a genuinely matching Pending proposal renders.
- Proposed agent provenance continues to include the proposal body.
- Dismiss behavior is outside the ordering defect and should stay compatible.

## Constraints and assumptions

- The latest valid parking transition per ticket is the correlation source.
- A latest Unpark row is not a current Park lease.
- Missing or malformed ledger evidence cannot establish a proposal/park match.
- Remedy text may still project without a matching proposal.
- Only the optional proposal subprojection should disappear when correlation fails.
- Failed application must stop at the first failed step.
- “Applied steps” means successfully completed steps before that failure.
- “Failed step” can use the prepared step's stable human description.
- The failure reason should remain available for diagnosis in addition to the step name.
- Step announcements should happen immediately before each step executes.
- File-edit preconditions must be checked at execution time, not only before the list starts.
- A clean apply still reopens the ticket.
- A failed apply should remain blocked but leave the sidecar non-Pending.
- A failed sidecar no longer suppresses a new proposal for a later/current park.
- The ledger is append-only and cannot make sidecar publication transactional.
- Tests can verify the normal write path and explicit ordering of emitted rows.
- Writer injection is the simplest way to pin step announcement strings in unit tests.

## Files in scope

- `crates/lisa-core/src/provenance.rs`: additive apply evidence vocabulary and tests.
- `crates/lisa-core/src/triage.rs`: Failed sidecar state and serialization test coverage.
- `crates/lisa-core/src/parking.rs`: latest-Park lookup, lease-aware projection, tests.
- `crates/lisa-cli/src/proposal.rs`: records-first execution and action correlation.
- `crates/lisa-cli/src/status.rs`: pass the configured ledger to projection.
- `crates/lisa-cli/src/unblock.rs`: pass the configured ledger to projection.
- `crates/lisa-plugin/src/lib.rs`: pass the state ledger to projection and update fixtures.
- No new module is required.
- No ticket or shared work artifact is edited directly by this attempt.
