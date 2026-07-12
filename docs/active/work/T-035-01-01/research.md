# T-035-01-01 Research — process-start signal producer

## Ticket boundary

- The ticket asks for one provider-neutral positive process-start signal.
- The producers are the native Claude and native Codex TUI `SessionStart` paths.
- The headless `lisa agent-exec` wrapper is explicitly outside this loop path.
- This ticket produces the signal; T-035-01-03 will consume it to gate `Owned`.
- T-035-01-04 owns bounded recovery when a start signal never arrives.
- T-035-01-02 separately owns atomic, bounded first-launch transport.

## Dispatch and lease identity

- Fresh assignment dispatch is in `crates/lisa-plugin/src/lib.rs`.
- Dispatch mints an `AttemptLease { ticket_id, attempt_id }` per ticket attempt.
- The lease is installed in `current_leases` and on the selected `AgentSlot`.
- Before a fresh launch, `write_pane_lease_marker` atomically writes
  `.lisa/signals/pane-<pane>.lease`.
- The marker is compact JSON serialized by `serde_json`.
- The marker is deliberately published before launch input is delivered.
- Recycled paths defer marker publication until the predecessor process exits or clears.
- `AttemptLease` equality is the existing authority contract throughout the scheduler.
- Stale signals must not be able to acquire a successor attempt's identity.

## Native launch commands

- `SpawnContext` in `crates/lisa-plugin/src/adapter.rs` carries the ticket directory,
  ticket ID, pane ID, private artifact directory, and optional Codex ack generation.
- It does not currently carry the attempt ID as a first-class launch value.
- Claude launch construction delegates from `ClaudeCodeAdapter::launch_command` to
  `build_claude_command` in `crates/lisa-plugin/src/lib.rs`.
- The Claude command exports `LISA_PANE_ID` and `LISA_TICKET_ID`.
- The Codex command is built by `CodexAdapter::interactive_line`.
- It exports `LISA_BIN`, `LISA_AGENT_CLIENT`, `LISA_PANE_ID`, and `LISA_TICKET_ID`.
- Both commands start a resident native TUI process in the selected Zellij pane.
- Both inherit their inline environment for lifecycle hook execution.

## Generated lifecycle hooks

- Hook templates live in `crates/lisa-cli/src/templates.rs`.
- `lisa init` materializes scripts under `.lisa/hooks/`.
- Claude configuration is generated in `.claude/settings.local.json`.
- Codex configuration is generated in `.codex/hooks.json`.
- Both native configurations currently bind `SessionStart` only with matcher `clear`.
- That binding invokes `.lisa/hooks/on-clear.sh` and writes `.cleared`.
- Neither configuration binds the `startup` SessionStart source.
- Therefore no existing lifecycle artifact proves that a freshly launched provider began.

## Existing provider-neutral signal pattern

- `ON_HEARTBEAT_HOOK` is shared by both native clients.
- It reads `.lisa/signals/pane-<pane>.lease`.
- It copies the lease to a temporary file and atomically renames it to `.heartbeat`.
- `check_heartbeat_signals` parses the result as `AttemptLease`.
- The consumer admits it only when pane, ticket, slot lease, and current lease all match.
- Invalid, missing, stale, and malformed heartbeats are consumed without activity credit.
- This establishes the appropriate payload and atomic-publication pattern for start.

## Start-specific safety constraint

- A process-start signal must prove which launched attempt produced it.
- Copying only the current pane marker is insufficient for this proof.
- A delayed predecessor process could run after the scheduler replaces the pane marker.
- If that predecessor blindly copied the marker, it would emit the successor's lease.
- The producer therefore needs process-bound attempt identity in addition to pane identity.
- `LISA_TICKET_ID` already binds the process to a ticket.
- A launch-scoped attempt ID is the missing value.
- Comparing process identity to the marker before copying prevents stale or mismatched starts.

## Initialization and upgrade behavior

- `plan_init` owns the list of Lisa-managed hook scripts.
- Each script is planned through `plan_owned_template`.
- Known previous byte generations are accepted through `LEGACY_*` constants.
- A new script can be added without overwriting user-owned divergent content.
- Both hook JSON merge functions use `ensure_hook`.
- Matchered hooks are deduplicated by matcher value.
- Adding a `SessionStart[startup]` group coexists with existing `SessionStart[clear]`.
- Re-merging must remain idempotent and preserve user hook commands.
- Init validation enumerates required hook files and expected JSON fragments.
- Tests already cover fresh init, upgrades, malformed JSON safety, and round trips.

## Test surfaces

- Template unit tests inspect hook content and parse generated hook JSON.
- Adapter and `build_claude_command` tests assert exact launch strings and env prefixes.
- Init tests assert script creation and generated native hook presence.
- Rust tests can execute a generated POSIX shell script in a temporary directory.
- A fixture can provide a compact lease marker and launch-scoped environment.
- Matching ticket/attempt input should create `.started` with exact lease bytes.
- A stale attempt ID should create no `.started` file.
- A mismatched ticket should create no `.started` file.
- Missing identity or missing marker should create no `.started` file.
- Not invoking the hook models the case where no provider process starts and necessarily
  produces no signal.

## Relevant files

- `crates/lisa-cli/src/templates.rs`: hook script, generated JSON, merge logic, tests.
- `crates/lisa-cli/src/init.rs`: hook materialization/validation and init tests.
- `crates/lisa-plugin/src/adapter.rs`: native provider launch commands and tests.
- `crates/lisa-plugin/src/lib.rs`: Claude command helper, dispatch context, lease marker.
- `.lisa/hooks/on-heartbeat.sh`: installed example of the lease-copy contract.
- `crates/lisa-core/src/types.rs`: serialized `AttemptLease` schema.

## Constraints and assumptions

- Signal content must remain provider-neutral and parse as `AttemptLease`.
- Signal publication must be atomic within `.lisa/signals`.
- The hook must be POSIX `sh` and require no `jq` or provider-specific binary.
- Ticket IDs are scheduler-controlled identifiers; attempt IDs are positive integers.
- Hook failure must not prevent the provider session from starting.
- Existing `clear`, heartbeat, stop, idle, and Codex acknowledgment behavior must remain.
- No scheduler ownership state should change in this ticket.
- No `agent-exec` behavior should be added or treated as native-loop evidence.
