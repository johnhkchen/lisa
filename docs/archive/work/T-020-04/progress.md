# T-020-04 Progress — timeout-exemption-surfacing

Executed plan.md in order. All steps complete, `just check` green. No deviations from
the plan.

## Completed

| Step | What | Status |
|------|------|--------|
| 1 | `ActiveThread.awaiting` field + all 6 ui.rs fixtures + `to_ui_state` set | ✅ |
| 2 | `render_threads` awaiting-aware active row (`[AWAITING]` + "Awaiting"/CYAN) | ✅ |
| 3 | `check_session_timeouts` kill exemption (`&& !awaiting_human.contains`) | ✅ |
| 4 | `detect_stale_threads` kill exemption (`let awaiting` + filter) | ✅ |
| 5 | 5 native tests (reclaimers ×4 + to_ui_state projection) | ✅ |
| 5 | 1 ui.rs render test (`test_render_threads_marks_awaiting`) | ✅ |
| 6 | `just check` | ✅ |

## Changes landed

**`crates/lisa-plugin/src/lib.rs`**
- `check_session_timeouts` (~1540): kill branch now requires
  `silent_for >= hard_silence && !self.awaiting_human.contains(&t.pane_id)`. An
  awaiting over-budget pane falls into the existing `over_budget_active` warn branch —
  warning still logs, kill suppressed.
- `detect_stale_threads` (~1590): `let awaiting = &self.awaiting_human;` then
  `.filter(|(_, t)| !awaiting.contains(&t.pane_id))` in the `stale` chain.
- `to_ui_state` (~2712): `awaiting: self.is_pane_awaiting(t.pane_id)` on each
  `ui::ActiveThread`.
- 5 new tests in `mod tests`.

**`crates/lisa-plugin/src/ui.rs`**
- `ActiveThread` gained `pub awaiting: bool` (doc-commented).
- `render_threads` active branch renders `T-xxx [AWAITING]` + status "Awaiting" in
  CYAN when `awaiting`, else unchanged "Running"/GREEN.
- 6 existing `ActiveThread` fixtures updated with `awaiting: false`.
- 1 new test `test_render_threads_marks_awaiting`.

## Deviations

None. The borrow strategy from design A4 held: the `for`-loop case
(`check_session_timeouts`) accepted the inline `self.awaiting_human.contains` as a
disjoint field borrow; the iterator-chain case (`detect_stale_threads`) used the local
`let awaiting` binding. Both compiled first try.

## Verification

- `cargo test -p lisa-plugin`: 177 passed (was 171; +6).
- `just check`: WASM `cargo check -p lisa-plugin --target wasm32-wasip1` Finished;
  workspace suite — lisa-cli 172, lisa-core 106, lisa-plugin 177 — all pass.
- No commits made in this session (working tree changes staged for the human's
  normal commit flow, consistent with prior S-020 tickets which left committing to
  the operator).
