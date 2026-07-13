# Research: field repro regression guard

## Ticket and workflow boundary

- Ticket `T-043-03-03` starts in Research.
- It is the final ticket in story `S-043-03`.
- Its prerequisites are `T-043-03-01`, `T-043-03-02`, and `T-043-02-02`.
- Those prerequisites have already landed the product behavior this ticket must
  preserve.
- This ticket asks for a deterministic regression guard, not a new reporting
  surface or another attribution algorithm.
- The private artifact directory is
  `.lisa/attempts/T-043-03-03/1/work/`.
- Lisa owns phase/status transitions and final artifact publication.
- Ticket-owned source changes must be committed with `lisa commit-ticket` and
  exact repository-relative include paths.

## Incident contract

- `docs/active/epic/E-043.md` records the field defect.
- A native Claude pane retains process-birth environment across `/clear` reuse.
- The old capture writer trusted `LISA_TICKET_ID` from that environment.
- Later tickets on the recycled pane therefore inherited the first ticket's
  key.
- The old writer used `std::fs::write` on `<key>.usage.json`.
- Every later Stop replaced the earlier file for the same stale key.
- The field run observed six such overwrites.
- A missing ticket key fell back through pane identity to a mutable `last`
  bucket.
- Other no-capture paths returned success without writing evidence.
- The deployed hook discarded stderr and forced success with
  `2>/dev/null || true`.
- The resulting state could be both incomplete and incorrectly attributed.
- Epic E-043 limits the repair to correctness of capture and provenance state.
- Claude/Codex cache-token parity and new cost reporting are explicitly out of
  scope.

## Existing capture fact contract

- `crates/lisa-core/src/capture.rs` owns `CaptureRecord`.
- A record contains `pane_id`, `session_id`, `captured_at`, `input_tokens`, and
  `output_tokens`.
- It deliberately contains no ticket identifier.
- `append_capture_record` serializes compact JSON and appends a newline.
- The provider capture ledger is `.lisa/<client>/captures.jsonl`.
- Multiple observations on one pane survive as separate physical rows.
- Capture time uses epoch seconds, matching provenance interval resolution.
- The record is a pre-attribution observation: the hook knows pane, provider
  session, time, and totals, but not authoritative ticket ownership.

## Existing CLI writer

- `crates/lisa-cli/src/capture_usage.rs` implements native Stop capture.
- `run_capture_usage` currently reads the Stop payload directly from stdin.
- It reads `LISA_AGENT_CLIENT` to choose Claude or Codex parsing.
- It reads `LISA_PANE_ID` as the physical pane fact.
- It does not read `LISA_TICKET_ID`.
- Claude transcript parsing sums usage from assistant messages.
- Claude input totals include fresh, cache-creation, and cache-read tokens.
- Codex transcript parsing selects the latest cumulative token-count event.
- A successful observation appends one `CaptureRecord`.
- The capture timestamp is obtained directly from `SystemTime::now()`.
- Missing or empty `session_id` is an actionable input error.
- Missing or invalid `LISA_PANE_ID` is an actionable input error.
- Missing, unreadable, or usage-empty transcripts are identified no-capture
  outcomes.
- Those outcomes append to `.lisa/<client>/no-captures.jsonl`.
- Each marker contains pane, session, capture time, and a stable reason.
- Marker reasons are `missing-transcript-path`, `unreadable-transcript`, and
  `empty-transcript`.
- A successfully persisted marker emits a concise stderr notice.
- Persistence failures propagate as command errors.
- `crates/lisa-cli/tests/capture_usage_cli.rs` drives the compiled binary.
- One integration test proves two successful observations append despite a
  stale `LISA_TICKET_ID`.
- Another proves empty and unreadable transcripts append markers and emit
  stderr.
- These tests cover writer behavior but not scheduler attribution.

## Existing plugin attribution consumer

- `crates/lisa-plugin/src/lib.rs` owns terminal provenance emission.
- `State::emit_provenance` builds the current terminal execution interval while
  the thread is still in memory.
- It passes that not-yet-appended record to `State::read_usage`.
- `read_usage` selects the provider-local `captures.jsonl`.
- It reads prior execution records from the provenance ledger.
- Assignment-transition records are intentionally excluded from ownership.
- It filters captures to the current physical pane.
- It leaves captures after the current interval's `ended_at` pending.
- It calls `ownership::owner_at` with prior records plus the current record.
- A capture uniquely owned by the current ticket contributes to checked token
  sums.
- A capture uniquely owned by another ticket is skipped.
- A capture with no unique owner is sent to quarantine.
- Arithmetic overflow returns null totals rather than fabricating a value.
- Capture facts contain no cost observation, so `cost_usd` remains null.
- The completed provenance row is appended after usage is filled.

## Existing pane-time ownership contract

- `crates/lisa-plugin/src/ownership.rs` owns `owner_at`.
- Ownership intervals use inclusive start and end timestamps.
- Records for other panes do not participate.
- Repeated overlapping intervals for the same ticket remain unambiguous.
- Overlap between different ticket IDs fails closed with `None`.
- Timestamps outside every interval also return `None`.
- The result does not depend on record ordering.
- This is the authoritative replacement for environment-keyed attribution.

## Existing quarantine contract

- `crates/lisa-plugin/src/quarantine.rs` owns quarantine persistence.
- Quarantine is provider-local and partitioned by observed session ID.
- Paths use `.lisa/<client>/quarantine/<encoded-session-id>.jsonl`.
- Opaque session bytes are percent encoded before becoming a filename.
- Each quarantine row retains the original `CaptureRecord`.
- It also records the one-based source line from `captures.jsonl`.
- Source-line identity makes repeated ledger scans idempotent.
- Two byte-identical observations on different source lines remain distinct.
- A conflicting prior row for the same source line is an error.
- A new quarantine append emits an `ActivityEvent::Warning`.
- The warning includes client, session, pane, timestamp, and destination.
- The activity event maps to a visible dashboard warning.
- A repeat scan neither appends nor warns again.
- Quarantine persistence failure emits an `ActivityEvent::Error`.
- Quarantined tokens never enter a provenance record.

## Existing regression coverage

- `provenance_recycled_pane_attributes_capture_sums_to_each_ticket` covers two
  tickets on one pane.
- It writes two captures for ticket A and two for ticket B.
- It closes A manually, appends A's provenance, then emits B through the normal
  terminal method.
- It proves each ticket receives its own sum and B appends rather than rewriting
  A.
- It also proves B's later captures are not prematurely quarantined while A is
  closing.
- `provenance_unattributable_capture_is_quarantined_by_session_and_visible`
  covers one unmatched observation.
- It proves null usage, a session-specific quarantine row, a warning, UI
  projection, and rescan idempotence.
- CLI integration coverage separately proves visible no-capture stderr.
- No single fixture reproduces the incident's six overwrite opportunities and
  carries all three repaired outcomes together.
- The present two-ticket fixture is smaller than the field evidence named in
  the ticket.

## Testability boundary

- The plugin crate already has a dev-dependency on `lisa-cli` for transaction
  regressions.
- `lisa-cli` has a library target, but it currently exports only
  `commit_transaction`.
- `capture_usage.rs` is compiled as a private binary module from `main.rs`.
- `run_capture_usage` is coupled to process stdin, environment, current time,
  and process stderr.
- A plugin unit test cannot deterministically invoke that function with several
  synthetic pane-time intervals.
- Mutating process environment in parallel Rust tests would introduce race
  risk.
- Invoking a nested Cargo process from a unit test would be slow and dependent
  on build ordering.
- Directly appending `CaptureRecord` rows would cover the consumer but would not
  connect the field guard to the repaired Stop writer or visible no-capture
  behavior.
- A narrow parameterized writer seam can separate process input acquisition
  from outcome processing without changing the command's external behavior.
- That seam can accept explicit reader, client, pane, timestamp, and diagnostic
  writer values.
- The ordinary command wrapper can continue to supply stdin, environment,
  `SystemTime::now()`, and stderr.
- Exporting this seam only behind a dev feature avoids adding a normal CLI API.

## Determinism constraints

- Epoch-second timestamps require deliberately non-overlapping intervals.
- The test must not depend on wall-clock sequencing between subprocesses.
- Each successful Stop needs distinct token totals so attribution errors are
  visible without relying only on record order.
- Seven sequential tickets on one physical pane create six later writes that
  the old stale-key writer would have directed at the first ticket's file.
- An unmatched successful observation should use its own session ID and a time
  before the first owned interval.
- A no-capture Stop should use another session and a stable empty transcript.
- The no-capture row must not be represented as measured zero usage.
- The old `<first-ticket>.usage.json`, `last.usage.json`, and shared quarantine
  bucket should remain absent.
- Provenance assertions should check the complete ordered set of ticket IDs and
  token totals, not only row count.
- Quarantine assertions should check the encoded session path and original
  observation.
- Visibility assertions should check both Stop stderr and plugin activity for
  the quarantine branch.

## Repository state and ownership

- The ordinary worktree currently contains Lisa-managed changes to
  `.lisa/provenance.jsonl`, `.lisa/completion-journal.jsonl`, and the active
  ticket file.
- Those files are not ticket-owned source changes for this implementation.
- They must not be staged or committed by this ticket.
- Likely ticket-owned paths are limited to CLI testability wiring and the plugin
  regression fixture.
- Existing prerequisite behavior should remain unchanged.
- Verification should include focused CLI/plugin tests and workspace gates.
