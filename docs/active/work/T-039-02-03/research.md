# Research: T-039-02-03

## Ticket boundary

- The ticket is the third and final task in story `S-039-02`.
- Its title is `lock-ingestion-boundary-regression`.
- Its current phase is Research.
- Its only acceptance criterion requires new regression tests.
- Those tests must detect changes to poll order.
- They must detect changes to attempt admission.
- They must detect changes to provider payload distinctions.
- They must detect changes to deletion distinctions.
- They must detect changes to lease distinctions.
- The full workspace test suite must pass.
- Clippy must pass.
- The existing characterization suite must be retained.
- The ticket follows `T-039-02-02` directly.
- That predecessor introduced the typed ingestion boundary.
- `T-039-02-01` introduced the pre-refactor characterization suite.
- The story intentionally sequences characterization, refactor, then regression lock.
- The story excludes failure/reclaim, timeout/liveness, and publication changes.
- This ticket is therefore test-only unless a test exposes a defect.

## Repository state

- The repository is a Rust workspace.
- The relevant crate is `crates/lisa-plugin`.
- The plugin is a Zellij WASM plugin with native unit-test support.
- The branch head includes the completed predecessor ticket.
- Commit `2b0af33` introduced the typed boundary.
- Commit `6dee27a` published the predecessor's artifacts.
- The ordinary worktree contains Lisa-owned changes to provenance and this ticket.
- Those existing changes are outside this ticket's source ownership.
- No ticket-owned source file is currently modified.
- Phase artifacts must be written under the attempt-private directory.
- Phase and status fields in the ticket must not be edited manually.
- Source commits must use `lisa commit-ticket` with exact include paths.

## Typed ingestion module

- `crates/lisa-plugin/src/signal.rs` owns filesystem ingestion.
- The module is private to `lisa-plugin`.
- `lib.rs` declares it with `mod signal`.
- Scheduler code imports `IdleTarget`, `SignalRecord`, and `SignalRequest`.
- `SignalRequest` is a closed crate-private enum.
- It has eight variants matching the eight consumer scans.
- The variants are `Heartbeats`, `ProcessStarts`, and `ShellReady`.
- They also include `CodexAcknowledgements`, `Awaiting`, and `Idle`.
- The final variants are `Transitions` and `Errors`.
- `SignalRecord` is a closed crate-private enum.
- It has nine variants because transitions split stopped from cleared.
- `Heartbeat` carries `pane_id` and `AttemptLease`.
- `ProcessStarted` carries `pane_id` and `AttemptLease`.
- `ShellReady` carries `pane_id` and `AttemptLease`.
- `CodexAcknowledgement` carries `pane_id` and raw `String` payload.
- `Awaiting` carries only `pane_id`.
- `Idle` carries an `IdleTarget`.
- `Stopped`, `Cleared`, and `Error` carry only `pane_id`.
- `IdleTarget` distinguishes `Pane(u32)` from `LegacyTicket(TicketId)`.
- These enum shapes encode provider and authority differences at compile time.

## Ingestion operation

- `signal::ingest` accepts a directory and one `SignalRequest`.
- It returns a `Vec<SignalRecord>`.
- Each invocation performs its own `read_dir` call.
- A missing or unreadable directory returns an empty vector.
- Directory entries that cannot be read are skipped.
- Each path is passed to request-specific recognition.
- Nonmatching paths remain in the directory.
- A consumer receives only records belonging to its request.
- Directory iteration order is not promised.
- Tests comparing multiple records must therefore normalize ordering.
- Per-request scans preserve the existing on-demand polling boundary.
- Later consumers can see records created after earlier consumers run.

## Lease-bearing records

- Heartbeat recognizes exact `pane-<u32>.heartbeat` filenames.
- Process start recognizes exact `pane-<u32>.started` filenames.
- Shell readiness recognizes exact `pane-<u32>.shell-ready` filenames.
- All three use the shared `ingest_lease` helper.
- The helper reads UTF-8 text.
- It deserializes that text as `AttemptLease` JSON.
- It removes a recognized file after the read/parse attempt.
- Invalid JSON produces no typed record.
- Unreadable content produces no typed record.
- Both failures still consume a strictly recognized path.
- Ingestion does not consult slots, threads, or current leases.
- A syntactically valid stale lease becomes a typed record.
- Currency and seat authority remain scheduler responsibilities.

## Provider payload records

- Codex acknowledgement recognizes exact `pane-<u32>.ack` filenames.
- It reads the payload as raw UTF-8 text.
- It does not parse the text as an `AttemptLease`.
- It does not parse the provider acknowledgement schema.
- It deletes a recognized path after the read attempt.
- Valid UTF-8 produces a record even when the JSON is malformed.
- The scheduler passes the raw string to `acknowledge_codex_assignment`.
- That downstream method owns provider-specific tag admission.
- This is intentionally different from lease-bearing ingestion.

## Presence records

- Awaiting recognizes exact `pane-<u32>.awaiting` filenames.
- Error recognizes exact `pane-<u32>.error` filenames.
- Neither reads or parses the file body.
- Both emit pane-only records.
- Stopped and cleared also emit pane-only records.
- Presence-only records cannot accidentally import payload semantics.
- Arbitrary file bodies are irrelevant to these record variants.

## Filename and deletion distinctions

- Six requests use strict pane-first recognition.
- Those are heartbeat, process start, shell ready, acknowledgement, awaiting, and error.
- Strict recognition parses the exact pane grammar before deletion.
- A malformed pane number is not recognized and remains on disk.
- A recognized strict path is one-shot even if payload acquisition fails.
- Idle uses broad `.idle` suffix recognition.
- Idle deletes after the suffix match and before target parsing.
- `pane-seven.idle` is deleted but emits no record.
- A non-pane idle stem becomes a legacy ticket target.
- No other request accepts legacy ticket filenames.
- Transitions use broad stopped/cleared suffix recognition after `pane-`.
- They delete before parsing the pane number.
- `pane-seven.stopped` is deleted but emits no record.
- Unrelated files remain available to their owning consumer.

## Scheduler consumers

- All eight `check_*_signals` methods live in `lib.rs`.
- Every method now calls `signal::ingest`.
- Consumers pattern-match the typed variant they requested.
- Filesystem reads and deletes no longer occur in those loops.
- Heartbeat performs explicit current-attempt admission.
- It checks the slot's ticket identity.
- It checks the slot's complete attempt lease.
- It checks the scheduler's current lease registry.
- Only an admitted heartbeat updates activity and clears gates.
- Process start delegates admission to `acknowledge_process_start`.
- Shell ready delegates admission to `acknowledge_shell_ready`.
- Codex acknowledgement delegates provider admission downstream.
- Awaiting has no attempt payload and records a pane gate.
- Idle resolves pane ownership or the historical ticket target.
- Transition dispatches typed stopped and cleared effects.
- Error uses running-thread authority after ingestion.

## Poll order

- `State::poll_tick` begins signal work with heartbeat.
- Awaiting runs second among signal consumers.
- `deliver_ready_assignments` is interleaved next.
- Process start is the third signal consumer.
- Shell ready is fourth.
- Codex acknowledgement is fifth.
- `check_artifact_advances` is then interleaved.
- Idle is the sixth signal consumer.
- Transition is seventh.
- Error is eighth.
- Transition timeouts follow error consumption.
- Assignment acknowledgement timeouts follow transition timeouts.
- The interleaving is part of scheduler behavior, not just formatting.
- Existing characterization asserts relative order of the eight consumers.
- It does not assert both non-signal calls interleaved between them.

## Existing characterization suite

- The file is `src/tests/signal_consumer_characterization.rs`.
- It contains eleven tests and 399 lines.
- It was added before the boundary refactor.
- It is included under the main `#[cfg(test)] mod tests`.
- It asserts relative poll consumer order by inspecting `lib.rs` source.
- It exercises deletion before failed payload or state admission.
- It proves idle alone accepts legacy ticket filenames.
- It tests current-lease heartbeat admission and effects.
- It tests exact starting-lease process admission.
- It tests exact reset-successor shell readiness.
- It tests exact Codex acknowledgement tag admission.
- It tests body-free awaiting behavior.
- It tests legacy idle behavior.
- It tests stopped presence behavior.
- It tests error presence and reclaim behavior.
- The predecessor retained this file byte-for-byte.
- The current ticket explicitly requires retaining it again.

## Existing boundary unit tests

- `signal.rs` contains seven focused unit tests.
- One test combines typed heartbeat parsing and malformed consumption.
- One test contrasts raw acknowledgement payload with awaiting presence.
- One test covers strict invalid-name retention.
- One test covers pane and legacy idle targets.
- One test covers transition co-scanning and malformed deletion.
- Two tests cover exact filename parsing, including non-UTF-8 on Unix.
- These tests are close to implementation details.
- They do not enumerate every `SignalRequest` to exact `SignalRecord` mapping.
- They do not explicitly combine syntactic ingestion with stale-attempt rejection.
- They do not lock the complete interleaved poll seam.

## Test integration conventions

- Plugin tests are native unit tests under `#[cfg(test)]` in `lib.rs`.
- Additional source test modules live in `crates/lisa-plugin/src/tests`.
- Such modules use `use super::*` to access private scheduler fixtures.
- The existing characterization module establishes this convention.
- A sibling regression module can access private `signal` types through imports from `lib.rs`.
- It can reuse `fresh_slot` and `install_current_attempt` test helpers.
- It can construct a `State` with a temporary signal directory.
- Temporary filesystem fixtures use the `tempfile` dev dependency.
- JSON fixtures use `serde_json`, already present in the crate.
- Source-structure assertions use `include_str!("../lib.rs")`.

## Constraints and assumptions

- The characterization suite must not be edited or renamed.
- The new suite should be visibly post-refactor regression coverage.
- Tests should assert behavior rather than duplicate implementation algorithms.
- Filesystem ordering must not make assertions flaky.
- Current lease admission must remain downstream of typed ingestion.
- Raw provider payload must not be normalized into a lease.
- Presence records must not acquire body fields.
- Broad and strict deletion rules must remain distinguishable.
- Legacy idle support must remain isolated to idle.
- No public API is required for the tests.
- No production behavior change is expected.
- Workspace formatting and Clippy rules apply to test code.
- The ordinary Git index must remain untouched.
