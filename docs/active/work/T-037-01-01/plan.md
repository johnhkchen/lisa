# Plan — T-037-01-01 provider-readiness-capability

Two commit units. Each compiles and tests green on its own.

## Step 1 — Adapter capability (commit unit 1)

File: `crates/lisa-plugin/src/adapter.rs`

1. Add `pub(crate) enum ReadinessMode { SessionStart, Grace }` with
   `#[derive(Debug, Clone, Copy, PartialEq, Eq)]` and doc comments, placed just
   before the `AgentAdapter` trait (beside `SignalCapabilities`/`ResetStrategy`).
2. Add `fn readiness_mode(&self) -> ReadinessMode;` to the `AgentAdapter` trait
   (no default body), documented as read by the scheduler at launch dispatch.
3. `impl AgentAdapter for ClaudeCodeAdapter`: `fn readiness_mode(&self) ->
   ReadinessMode { ReadinessMode::SessionStart }`.
4. `impl AgentAdapter for CodexAdapter`: `fn readiness_mode(&self) ->
   ReadinessMode { ReadinessMode::Grace }`.
5. Tests in the module: `claude_reports_session_start_readiness`,
   `codex_reports_grace_readiness`.

Verify: `cargo test -p lisa-plugin adapter::` passes; WASM check compiles.

Commit: `lisa commit-ticket --ticket-id T-037-01-01 --message "feat(adapter):
classify provider bootstrap readiness mode" --include
crates/lisa-plugin/src/adapter.rs`

## Step 2 — Scheduler reads the mode at launch dispatch (commit unit 2)

File: `crates/lisa-plugin/src/lib.rs`

1. Extend the adapter import (line 17) to include `ReadinessMode`.
2. Add `seat_readiness: HashMap<u32, ReadinessMode>` field to `State`, beside
   `seat_assignments` with a doc comment (observational; disjoint; consumed by
   02). No constructor change (Default-derived).
3. Add `fn seat_readiness_mode(&self, pane_id: u32) -> Option<ReadinessMode>`
   accessor near `seat_assignment`, with `#[cfg_attr(not(test),
   allow(dead_code))]`.
4. Record at the two fresh-`Starting` dispatch sites:
   - primary dispatch after the `assignment_state` insert (~2765), gated on
     `fresh_launch`;
   - post-exit recovery relaunch after the `Starting` insert (~4003).
   Both: `self.seat_readiness.insert(pane_id, adapter.readiness_mode());`.
5. Test `scheduler_records_provider_readiness_mode_at_dispatch`: Codex dispatch
   → `Grace` + still `Starting`; Claude dispatch → `SessionStart`.

Verify: `cargo test -p lisa-plugin`, then `cargo test --workspace`, then
`cargo build -p lisa-plugin --target wasm32-wasip1 --release` (WASM must still
compile — the plugin is the WASM artifact).

Commit: `lisa commit-ticket --ticket-id T-037-01-01 --message "feat(scheduler):
read provider readiness mode at launch dispatch" --include
crates/lisa-plugin/src/lib.rs`

## Testing strategy

- **Unit (adapter):** direct per-adapter equality on `readiness_mode()`. Mirrors
  existing `native_signals_all_true` / `codex_signals_include_clear_handshake`.
- **Integration (scheduler):** drive real `schedule_ready_tickets()` through the
  `pane_name_schedule_state` harness and assert the recorded classification plus
  the *unchanged* `Starting` seat state — this is the "no behavior change"
  guard and the "scheduler reads at dispatch" proof in one test.
- **Regression:** full `cargo test --workspace` must stay green — no existing
  `SeatAssignmentState` assertion should shift, since no transition changed.
- **Build:** WASM target build confirms the plugin still compiles for its real
  deployment target.

## Verification criteria (maps to AC)

- [ ] `ClaudeCodeAdapter::readiness_mode() == SessionStart` (native test).
- [ ] `CodexAdapter::readiness_mode() == Grace` (native test).
- [ ] After `schedule_ready_tickets`, `seat_readiness_mode(pane)` equals the
      provider's mode — the scheduler read it at launch dispatch (native test).
- [ ] Seat state after dispatch is unchanged (`Starting {..}` for both
      fresh-launch providers) — no seat-state behavior change.
- [ ] `cargo test --workspace` green; WASM build green.

## Risks / mitigations

- *Scope creep into the state machine* → mitigated by the disjoint map; no
  `SeatAssignmentState` edit. Reviewer can grep the diff for `SeatAssignmentState`
  and find no new variant/field.
- *Half-populated map misleads 02* → record at both fresh-`Starting` sites so any
  pane reaching `Starting` is classified; overwrite-on-launch prevents stale
  reads.
- *Dead-code warning on accessor* → `#[cfg_attr(not(test), allow(dead_code))]`;
  removed by 02 when it consumes the accessor.
