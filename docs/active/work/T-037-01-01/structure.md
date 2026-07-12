# Structure — T-037-01-01 provider-readiness-capability

Two files change. Both are ticket-owned. No file is created or deleted.

## 1. `crates/lisa-plugin/src/adapter.rs` (MODIFY)

### New public type — `ReadinessMode`

Place beside `ResetStrategy` / `SignalCapabilities` (the sibling classification
descriptors), before the `AgentAdapter` trait.

```rust
/// How a provider proves it is ready to receive its first prompt after a fresh
/// launch. Read by the scheduler at launch dispatch to pick the bootstrap-
/// readiness path (E-037).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ReadinessMode {
    /// A truthful pre-prompt process-start signal proves readiness
    /// (Claude `SessionStart`): Starting → ReadyForAssignment on positive
    /// evidence.
    SessionStart,
    /// No truthful pre-prompt readiness hook exists; a bounded named startup
    /// grace paces the first prompt (Codex). Elapsed time paces, never proves
    /// readiness or ownership.
    Grace,
}
```

### New trait method on `AgentAdapter`

Add beside `reset_strategy` / `signals`:

```rust
/// Which bootstrap-readiness path this provider uses after a fresh launch
/// (see [`ReadinessMode`]). The scheduler reads this at launch dispatch.
fn readiness_mode(&self) -> ReadinessMode;
```

No default body — each of the two adapters answers explicitly, matching how
`reset_strategy`/`signals` are implemented per adapter. (A default would hide a
new provider silently defaulting to the wrong path.)

### Impl answers

- `impl AgentAdapter for ClaudeCodeAdapter` → `fn readiness_mode(&self) ->
  ReadinessMode { ReadinessMode::SessionStart }`.
- `impl AgentAdapter for CodexAdapter` → `fn readiness_mode(&self) ->
  ReadinessMode { ReadinessMode::Grace }`.

### Tests (append to `adapter.rs` `#[cfg(test)] mod tests`)

- `claude_reports_session_start_readiness`:
  `assert_eq!(ClaudeCodeAdapter::default().readiness_mode(),
  ReadinessMode::SessionStart);`
- `codex_reports_grace_readiness`:
  `assert_eq!(CodexAdapter::new(Some("/abs/lisa"), None).readiness_mode(),
  ReadinessMode::Grace);`

## 2. `crates/lisa-plugin/src/lib.rs` (MODIFY)

### Import

Extend the existing `adapter` use to bring in `ReadinessMode` (the module
already imports `resolve_adapter_or_native`, `SpawnContext`, `ResetStrategy`,
etc.). Add `ReadinessMode` to that import list.

### New `State` field — observational, machine-disjoint

Beside `seat_assignments` (line 433):

```rust
/// Provider bootstrap-readiness classification per pane, recorded at launch
/// dispatch (T-037-01-01). Observational only in this ticket; T-037-01-02
/// keys the Codex grace transition on it. Disjoint from `seat_assignments`.
seat_readiness: HashMap<u32, ReadinessMode>,
```

`State` derives `Default`, and `HashMap` is `Default`, so no constructor edit.

### New accessor (near `seat_assignment`, line 1380)

```rust
#[cfg_attr(not(test), allow(dead_code))]
fn seat_readiness_mode(&self, pane_id: u32) -> Option<ReadinessMode> {
    self.seat_readiness.get(&pane_id).copied()
}
```

`allow(dead_code)` off-test because the only non-test consumer arrives in
T-037-01-02; the scheduler still *writes* it in this ticket (the AC's "reads at
dispatch").

### Recording sites — where the scheduler reads the mode

Record immediately after the fresh-`Starting` seat insertions, using the
`adapter` already in scope:

1. Primary dispatch (`schedule_ready_tickets`, ~2749–2765): after
   `self.seat_assignments.insert(pane_id, assignment_state);` (2765), when
   `fresh_launch` is true, record
   `self.seat_readiness.insert(pane_id, adapter.readiness_mode());`.
   Gating on `fresh_launch` keeps the record aligned with the `Starting`
   insertion (reused-owned Claude panes never enter `Starting`).

2. Post-exit recovery relaunch (~3985–4004): after the
   `SeatAssignmentState::Starting { generation: recovery_generation, .. }`
   insertion, record `self.seat_readiness.insert(pane_id,
   adapter.readiness_mode());` using that block's `adapter`. Keeps every pane
   that reaches `Starting` classified.

Both are pure map inserts — no branch, deadline, launch line, or log changes.

### Test (append to `lib.rs` `#[cfg(test)] mod tests`)

`scheduler_records_provider_readiness_mode_at_dispatch`:
- Codex: `pane_name_schedule_state("codex", AgentClient::Codex, None)` →
  `schedule_ready_tickets()` → assert `seat_readiness_mode(10) ==
  Some(ReadinessMode::Grace)` AND `seat_assignment(10)` is still
  `Starting {..}` (proves no behavior change).
- Claude: `pane_name_schedule_state("claude", AgentClient::Claude, None)` →
  `schedule_ready_tickets()` → assert `seat_readiness_mode(10) ==
  Some(ReadinessMode::SessionStart)`.

## Ordering of changes

1. adapter.rs: enum + trait method + two impls + two adapter tests (self-
   contained; compiles and tests green alone).
2. lib.rs: import + field + accessor + two recording inserts + scheduler test.

Commit as two ticket-owned units (adapter capability, then scheduler read) via
`lisa commit-ticket`.

## Non-goals / untouched

- `SeatAssignmentState` and every transition/deadline: unchanged.
- Launch commands, `SpawnContext`, assignment text, logs: unchanged.
- Codex grace transition, delayed-send/prompt-miss tests: T-037-01-02 / -03.
