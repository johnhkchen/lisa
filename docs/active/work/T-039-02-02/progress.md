# Progress: T-039-02-02

## Status

- Research complete.
- Design complete.
- Structure complete.
- Plan complete.
- Implementation complete.
- Focused and repository-wide verification complete.
- Source commit complete.
- Review artifact remains after the isolated source commit.

## Baseline

- Read `CLAUDE.md`, `AGENTS.md`, the ticket, and the full RDSPI workflow.
- Inspected all eight `check_*_signals` consumers.
- Inspected `poll_tick` and its interleaved scheduler operations.
- Inspected the T-039-02-01 research, design, structure, and review handoff.
- Read the complete unchanged characterization suite.
- Observed preexisting Lisa-owned modifications to:
  - `.lisa/provenance.jsonl`;
  - `docs/active/tickets/T-039-02-02.md`.
- Did not edit or revert either Lisa-owned file.
- Confirmed the ordinary index was empty.

Baseline verification:

- Command: `cargo test -p lisa-plugin signal_consumer_characterization`.
- Result: 11 passed, 0 failed.
- This established the unchanged suite was green before source edits.

## Implemented boundary

Created `crates/lisa-plugin/src/signal.rs`.

The module now defines:

- `SignalRequest`, a closed request set matching the eight consumers;
- `SignalRecord`, a typed record set matching the nine concrete signal kinds;
- `IdleTarget`, explicitly separating pane identity from legacy ticket identity;
- `signal::ingest`, the single filesystem-facing operation.

Typed payload distinctions:

- heartbeat carries `AttemptLease`;
- process start carries `AttemptLease`;
- shell readiness carries `AttemptLease`;
- Codex acknowledgement carries a raw provider `String`;
- awaiting carries pane presence only;
- idle carries `IdleTarget` only;
- stopped carries pane presence only;
- cleared carries pane presence only;
- error carries pane presence only.

## Preserved deletion contracts

Strict pane-first recognition remains in place for:

- heartbeat;
- process start;
- shell ready;
- Codex acknowledgement;
- awaiting;
- error.

For those families, an invalid pane filename remains untouched. Once a valid
pane-scoped name is recognized, the file remains one-shot even if a required
payload cannot be read or parsed.

Broad delete-before-pane-parse recognition remains in place for:

- idle;
- stopped;
- cleared.

Malformed recognized idle and transition names are deleted without yielding a
typed record. Unrelated suffixes remain untouched.

## Preserved admission boundaries

- The ingestion module parses lease shape only.
- It has no access to scheduler slots or current leases.
- Heartbeat still checks slot ticket, slot lease, and scheduler current lease.
- Process start still delegates to `acknowledge_process_start`.
- Shell ready still delegates to `acknowledge_shell_ready`.
- Raw Codex provider JSON still delegates to `acknowledge_codex_assignment`.
- Idle phase/artifact admission remains in `State`.
- Error recovery and running-thread authority remain in `State`.

## Routed consumers

Modified `crates/lisa-plugin/src/lib.rs` so all eight loops call the boundary:

1. `check_heartbeat_signals` requests `Heartbeats`.
2. `check_awaiting_signals` requests `Awaiting`.
3. `check_process_start_signals` requests `ProcessStarts`.
4. `check_shell_ready_signals` requests `ShellReady`.
5. `check_codex_ack_signals` requests `CodexAcknowledgements`.
6. `check_idle_signals` requests `Idle`.
7. `check_transition_signals` requests `Transitions`.
8. `check_error_signals` requests `Errors`.

`poll_tick` itself was not restructured. Every call and all interleaved work
remain in their characterized relative positions.

## Test changes

Added seven focused tests inside `signal.rs`:

- valid typed lease plus malformed lease one-shot behavior;
- raw provider payload versus presence distinction;
- strict invalid pane-name retention;
- pane and legacy idle targets plus malformed idle deletion;
- combined stopped/cleared scan plus malformed transition deletion;
- exact pane filename grammar;
- non-UTF-8 pane filename rejection on Unix.

The two parser tests previously in `lib.rs` moved with the parser into the new
module. This produces five net-new tests.

The T-039-02-01 characterization source file was not edited.

## Focused verification

- Command: `cargo test -p lisa-plugin signal::tests`.
- Result: 7 passed, 0 failed.
- Command: `cargo test -p lisa-plugin signal_consumer_characterization`.
- Result: 11 passed, 0 failed.
- The characterization suite passed after the refactor unchanged.

## Repository verification

- Command: `just check`.
- WASM target check: passed.
- Workspace native test suite: passed.
- Plugin unit suite: 308 passed, 0 failed.
- Command: `just lint`.
- Plugin WASM Clippy with `-D warnings`: passed.
- Core Clippy with `-D warnings`: passed.
- CLI Clippy with `-D warnings`: passed.
- Command: `just fmt-check`.
- Result: passed.
- Command: `git diff --check`.
- Result: passed.

## Source inspection

- Confirmed exactly eight `signal::ingest` call sites in the eight consumers.
- Confirmed no direct `read_dir(&self.signal_dir)` remains in those consumers.
- Confirmed `poll_tick` call ordering is unchanged.
- Confirmed the characterization module has an empty diff.
- Confirmed ticket-owned source changes are limited to:
  - `crates/lisa-plugin/src/lib.rs`;
  - `crates/lisa-plugin/src/signal.rs`.
- Confirmed no ticket-owned source path is staged in the ordinary index.

## Deviations from plan

One minor implementation correction occurred during the first focused compile.
The transition recognizer initially retained a borrowed filename slice while
moving its path into `remove_file`, which Rust rejected with E0505. The
recognizer now owns the parsed pane substring before deletion, preserving the
required delete-before-numeric-parse behavior. No product behavior or planned
file scope changed.

The plan anticipated attempt-private artifacts only. During the workflow Lisa
admitted/published phase artifacts into `docs/active/work/T-039-02-02/` as part
of its phase automation. Those shared files were not written directly by this
agent and are not included in the source transaction.

## Source commit

- Command: `lisa commit-ticket` with ticket ID `T-039-02-02`.
- Message: `T-039-02-02: add typed signal ingestion boundary`.
- Exact include: `crates/lisa-plugin/src/lib.rs`.
- Exact include: `crates/lisa-plugin/src/signal.rs`.
- Commit: `2b0af33d080d037e00365be304f07e1620c213c8`.
- The ordinary Git index was not used.
- Both ticket-owned source paths are clean after the transaction.
- The ordinary index remains empty.
- A post-commit characterization run passed 11 of 11 tests.

## Remaining

1. Write `review.md` in the attempt-private work directory.
2. Remain on this ticket and stop for Lisa's completion publication.
