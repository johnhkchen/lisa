# Review: proposal apply records first

## Outcome

The ticket is ready to complete. Proposal application now records operator intent before any
prepared mutation, records a terminal outcome for success or failure, announces every step before
execution, and moves the sidecar out of Pending on either outcome. Proposal rendering, action,
and triage suppression are all correlated to the latest current Park attempt lease.

## Source changes

### `crates/lisa-core/src/provenance.rs`

- Added `ProposalAction::Attempted`.
- Added `ProposalAction::Failed`.
- Extended `ProposalActionRecord` additively with:
  - `step_count`;
  - `applied_steps`;
  - `failed_step`;
  - `failure_reason`.
- All new fields use serde defaults and omission rules.
- Historical proposal rows remain deserializable.
- Existing `proposed`, `applied`, and `dismissed` action spellings are unchanged.
- Attempted rows can carry the full proposal and total step count.
- Applied rows name the full ordered applied-step list.
- Failed rows name the successful prefix, failed step, and diagnostic reason.
- Added JSON field/action pinning and round-trip coverage.

### `crates/lisa-core/src/triage.rs`

- Added `ProposalState::Failed`.
- Failed is a terminal state distinct from operator dismissal.
- Existing sidecar documents remain compatible.
- Added atomic write/read round-trip coverage for Failed.

### `crates/lisa-core/src/parking.rs`

- Added `latest_park_attempt_leases` for observational ledger correlation.
- It reads mixed provenance rows and retains only each ticket's latest current Park lease.
- A later Unpark removes the current lease.
- Missing or malformed ledger evidence fails closed for proposal projection.
- `collect_parked_remedies` now receives the provenance ledger path.
- Remedy asks/reasons/checks still project from blocked ticket status and disposition.
- The optional proposal projects only when:
  - sidecar ticket id matches;
  - sidecar state is Pending;
  - sidecar source lease equals the latest current Park lease.
- Matching proposal behavior remains unchanged.
- Added matching, mismatching, and Unpark regression coverage.

### `crates/lisa-cli/src/proposal.rs`

- Preserved the public `run_proposal_action` interface.
- Added an internal writer-taking entrypoint for deterministic output tests.
- Apply and dismiss now reject a sidecar from a stale Park attempt.
- Apply appends an Attempted row before invoking the prepared-step executor.
- The Attempted row includes operator actor, proposal body, and step count.
- No prepared mutation runs if that append fails.
- Every step emits `Applying proposal step: <description>` and flushes before execution.
- Command child output behavior remains inherited from the CLI process.
- The executor records the ordered descriptions of steps that completed.
- Exact-text file edits recheck their precondition immediately before replacement.
- An earlier command that invalidates a later file edit now produces a real failure.
- Clean execution publishes Applied sidecar and Applied outcome evidence.
- Failed execution publishes Failed sidecar and Failed outcome evidence.
- Both sidecar and outcome writes are attempted after execution.
- A clean apply reopens the ticket after terminal publications succeed.
- A failed apply leaves the ticket blocked and returns the execution diagnostic.
- Dismiss remains Dismissed and retains the original park.

### `crates/lisa-cli/src/status.rs`

- Passed the canonical `.lisa/provenance.jsonl` path into parked-remedy projection.
- No output formatting changed.

### `crates/lisa-cli/src/unblock.rs`

- Passed the canonical ledger path for single-ticket and world recheck projections.
- Check sandboxing and ticket reopen behavior are unchanged.

### `crates/lisa-plugin/src/lib.rs`

- Passed `State.ledger_path` to every parked-remedy projection.
- Dashboard advice now excludes stale-Park proposals.
- Operator triage no longer treats a stale proposal as suppression evidence.
- Proposed agent rows initialize the new additive evidence fields neutrally.
- The triage ledger reducer treats an operator Failed action as resetting the prior same-Park
  triage guard.
- A subsequent Triage Started row consumes that reset in ledger order.
- This permits exactly one new first-responder pass after failed apply without poll-driven loops.
- Added regressions for matching suppression, stale-Park re-triage, and failed-apply re-triage.

## Acceptance evidence

### Mid-list failure is always recorded

- `mid_list_failure_records_landed_and_failed_steps_and_leaves_failed_sidecar` prepares two steps.
- The first command mutates `prepared.txt` and invalidates the second exact-text edit.
- The test proves the first mutation remains on disk.
- It proves the ledger contains Attempted followed by Failed.
- Attempted carries `step_count: 2`.
- Failed carries the first description in `applied_steps`.
- Failed carries the second description in `failed_step`.
- Failed carries the exact-match diagnostic.
- The sidecar is Failed, not Pending.
- The ticket remains blocked.

### Clean apply leaves attempt and Applied rows

- `apply_executes_prepared_edit_records_and_reopens` parses proposal action records.
- It proves the order is Attempted then Applied.
- It proves the attempt actor is operator.
- It proves the attempt carries the proposal and step count.
- It proves Applied names the completed step.
- It proves the sidecar is Applied.
- It proves the prepared edit landed and the ticket reopened.

### Every step is announced at apply time

- Clean apply pins `Applying proposal step: Use the calibrated bound.`.
- Mid-list failure pins both lines in execution order.
- The implementation writes and flushes each line immediately before the matching step.
- The announcement contains the stable prepared-step description rather than an inferred status.

### Stale proposal is neither rendered nor actionable

- Core projection tests prove mismatched source lease yields `proposal: None`.
- Matching lease restores the exact proposal.
- Latest Unpark removes correlation evidence.
- CLI stale-lease test appends a newer Park and proves action is rejected before mutation or
  proposal-action provenance.

### Stale proposal does not suppress triage

- `proposal_suppresses_triage_only_for_the_current_park_lease` proves matching behavior is
  unchanged and a stale sidecar launches triage against the newer lease.
- `failed_apply_resets_same_park_triage_guard_exactly_once` proves Failed allows one fresh pass
  against the same Park and a new Started/in-flight state prevents a duplicate launch.

### Existing behavior remains covered

- Existing S-049-07-era status, dashboard, proposal publication, dismiss, and parked UX tests
  remain enabled and passed.
- No prior assertion was removed or weakened.
- The full workspace suite passed after all changes and concurrent ticket integration settled.

## Verification

- `cargo test -p lisa-core` — passed.
- `cargo test -p lisa-cli proposal::tests` — 4 passed.
- `cargo test -p lisa-plugin proposal_suppresses_triage_only_for_the_current_park_lease` — passed.
- `cargo test -p lisa-plugin failed_apply_resets_same_park_triage_guard_exactly_once` — passed.
- `cargo test --workspace` — passed before final review.
- `just check` — passed after final integration:
  - `cargo check -p lisa-plugin --target wasm32-wasip1` passed;
  - all workspace tests passed;
  - final plugin result was 439 passed;
  - one pre-existing real-Zellij test remained ignored by its declared environment prerequisites.
- `git diff --check` — passed.

## Commits

- `30b55af` — Add proposal apply evidence and park correlation.
- `c9f3443` — Record proposal apply before executing steps.
- `3defb16` — Correlate triage proposals with current park.
- `bd95482` — Retry triage after failed proposal apply.

Every ticket-owned source unit was committed with `lisa commit-ticket` and exact include paths.
No ticket-owned source file remains staged, modified, or untracked.

## Compatibility and risk assessment

- Ledger schema changes are additive and defaulted.
- Existing enum spellings are preserved; only new action/state values were introduced.
- Proposal visibility now requires Park evidence, which intentionally fails closed for stale or
  uncorrelatable sidecars.
- Ticket status remains scheduling authority and remedy text still renders without a proposal.
- Cross-file sidecar/ledger publication cannot be fully transactional with the current storage
  model. The implementation attempts both writes and reports either failure; ordinary step
  failures satisfy the ticket's durable evidence requirements.
- Ticket reopening remains after terminal publication, so a successful mutation cannot be
  presented as schedulable before evidence is written.

## Open concerns

- None blocking.
- A future schema version could model apply attempt/outcome as a dedicated record family, but the
  additive ProposalActionRecord representation is compatible and sufficient here.
- A future transaction layer could atomically couple sidecar and JSONL publication; this ticket
  does not introduce such an abstraction and no current acceptance criterion requires it.

## Disposition

Pass. Both acceptance groups are covered by focused regressions and the repository's full gate.
