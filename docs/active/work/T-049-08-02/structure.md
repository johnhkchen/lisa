# Structure: proposal apply records first

## Modified files

### `crates/lisa-core/src/provenance.rs`

- Extend `ProposalAction` with `Attempted` and `Failed`.
- Keep lowercase serde naming so existing action JSON is byte-compatible.
- Extend `ProposalActionRecord` with additive evidence fields:
  - optional `step_count`;
  - defaulted/omitted-empty `applied_steps`;
  - optional `failed_step`;
  - optional `failure_reason`.
- Proposed rows continue to carry `proposal: Some`.
- Attempted rows also carry `proposal: Some` and `step_count: Some`.
- Applied rows carry `applied_steps` and no failure fields.
- Failed rows carry applied prefix, failed step, and reason.
- Dismissed rows carry no step evidence.
- Update every in-tree struct literal with explicit neutral values.
- Add serialization/round-trip assertions for the new row shapes.

### `crates/lisa-core/src/triage.rs`

- Add `ProposalState::Failed`.
- Preserve current lowercase JSON representation.
- Add or extend sidecar round-trip coverage for the terminal failure state.
- No changes to proposal validation or file publication boundaries.

### `crates/lisa-core/src/parking.rs`

- Import provenance ledger record and parking transition types.
- Import `AttemptLease` and a map type.
- Add a public observational helper:
  - input: provenance ledger path;
  - output: latest current Park attempt lease by ticket;
  - malformed/missing ledger produces no lease rather than a fatal projection error.
- Change `collect_parked_remedies` to receive the ledger path explicitly.
- Compute the Park lease map once per collection call.
- Extend proposal filtering with exact source lease equality.
- Preserve all non-proposal remedy projection behavior.
- Extend proposal fixture setup with a matching Park record.
- Add stale/matching lease assertions without weakening Pending/Dismissed assertions.
- Test latest Unpark behavior if the helper is directly exposed.

### `crates/lisa-cli/src/proposal.rs`

- Import latest Park lease lookup and IO writing support.
- Split public execution from a writer-taking internal implementation.
- Public `run_proposal_action` locks stdout and delegates.
- Validate sidecar lease against the latest Park before either action.
- Keep Dismiss as one terminal action row plus Dismissed sidecar.
- Replace list-level apply prevalidation/application with stepwise execution.
- Add an internal apply outcome type containing applied prefix and failure detail.
- Add one helper to construct proposal action records consistently.
- Append Attempted before calling the step executor.
- Announce each step immediately before execution.
- Recheck exact file-edit preconditions inside that step.
- Publish Applied/Failed sidecar and matching outcome row after execution.
- Reopen only on the Applied branch.
- Preserve useful command/file error text.
- Expand setup to write a matching Park row.
- Add multi-step clean, mid-list failure, output pin, and stale lease tests.

### `crates/lisa-cli/src/status.rs`

- Pass `root/.lisa/provenance.jsonl` to `collect_parked_remedies`.
- No rendering changes.

### `crates/lisa-cli/src/unblock.rs`

- Pass `root/.lisa/provenance.jsonl` in single-ticket unblock projection.
- Pass the same path in world recheck projection.
- No check execution or status semantics change.

### `crates/lisa-plugin/src/lib.rs`

- Pass `self.ledger_path` to every parked-remedy projection.
- This includes world-park detection, operator triage scheduling, and dashboard waiting items.
- Update test-only direct calls with each test state's ledger path.
- Add a triage scheduler regression for a stale Pending proposal.
- Ensure the matching proposal fixture still renders and suppresses duplicate triage.
- Update Proposed action record literals with neutral new evidence fields.

## New files

- No production module is created.
- No new integration-test file is required unless stdout capture proves cleaner at binary level.
- Phase artifacts remain in the attempt-private work directory.

## Deleted files

- None.

## Public interface changes

### Proposal provenance vocabulary

- `ProposalAction::Attempted` is a new serialized action.
- `ProposalAction::Failed` is a new serialized action.
- `ProposalActionRecord` receives additive public fields.
- Downstream exhaustive matches on ProposalAction will need the two new branches.
- Downstream struct literals must initialize the new fields.
- Existing JSON readers using serde defaults remain compatible.

### Proposal sidecar vocabulary

- `ProposalState::Failed` is a new serialized state.
- Pending remains the only renderable/actionable state.
- Applied, Failed, and Dismissed are terminal for the stored proposal.

### Parked remedy collection

- `collect_parked_remedies` gains a provenance ledger path parameter.
- All first-party callers already know the root/state ledger path.
- The ledger remains observational; blocked ticket status remains the initial filter.

## Internal organization

### Latest Park lease reader

- Place in `parking.rs` beside remedy projection because correlation is projection policy.
- Parse each ledger line as `ProvenanceLedgerRecord`.
- Observe only `ParkingTransition` variants.
- Store the latest transition per ticket by line order.
- Return only entries whose latest transition type is Park.
- Clone the record's attempt lease into the result map.

### Apply record constructor

- Centralize schema, seal, record type, ticket, lease, actor, and timestamp creation.
- Let call sites provide action and evidence fields.
- Avoid timestamp reuse requirements; each append represents its actual event time.
- Attempted and outcome rows can share the same second without losing line order.

### Step executor

- Input: root, ordered prepared steps, mutable writer.
- Output: successful ordered descriptions or a failure carrying prefix/step/reason.
- Announcement is written before matching on the step kind.
- Writer failure counts as failure before the prepared mutation begins.
- Command success appends its description.
- File-edit success appends after atomic publication.
- Failed description is always the step whose announcement was emitted.

### Apply finalizer

- Convert executor result to sidecar state and action outcome.
- Attempt sidecar publication and outcome append independently.
- On clean execution, publication errors prevent ticket reopen.
- On failed execution, return an error after required terminal writes are attempted.
- Error formatting can combine execution and publication diagnostics.

## Ordering invariants

1. Ticket/disposition/sidecar/Park validation precedes all ticket mutations.
2. Attempted provenance append precedes all step announcements and execution.
3. A step announcement precedes that step's command or file access.
4. Applied prefix collection follows successful completion of each step.
5. Sidecar terminal state and outcome append follow step execution.
6. Ticket reopen follows successful Applied state and outcome publication.
7. A failed apply never changes the ticket out of Blocked.

## Correlation invariants

1. A proposal's ticket id equals its embedded lease ticket id by sidecar validation.
2. A proposal projects only if the ledger's latest ticket transition is Park.
3. The sidecar lease must exactly equal that Park's attempt lease.
4. A later Unpark invalidates the proposal projection.
5. A later Park from a new attempt invalidates an older sidecar.
6. Action execution repeats the same check independent of rendering.
7. A mismatched Pending sidecar behaves as absent for new triage scheduling.

## Test ownership

- Provenance serialization tests stay in `provenance.rs`.
- Sidecar state tests stay in `triage.rs`.
- Projection correlation tests stay in `parking.rs`.
- Apply ordering/output tests stay in `proposal.rs`.
- Scheduler suppression regression stays in plugin tests.
- Existing package and workspace suites remain the broad regression gates.

## Commit boundaries

1. Core evidence and correlation model/tests:
   - `crates/lisa-core/src/provenance.rs`
   - `crates/lisa-core/src/triage.rs`
   - `crates/lisa-core/src/parking.rs`
2. CLI records-first apply and caller updates:
   - `crates/lisa-cli/src/proposal.rs`
   - `crates/lisa-cli/src/status.rs`
   - `crates/lisa-cli/src/unblock.rs`
3. Plugin projection/scheduler updates and regressions:
   - `crates/lisa-plugin/src/lib.rs`

Each boundary is committed through `lisa commit-ticket` with only exact paths from that unit.
