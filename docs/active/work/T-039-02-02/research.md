# Research: T-039-02-02

## Scope

- The ticket targets the eight `State::check_*_signals` loops in
  `crates/lisa-plugin/src/lib.rs`.
- The requested change is structural: introduce one typed ingestion boundary.
- Runtime behavior is constrained by the characterization suite added by
  T-039-02-01.
- The suite must pass unchanged.
- Poll order and attempt admission are explicit acceptance constraints.
- Provider payload, deletion, and lease differences must remain visible.
- The ticket does not request hook, adapter, CLI, or file-format changes.

## Existing module organization

- `crates/lisa-plugin/src/lib.rs` contains the plugin entry point and scheduler
  state.
- The same file contains signal filename parsing and all eight consumers.
- `adapter.rs` describes provider capabilities and provider launch behavior.
- `codex_ack.rs` parses and validates raw Codex `UserPromptSubmit` payloads.
- `pane_name.rs` and `ui.rs` are unrelated focused modules.
- There is no signal-ingestion module today.
- `AttemptLease` is defined in `lisa-core` and imported into the plugin.
- Signal files are runtime filesystem records under `State::signal_dir`.

## Shared filename grammar

- Current pane-scoped names have the form `pane-<u32>.<suffix>`.
- `pane_id_from_signal_filename` implements this grammar near the top of
  `lib.rs`.
- It accepts an `OsStr`, requires UTF-8, strips `pane-`, strips the requested
  suffix, and parses the remainder as `u32`.
- Heartbeat uses `.heartbeat`.
- Process start uses `.started`.
- Shell readiness uses `.shell-ready`.
- Codex acknowledgement uses `.ack`.
- Awaiting-human uses `.awaiting`.
- Idle uses `.idle` and also accepts legacy `<ticket-id>.idle` names.
- Transition consumes both `.stopped` and `.cleared` in one scan.
- Error uses `.error`.

## Poll ordering

- `poll_tick` invokes heartbeat first.
- Awaiting-human is second.
- Ready assignments are delivered between awaiting and process start.
- Process start is the third signal consumer.
- Shell ready is fourth.
- Codex acknowledgement is fifth.
- Artifact advancement occurs before idle.
- Idle is sixth.
- Transition is seventh.
- Error is eighth.
- Transition timeout, acknowledgement timeout, health, and stale detection run
  after the eight signal consumers.
- The characterization test reads the `poll_tick` source and asserts this exact
  relative order.

## Directory access pattern

- Every consumer independently calls `std::fs::read_dir(&self.signal_dir)`.
- A missing or unreadable directory causes that consumer to return.
- Each consumer uses `entries.flatten()`, silently ignoring individual entry
  errors.
- Each consumer examines `DirEntry::path()`.
- Filesystem iteration order is not sorted or otherwise guaranteed.
- Nonmatching records are left for later consumers or later polls.
- This repeated scan is the duplication that forms the current implicit
  ingestion boundary.

## Lease-bearing records

- Heartbeat, process start, and shell ready carry `AttemptLease` JSON.
- Each first requires a valid pane-scoped filename.
- Each reads the whole file as UTF-8 text.
- Each deserializes the text with `serde_json` into `AttemptLease`.
- Each removes the recognized file after the read/parse attempt.
- A read error, invalid UTF-8, or invalid JSON therefore produces no downstream
  action but still consumes the recognized file.
- Invalid pane filenames are not recognized and are not deleted.
- The ingestion step currently parses the lease but does not decide whether it
  is current.

## Heartbeat admission

- Heartbeat performs additional admission in the consumer.
- The pane must correspond to an existing agent slot.
- The slot ticket must equal the candidate lease ticket.
- The slot lease must equal the complete candidate lease.
- The candidate must also equal the scheduler's current lease for its ticket.
- Rejected leases have no state effect after their file is consumed.
- Admitted leases refresh pane and thread activity.
- Admitted leases clear the attention notification debounce.
- Admitted leases clear awaiting-human state.

## Process-start admission

- Process start passes the parsed candidate to `acknowledge_process_start`.
- That method owns seat-state and generation admission.
- Only the exact current `Starting` lease promotes the seat.
- Promotion reaches `ReadyForAssignment`, not ticket ownership.
- The separation makes process readiness observable across a scheduler boundary.

## Shell-ready admission

- Shell ready passes the parsed candidate to `acknowledge_shell_ready`.
- The current time is captured at dispatch.
- The downstream method validates the exact reset successor lease.
- A predecessor or unrelated generation is consumed without relaunch.
- An admitted successor permits the bounded same-pane startup relaunch.

## Raw provider payload records

- Codex acknowledgement is the only raw provider-payload consumer.
- Its filename remains pane-scoped.
- It reads the complete file into a `String` without deserializing at scan time.
- It deletes the recognized file before downstream acknowledgement admission.
- Read failure still results in deletion and no downstream action.
- `acknowledge_codex_assignment` delegates payload meaning to `codex_ack.rs`.
- That parser requires the native hook event shape and exact embedded assignment
  tag.
- The tag contains ticket and generation identity but is not an `AttemptLease`
  file format.
- Successful admission promotes the seat to owned, refreshes activity, and logs
  acknowledgement.
- Stale or malformed provider payloads remain one-shot no-ops.

## Presence-only pane records

- Awaiting, stopped, cleared, and error ignore file contents.
- Their bodies may contain arbitrary provider text.
- Awaiting requires a valid pane filename before deleting the file.
- Awaiting inserts the pane into `awaiting_human` and logs only on first insert.
- Awaiting intentionally does not refresh activity.
- Error also requires a valid pane filename before deletion.
- Error deletes before inspecting seat or thread state.
- Error may fail assignment recovery or reclaim a matching running thread.
- Unknown and idle panes consume error records harmlessly.

## Transition record handling

- Stopped and cleared share one consumer and one directory scan.
- The consumer first obtains a UTF-8 filename.
- It requires the broad `pane-` prefix.
- It then branches on `.stopped` or `.cleared` suffix.
- It deletes a broadly recognized suffix before parsing the pane number.
- Consequently `pane-not-a-number.stopped` is deleted.
- This differs from awaiting and error, which parse the pane before deletion.
- A valid stopped or cleared record refreshes pane activity.
- Stopped dispatches to `handle_stopped_signal`.
- Cleared dispatches to `handle_cleared_signal`.
- Idle records remain untouched by the transition consumer.

## Idle record handling

- `check_idle_signals` clears `idle_alerts` before accessing the directory.
- It recognizes every UTF-8 filename ending in `.idle`.
- It deletes the file immediately after that broad suffix match.
- Pane-scoped idle then parses the pane number.
- A malformed pane number is therefore consumed and ignored.
- A valid pane must resolve to a slot whose transition state is `Idle`.
- Pane-scoped idle refreshes activity before resolving its assigned ticket.
- A slot without a ticket produces no further action.
- Any non-pane `.idle` filename is treated as a legacy ticket identifier.
- Legacy idle does not refresh a pane activity clock because it has no pane.
- Downstream behavior depends on the running thread and current RDSPI phase.
- Implement idle attempts `progress.md` admission and advances to Review.
- Other active phases require the matching phase artifact.
- Idle without an artifact creates an alert and may notify for pane-scoped input.
- Review completion can request the final completion transaction.

## Characterization boundary

- `crates/lisa-plugin/src/tests/signal_consumer_characterization.rs` contains 11
  focused tests.
- The tests are a child of the `lib.rs` unit-test module and can access private
  scheduler state.
- The suite asserts the exact relative poll order.
- It covers deletion of recognized but malformed or inapplicable signals.
- It covers the legacy filename matrix for all eight consumers.
- Only idle accepts the legacy ticket-named family.
- It covers positive state effects for each consumer.
- It distinguishes lease JSON, raw acknowledgement JSON, and ignored bodies.
- The acceptance criterion requires this file to remain unchanged.

## Existing tests outside the characterization suite

- `lib.rs` has focused tests for each signal consumer and state transition.
- Startup tests cover malformed and stale process-start records.
- Shell recovery tests cover exact successor admission.
- Codex acknowledgement tests cover generation and ticket tags.
- Idle tests cover multiple phase and artifact combinations.
- Transition tests cover waiting-for-stop and waiting-for-clear states.
- Error tests cover running, idle, recovery, and mixed-provider cases.
- A parser test covers pane filename grammar, including non-UTF-8 on Unix.
- A cost probe populates the signal directory and checks selective scanning.

## State and ownership constraints

- Signal ingestion is filesystem-facing and provider-facing.
- State mutation after ingestion belongs to `State` methods.
- Attempt currency is scheduler state, not a property discoverable from a file
  alone.
- Therefore lease deserialization and lease admission are separate operations.
- Provider acknowledgement interpretation already has a dedicated module.
- Presence records must not acquire invented payload semantics.
- Legacy idle naming must not spread to other signal types.
- Delete-before-admission prevents stale or malformed records replaying.
- Poll order is semantically relevant even if scanning is factored out.

## Repository and workflow constraints

- The working tree already contains Lisa-owned changes to the ticket and
  provenance ledger.
- Those files are not ticket-owned source edits and must be preserved.
- Phase artifacts belong only in the attempt-private work directory.
- Ticket frontmatter must not be edited manually.
- Ticket-owned source changes must be committed with `lisa commit-ticket`.
- Exact repository-relative include paths are mandatory.
- Ordinary `git add` and `git commit` are prohibited for this ticket.
- Required verification includes the unchanged characterization suite,
  workspace tests, and Clippy.

## Observed constraints summary

- One boundary must support multiple typed record families.
- The boundary must retain per-family filename recognition.
- The boundary must retain per-family deletion timing.
- Lease parsing must not become lease currency admission.
- Raw Codex provider payloads must remain raw at the ingestion edge.
- Presence-only signals must remain payload-free.
- Idle must preserve its pane and legacy ticket targets.
- Transition must preserve its combined stopped/cleared scan.
- The eight `check_*_signals` calls must retain their current order.
- Existing downstream scheduler methods remain the authorities for effects.
