# Review: T-039-02-02

## Outcome

The ticket is complete. All eight `check_*_signals` loops now consume typed
records through one private signal-ingestion boundary. The refactor preserves
the existing poll order, downstream attempt admission, provider-specific
payload meaning, legacy idle compatibility, and the characterized one-shot
deletion rules.

The T-039-02-01 characterization suite passes unchanged before and after the
refactor. Workspace tests, WASM checking, Clippy, formatting, and whitespace
checks are green.

## Commit

- Commit: `2b0af33d080d037e00365be304f07e1620c213c8`.
- Message: `T-039-02-02: add typed signal ingestion boundary`.
- Created with `lisa commit-ticket`.
- Exact include: `crates/lisa-plugin/src/lib.rs`.
- Exact include: `crates/lisa-plugin/src/signal.rs`.
- The ordinary Git index was not used.
- Both ticket-owned source paths are clean after the transaction.
- The ordinary index is empty.

## Files changed

### `crates/lisa-plugin/src/signal.rs`

- Added a private typed filesystem-ingestion module.
- Added the closed `SignalRequest` enum.
- Added the typed `SignalRecord` enum.
- Added `IdleTarget` for pane versus legacy ticket identity.
- Added the single `signal::ingest` operation.
- Moved exact pane filename parsing into the boundary.
- Added request-specific recognition and deletion behavior.
- Added seven focused unit tests.

### `crates/lisa-plugin/src/lib.rs`

- Declared the new private `signal` module.
- Imported typed request, record, and idle-target types.
- Routed all eight signal loops through `signal::ingest`.
- Removed duplicated directory scanning, filename parsing, payload reads, and
  file deletion from the consumers.
- Preserved scheduler admission and state effects in their existing methods.
- Moved two parser tests into the module that now owns the parser.

### Files not changed

- `crates/lisa-plugin/src/tests/signal_consumer_characterization.rs` is
  unchanged.
- No adapter code changed.
- No Codex acknowledgement parser changed.
- No hook or CLI templates changed.
- No core domain type changed.
- No public API changed.
- Ticket phase and status were not manually edited.

## Boundary shape

`SignalRequest` exposes exactly the eight existing scan intents:

1. heartbeats;
2. process starts;
3. shell readiness;
4. Codex acknowledgements;
5. awaiting-human;
6. idle;
7. transitions;
8. errors.

The transition request intentionally covers both stopped and cleared records in
one directory scan, matching the preexisting consumer.

`SignalRecord` exposes nine concrete record variants because transitions have
two distinct meanings. This prevents a stopped event and a cleared event from
being represented as an untyped suffix string.

## Provider and payload distinctions

### Lease JSON

Heartbeat, process-start, and shell-ready records carry parsed `AttemptLease`
values. The boundary validates only the serialized shape. It does not claim the
lease is current or admitted.

Current-attempt authority remains downstream:

- heartbeat still checks slot ticket identity;
- heartbeat still checks the slot's complete lease;
- heartbeat still checks the scheduler's current lease;
- process start still delegates to `acknowledge_process_start`;
- shell ready still delegates to `acknowledge_shell_ready`.

### Raw provider JSON

Codex acknowledgement records carry raw UTF-8 provider payload strings. The
boundary does not deserialize them as leases and does not import provider
acknowledgement logic. `acknowledge_codex_assignment` remains responsible for
the native hook event shape and exact embedded ticket/generation tag.

### Presence-only records

Awaiting, stopped, cleared, and error variants carry only pane identity. Their
bodies are not read. Arbitrary provider text therefore cannot accidentally
become a shared cross-provider payload contract.

### Idle identity

Idle carries an `IdleTarget`:

- `Pane(u32)` for current pane-scoped signals;
- `LegacyTicket(TicketId)` for the one historical filename route.

No other signal request accepts legacy ticket-named files.

## Deletion behavior

The boundary preserves two deliberately different recognition policies.

Strict pane-first signals are heartbeat, process start, shell ready, Codex
acknowledgement, awaiting, and error. A filename must parse as the exact
`pane-<u32>.<suffix>` grammar before deletion. Invalid pane names remain
untouched. Once recognized, records are deleted even if a required body cannot
be read or parsed.

Broad suffix-first signals are idle, stopped, and cleared. Their existing loops
recognized the broad suffix and deleted before parsing the pane number. The new
boundary retains that order. A malformed `pane-seven.idle` or
`pane-seven.stopped` is consumed without producing a typed record.

This distinction is explicit in separate ingestion branches and directly
covered by both focused tests and the unchanged characterization suite.

## Poll order and timing

`poll_tick` was not reorganized. It still invokes the consumers in this relative
order:

1. `check_heartbeat_signals`;
2. `check_awaiting_signals`;
3. `check_process_start_signals`;
4. `check_shell_ready_signals`;
5. `check_codex_ack_signals`;
6. `check_idle_signals`;
7. `check_transition_signals`;
8. `check_error_signals`.

Assignment delivery and artifact work remain interleaved at the same points.
Each consumer invokes the boundary on demand rather than sharing one eager poll
snapshot. That preserves the possibility that later consumers observe files
created during earlier work in the same tick.

## State effects preserved

- Admitted heartbeat refreshes pane/thread activity.
- Admitted heartbeat clears attention debounce and awaiting-human gates.
- Process start proves readiness without proving assignment ownership.
- Shell ready admits only the exact reset successor before relaunch.
- Exact Codex acknowledgement promotes ownership, refreshes activity, and logs.
- Awaiting suppresses injection without refreshing the inactivity clock.
- Pane idle retains transition-state gating and activity refresh.
- Legacy idle retains phase/artifact behavior without invented pane identity.
- Stopped and cleared refresh activity before state-machine handling.
- Error still handles assignment recovery before running-thread reclamation.
- Error still uses running threads, not stale slot bindings, as authority.

## Test coverage

### New boundary tests

Command:

`cargo test -p lisa-plugin signal::tests`

Result: 7 passed, 0 failed.

Coverage includes:

- typed lease deserialization;
- deletion of malformed recognized lease records;
- exact raw provider payload preservation;
- presence records remaining payload-free;
- retention of invalid strict pane filenames;
- pane and legacy idle target construction;
- deletion of malformed broadly recognized idle names;
- stopped and cleared variants from one scan;
- deletion of malformed broadly recognized transition names;
- unrelated transition suffix retention;
- exact filename grammar;
- non-UTF-8 filename rejection on Unix.

### Unchanged characterization suite

Command:

`cargo test -p lisa-plugin signal_consumer_characterization`

Result before implementation: 11 passed, 0 failed.

Result after implementation: 11 passed, 0 failed.

Result after source commit: 11 passed, 0 failed.

The suite remains byte-for-byte unchanged and verifies poll order, payload
admission, legacy compatibility, deletion timing, and all consumer effects.

### Repository gates

- `just check`: passed.
- WASM `wasm32-wasip1` plugin check: passed.
- `cargo test --workspace` through `just check`: passed.
- Plugin suite: 308 passed, 0 failed.
- `just lint`: passed.
- Plugin, core, and CLI Clippy ran with warnings denied.
- `just fmt-check`: passed.
- `git diff --check`: passed.

## Acceptance criteria assessment

- Eight loops consume typed signal records: satisfied.
- One new ingestion boundary: satisfied by `signal::ingest`.
- T-039-02-01 characterization passes unchanged: satisfied.
- Workspace tests green: satisfied.
- Clippy green: satisfied.
- Poll order preserved: satisfied structurally and by characterization.
- Attempt admission preserved: satisfied; it remains downstream in scheduler
  state and acknowledgement methods.
- Provider payload distinctions explicit: satisfied by lease, raw payload, and
  payload-free record variants.
- Deletion distinctions explicit: satisfied by strict and broad recognition
  branches.
- Lease distinctions explicit: satisfied by typed lease variants without moving
  current-lease authority.
- N1 accidental provider uniformity avoided: satisfied.

## Open concerns and limitations

- The boundary still performs eight directory scans per poll because that timing
  is part of the existing scheduling shape. A future performance change to one
  eager snapshot would need separate behavioral analysis and characterization.
- `SignalRequest` and `SignalRecord` are paired by the boundary implementation;
  the return type is a shared enum, so consumers defensively ignore unexpected
  variants. The closed request match makes current pairings exhaustive.
- Filesystem iteration order remains unspecified, matching the previous code.
- Filesystem errors remain intentionally silent, matching current behavior.
- Deletion failures remain best-effort. A path that cannot be removed can replay,
  which is preexisting behavior and outside this structural ticket.
- Provider acknowledgement payload validation remains intentionally outside the
  ingestion module in `codex_ack.rs` and scheduler admission code.

## Critical issues

None found. There is no known behavior regression, failing gate, public API
change, staged ticket-owned file, or uncommitted ticket-owned source path.

## Workflow state

Lisa has independently updated its ticket/provenance runtime state and admitted
phase artifacts under `docs/active/work/T-039-02-02/`. Those files were not
included in the ticket-owned source transaction. This review is written to the
attempt-private work directory as required. Lisa should now perform the final
completion publication and commit. This agent remains on T-039-02-02 and does
not start another ticket.
