# T-020-04 Plan — timeout-exemption-surfacing

Ordered, independently-verifiable steps. Each compiles (or is a test-only addition).
Commit after each logical group. Verification: `cargo test -p lisa-plugin` per step,
`just check` at the end.

## Step 1 — `ActiveThread.awaiting` field + update all literals (compile gate)
- `ui.rs` S5: add `pub awaiting: bool` to `ActiveThread`.
- `ui.rs` S7: add `awaiting: false` to every existing `ActiveThread { … }` test
  fixture so the crate compiles.
- `lib.rs` S3: add `awaiting: self.is_pane_awaiting(t.pane_id)` to the `to_ui_state`
  `active_threads` literal.
- **Verify:** `cargo build -p lisa-plugin` clean (no missing-field errors).
- Commit: "Add awaiting flag to ui::ActiveThread, wired from awaiting_human".

## Step 2 — render the marker
- `ui.rs` S6: awaiting-aware active row (`[AWAITING]` ticket token + "Awaiting"/CYAN
  status when `active.awaiting`, else unchanged).
- **Verify:** existing `render_threads` tests still pass (non-awaiting rows
  unchanged — they assert "Running").
- Commit: "Render [AWAITING] marker for awaiting-human threads".

## Step 3 — exempt the kill in `check_session_timeouts`
- `lib.rs` S1: gate the `timed_out` push on
  `silent_for >= hard_silence && !self.awaiting_human.contains(&t.pane_id)`.
- **Verify:** `cargo build -p lisa-plugin`; existing session-timeout tests pass
  (none flag a pane, so behavior is unchanged for them).
- Commit: "Exempt awaiting panes from session-timeout reclamation".

## Step 4 — exempt the kill in `detect_stale_threads`
- `lib.rs` S2: `let awaiting = &self.awaiting_human;` + `.filter(|(_, t)|
  !awaiting.contains(&t.pane_id))` in the `stale` chain.
- **Verify:** build clean; existing stale tests pass.
- Commit: "Exempt awaiting panes from stale-thread reclamation".

## Step 5 — tests
Add the native tests (structure S4 + S8):

Reclaimer exemption (lib.rs):
1. `test_session_timeout_skips_kill_when_awaiting`
2. `test_session_timeout_kills_after_flag_clears`
3. `test_detect_stale_skips_when_awaiting`
4. `test_detect_stale_kills_after_flag_clears`
5. `test_to_ui_state_marks_awaiting_thread`

UI marker (ui.rs):
6. `test_render_threads_marks_awaiting`

- **Verify:** `cargo test -p lisa-plugin` — all green; the four reclaimer tests prove
  exempt-when-flagged and kill-when-cleared on **both** paths.
- Commit: "Tests: awaiting-human reclaim exemption and dashboard marker".

## Step 6 — full check
- `just check` (WASM check + full workspace suite + clippy/fmt as configured).
- **Verify:** all green.
- Commit only if `just` made incidental changes (it shouldn't).

## Testing strategy

- **Unit (native), reclaimers:** build a Running `Thread` with `last_activity`
  backdated past `stuck_threshold_secs*2`. For session-timeout, also backdate
  `started_at` past `session_timeout_secs` (default 3600) so a budget is exceeded.
  Assert presence/absence in `state.threads` after the call. The paired
  flag-set/flag-clear tests are the core acceptance evidence: same fixture, only the
  `awaiting_human` membership differs, opposite outcomes.
- **Unit (native), UI mirror:** `to_ui_state` projection test confirms the marker is
  driven off the same `awaiting_human` set (anti-divergence AC).
- **Unit (native), render:** substring assertion on `render_threads` output for
  `[AWAITING]`.
- **Not exercised natively:** none of the new code calls a zellij host fn, so all
  paths are directly testable (unlike T-020-03's in-method guard). No integration
  test needed.

## Risk / rollback

- Smallest-possible blast radius: 2 one-line-ish guards + 1 field + 1 render branch.
- If a reclaimer guard regressed, a flagged pane would be killed mid-question — caught
  by tests 1 & 3. If the field/marker diverged from the set, test 5 catches it.
- Rollback is per-commit; the field addition (Step 1) is the only step other commits
  depend on.
