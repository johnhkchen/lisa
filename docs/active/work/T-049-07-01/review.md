# Review: block triage proposal

## Disposition

Pass.

Commit `49ed4b2553faebd8cf42495805d8c210e7d976bd` implements the
first-responder proposal flow as a fail-open extension of the existing durable
park machinery. The implementation satisfies both ticket acceptance criteria,
has cross-crate fixture coverage, and passes workspace tests and warning-denying
Clippy when verified from the exact committed snapshot.

## Changes reviewed

### Typed proposal and parked projection

`lisa-core` now owns a typed `TriageProposal` contract. A proposal contains:

- one plain summary sentence;
- a recommended operator action; and
- one or more prepared command or exact file-edit steps.

Validation rejects empty prose, unsafe project-relative paths, and file edits
whose old and new text do not differ. The stored sidecar binds the proposal to
the ticket and source attempt generation and records Pending, Applied, or
Dismissed state.

Sidecars are written through a same-directory temporary followed by rename.
The parked projection attaches advice only when the sidecar is valid, Pending,
and matches the current park lease. Missing, malformed, stale, applied, or
dismissed sidecars leave the established parked remedy unchanged.

### Durable and visible provenance

The additive provenance schema is version seven. Triage transition rows record
Started, Proposed, Failed, TimedOut, or Invalid outcomes together with the
ticket, source generation, configured route, timeout, and timestamps.

Proposal action rows separately record Proposed, Applied, and Dismissed. Agent
creation and explicit operator disposition therefore remain distinguishable.
The Started row is the durable one-attempt-per-generation spend fence, including
across plugin restart. Proposal provenance is appended before the pending
sidecar becomes visible, and operator provenance is appended before a sidecar
state change.

Legacy ledger rows remain readable. Mixed-ledger tests cover the new disjoint
record shapes alongside existing provenance records.

### Configuration and bounded runner

Native configuration accepts `[triage] enabled` and `timeout_secs`, defaulting
to enabled and 120 seconds. Zero is rejected, unknown keys use the existing
warning path, and resolved values flow through generated KDL into plugin
configuration. Bad or legacy runtime maps retain bounded defaults.

The hidden `lisa triage-agent` command runs exactly one configured provider in
read-only mode. Its prompt identifies the ticket, disposition, and cited
evidence, prohibits mutation, and requests the exact typed JSON shape.

Provider output is captured in temporary files so a verbose child cannot block
on pipe back-pressure. A hard deadline polls and kills the provider process
group; timeout exits with code 124 for deterministic plugin classification.
Claude and Codex envelopes are reduced to a final candidate, then core schema
validation occurs before compact JSON is returned.

### Scheduling and fail-open behavior

The plugin considers only latest operator-owned parks. Normal ticket threads
and in-flight triage jobs share the global thread budget, and triage also counts
against the selected provider's configured cap. Route and model come from the
blocked ticket's normal resolution.

The original durable park is completed before triage is requested. Disabled
triage, missing host boundaries, lack of capacity, provenance failure, provider
failure, timeout, and invalid output never delay or alter that park. An existing
pending proposal, in-flight job, or same-generation triage attempt suppresses
repeat spend.

The implementation does not create a synthetic DAG ticket and does not edit
ticket status or Review disposition from an agent result.

### Explicit operator disposition

`lisa proposal apply <ticket>` requires a matching Pending proposal on an
operator-owned block. It validates all prepared file edits before executing the
action list. Commands and edits run only after this explicit invocation. Exact
file replacements require a unique old-text match and publish atomically.

A successful apply records Operator provenance, marks the proposal Applied,
and reopens the ticket for the ordinary workflow. `lisa proposal dismiss
<ticket>` records Operator provenance, marks the proposal Dismissed, and keeps
the original park blocked. A bad proposal therefore costs one dismissal and no
implicit mutation.

Prepared command lists are deliberately not transactional across multiple
steps. This is acceptable because the interface presents exact steps for an
operator decision and executes only after explicit approval; failures are
reported without falsely reopening the ticket.

### Waiting-on-you presentation

CLI status and plugin dashboard use the shared parked projection and render:

1. the first-responder summary;
2. the suggested action;
3. the prepared actions;
4. the original operator ask; and
5. the raw reviewer reason.

Without a proposal, the established ask-first presentation remains unchanged.
UI fixtures pin the proposal ahead of jargon-heavy raw reason text.

## Acceptance-criteria trace

The first criterion is satisfied by fixtures covering a proposal record with
summary, recommendation, and prepared steps; CLI and dashboard presentation;
and triage-attempt plus proposal-creation provenance. Disabled triage preserves
ticket, disposition, and ledger bytes. Failure and real timeout preserve the
already durable park and record terminal attempt states. The one-second fake
provider fixture verifies that timeout is bounded rather than inferred from a
mocked result.

The second criterion is satisfied by the T-046-06-03 regression fixture. The
proposal names the criteria-versus-measured-evidence gap and recommends the
two-sentence calibrated-bound amendment. Apply and dismiss fixtures prove that
both outcomes are explicit Operator actions with durable provenance and the
correct reopen-versus-remain-blocked behavior.

## Verification reviewed

- `cargo fmt --all` passed before the scoped source commit.
- Focused core, CLI, bounded-runner, proposal-action, plugin, and UI tests
  passed during implementation.
- `cargo test -p lisa-plugin --no-fail-fast` passed 433 tests.
- The exact commit was exported to an isolated directory and
  `cargo test --workspace --no-fail-fast` passed across the workspace. The
  single real-Zellij boundary test was ignored because its declared external
  environment was unavailable.
- `cargo clippy --workspace --all-targets -- -D warnings` passed against that
  same exact exported commit.
- `git diff --check` passed after the commit.

## Repository and concurrency review

The source unit was committed only through `lisa commit-ticket` with fourteen
exact ticket-owned include paths. No ordinary index operation was used.

T-049-06-02 ran concurrently and overlaps three CLI integration surfaces. Its
current modifications add the separate Notes command and projections on top of
this commit. Those modifications and its untracked Notes files are not part of
T-049-07-01. Exact-snapshot verification isolates this review from that live
worktree activity while preserving the neighboring ticket's ownership.

## Open concerns

No blocking concerns remain.

The triage provider consumes configured capacity and is provenance-visible,
but it is a one-shot host command rather than a long-lived visible terminal
pane. The route, model, deadline, and terminal result remain inspectable in the
ledger, which meets the ticket's no-hidden-spend requirement without adding a
synthetic parked thread.

Provider advice can still be wrong by design. The safeguards are typed and
bounded output, read-only generation, exact prepared steps, explicit apply,
durable operator attribution, and one dismissal. This preserves the stated
boundary: triage proposes; only the operator disposes.
