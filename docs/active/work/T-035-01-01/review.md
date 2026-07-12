# T-035-01-01 Review — process-start signal producer

## Outcome

Fresh native Claude and Codex processes now share one positive, provider-neutral startup
contract. Each `SessionStart[startup]` invokes the same managed hook, and the hook publishes
`pane-<pane>.started` only when the launched process's ticket/attempt identity exactly
matches the scheduler-owned pane lease marker.

The signal payload is the existing compact `AttemptLease` JSON. It is written through a
same-directory temporary file and atomic rename, matching the established heartbeat signal
pattern. No process start means the hook never runs and no signal exists.

## Files changed

### `crates/lisa-cli/src/templates.rs`

- Added `ON_START_HOOK` and its owned-template legacy inventory.
- Added `SessionStart[startup]` to generated Claude settings.
- Added the same startup binding to generated native Codex hooks.
- Added the startup group to both merge paths without disturbing `clear`.
- Added template, JSON, idempotency, and executable fixture tests.

### `crates/lisa-cli/src/init.rs`

- Added `on-start.sh` to init planning and executable materialization.
- Added the script and startup binding to validation.
- Updated init fixture inventories and managed-file counts.
- Extended fresh-init checks to cover the new script.

### `crates/lisa-plugin/src/adapter.rs`

- Added immutable `attempt_id` to `SpawnContext`.
- Passed attempt identity into Claude launch construction.
- Exported `LISA_ATTEMPT_ID` in native Codex launches.
- Updated provider launch shape tests.

### `crates/lisa-plugin/src/lib.rs`

- Added the attempt ID parameter to `build_claude_command`.
- Exported `LISA_ATTEMPT_ID` in native Claude launches.
- Populated fresh dispatch from the exact minted lease.
- Threaded current pane attempt identity through auxiliary spawn contexts.
- Updated command and environment tests.

## Commits

- `e4f812d feat: scaffold native process-start signal`
- `7379efd feat: bind native starts to attempt identity`

Both commits used `lisa commit-ticket` with exact repository-relative include paths. No
ordinary Git index command was used for ticket source.

## Acceptance coverage

### Same signal for Claude and Codex

Both generated lifecycle configurations bind `SessionStart` matcher `startup` to the exact
same guarded `.lisa/hooks/on-start.sh` command. Existing `SessionStart[clear]` continues to
invoke `on-clear.sh` separately.

### Pane/ticket/attempt scope

Both provider launch commands carry pane ID, ticket ID, and immutable attempt ID. The hook
constructs the expected compact lease identity and requires exact equality with the pane
marker before publishing.

### Starts before heartbeat

The producer is bound to provider `SessionStart[startup]`; heartbeat remains bound to
`PostToolUse`. Thus the positive startup lifecycle event is available before the first tool
progress heartbeat.

### Rejection fixtures

The executable shell fixture proves:

- matching pane/ticket/attempt writes exact lease bytes;
- stale attempt ID writes no `.started` signal;
- mismatched ticket writes no signal;
- invalid attempt identity writes no signal;
- missing lease marker writes no signal;
- no hook invocation (no provider process start) writes no signal;
- rejected executions leave no temporary start artifact.

## Verification

- `cargo fmt --all -- --check` — passed.
- `cargo test -p lisa-cli templates` — 32 focused tests passed.
- `cargo test -p lisa-plugin` — 276 tests passed.
- `cargo test --workspace` — passed in full.
- The workspace run includes 273 CLI tests, the atomic provider-contract integration test,
  155 core tests, 276 plugin tests, and doc tests.
- Existing Codex ack matching/stale-generation fixtures passed.
- Existing stale-attempt heartbeat, split-brain fencing, and bounded recovery tests passed.

## Open concerns and boundaries

- This ticket intentionally only produces `.started`; T-035-01-03 owns consuming it and
  withholding `Owned` until it is admitted.
- T-035-01-04 owns bounded timeout/recovery when the signal is absent.
- Reused in-process prompts do not emit a new process startup and retain their existing
  clear/ack contracts.
- Auxiliary historical unleased test paths map absent attempt identity to `0`; production
  fresh dispatch always uses the positive minted lease directly, and the hook rejects any
  marker that does not exactly match.
- The POSIX hook compares Lisa's current compact JSON serialization byte-for-byte. A future
  lease schema/serialization change must update the hook and fixture together.
- Real Zellij PTY validation remains assigned to the later story ticket; this ticket's
  producer behavior is deterministic fixture coverage.

## Reviewer focus

The main safety property to inspect is that process-bound `LISA_ATTEMPT_ID`, rather than the
mutable pane marker alone, determines whether startup can publish. This prevents a delayed
predecessor process from copying and masquerading under a successor attempt's lease.

No critical unresolved issue remains within this ticket's producer scope.
