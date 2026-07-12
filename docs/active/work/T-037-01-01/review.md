# Review — T-037-01-01 provider-readiness-capability

## What changed

Exposed provider bootstrap-readiness as a first-class classification at the
`AgentAdapter` boundary, and had the scheduler read and record it at launch
dispatch. No seat-state behavior changed — this is the settled shape that
T-037-01-02 (Codex grace transition) and T-037-01-03 (tests) build on.

### `crates/lisa-plugin/src/adapter.rs` (commit 98a5abec)
- New `pub(crate) enum ReadinessMode { SessionStart, Grace }` — a `Copy`
  classification beside `ResetStrategy` / `SignalCapabilities`.
- New trait method `AgentAdapter::readiness_mode(&self) -> ReadinessMode`, no
  default body (each adapter answers explicitly).
- `ClaudeCodeAdapter` → `SessionStart` (pre-prompt SessionStart evidence).
- `CodexAdapter` → `Grace` (no truthful pre-prompt hook; grace paces the first
  prompt — the E-037 root cause).
- Tests: `claude_reports_session_start_readiness`, `codex_reports_grace_readiness`.

### `crates/lisa-plugin/src/lib.rs` (commit 9de83016)
- Imported `ReadinessMode`.
- New `State::seat_readiness: HashMap<u32, ReadinessMode>` — observational,
  keyed by pane, deliberately disjoint from `seat_assignments`.
- New accessor `seat_readiness_mode(pane_id) -> Option<ReadinessMode>`
  (`#[cfg_attr(not(test), allow(dead_code))]`; the non-test consumer is 02).
- Recorded `adapter.readiness_mode()` at the two fresh-`Starting` dispatch
  sites: primary `schedule_ready_tickets` dispatch (gated on `fresh_launch`) and
  the post-exit recovery relaunch. Pure map inserts — no branch/deadline/launch
  line/log change.
- Test: `scheduler_records_provider_readiness_mode_at_dispatch` — Codex dispatch
  records `Grace` and the seat is still `Starting {..}`; Claude records
  `SessionStart`.

## Acceptance criteria mapping

> A native test asserts ClaudeCodeAdapter reports SessionStart-based readiness
> and CodexAdapter reports grace-based readiness, and the scheduler reads that
> mode at launch dispatch — a settled classification with no seat-state behavior
> change yet.

- ✅ `claude_reports_session_start_readiness` / `codex_reports_grace_readiness`
  assert the per-adapter modes.
- ✅ `scheduler_records_provider_readiness_mode_at_dispatch` proves the scheduler
  reads the mode at launch dispatch (records it per pane for both providers).
- ✅ No seat-state behavior change: the same test asserts the post-dispatch seat
  is unchanged (`Starting {..}`); no `SeatAssignmentState` variant/transition was
  touched; all 286 workspace tests still pass unchanged.

## Test coverage

- Adapter unit: both modes asserted directly (mirrors `signals()` tests).
- Scheduler integration: real `schedule_ready_tickets()` through the
  `pane_name_schedule_state` harness, both providers, asserting the recorded
  classification *and* the unchanged seat — one test covers "reads at dispatch"
  and "no behavior change."
- Regression: `cargo test --workspace` → 286 passed / 0 failed.
- Build: WASM target (`wasm32-wasip1 --release`) compiles clean; `cargo clippy
  -p lisa-plugin` silent.

### Gaps / not covered (by design)
- The recovery-relaunch recording site has no dedicated test; it is a verbatim
  copy of the primary-site insert and the recovery `Starting` path is already
  exercised by existing recovery tests (behavior unchanged there). A targeted
  assertion would naturally land with 02, which makes the recovery path
  behaviourally consume the mode.
- No cross-provider mixed-loop readiness test; the two-provider coverage here is
  sufficient for a pure classification, and mixed routing is already covered for
  adapters by `mixed_route_resolves_heterogeneous_adapters_in_one_loop`.

## Open concerns / handoff notes

- **`seat_readiness` lifecycle:** entries are overwritten on every launch
  dispatch and never removed. Safe now because a pane only reaches `Starting`
  via a dispatch that just (re)classified it, so no stale entry is read before a
  fresh one replaces it. T-037-01-02, when it consumes the accessor, should
  confirm this holds for its read points (and may add removal on seat release if
  it wants strictness).
- **Disjointness is intentional:** the mode lives in its own map, not on
  `SeatAssignmentState::Starting`, so this ticket does not collide with 02/03's
  edits to the `lib.rs` `Starting` region. If 02 prefers to fold the mode into
  `Starting`, this map is the migration source and the accessor is the seam.
- **Claude path untouched:** only labeled `SessionStart`; the
  `acknowledge_process_start` → `ReadyForAssignment` evidence path is unchanged,
  per E-037's constraint.

## Risk assessment

Low. Additive-only: one enum, one trait method, one `State` field, one accessor,
two map inserts, three tests. No existing transition, deadline, launch line, or
log was modified; the full suite passing unchanged is the proof.
