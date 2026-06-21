# T-020-04 Research — timeout-exemption-surfacing

Map of the code that reclaims silent panes and the dashboard that renders threads,
so the `awaiting_human` exemption + an "awaiting" marker can be added without
disturbing the v0.2.11 liveness invariant. Descriptive only — no solutions here.

## The feature in one sentence

A pane explicitly flagged `awaiting_human` (a real `AskUserQuestion` PreToolUse
signal arrived) must not be **killed** by the two wall-clock reclaimers, and it must
be **visible** on the dashboard so an exempt pane is never silently parked.

## Upstream state this builds on (T-020-03, already merged)

`crates/lisa-plugin/src/lib.rs`:
- `awaiting_human: HashSet<u32>` field on `State` (line ~249), keyed by **pane id**
  (u32), sibling of `notified_attention`.
- `is_pane_awaiting(&self, pane_id: u32) -> bool` accessor (line ~297) — just
  `self.awaiting_human.contains(&pane_id)`.
- Set by `check_awaiting_signals()` (consumes `pane-<id>.awaiting`, inserts id,
  deletes file); cleared in `check_heartbeat_signals()` beside the
  `notified_attention.remove` (line ~820). Set insert at line ~857.
- `poll_tick` ordering (line ~1654): heartbeat → **awaiting** → artifact → idle →
  transition signals → transition timeouts → review timeouts → evaluate_health →
  **session_timeouts** → **detect_stale_threads** → rebuild_dag → … → schedule.
  So `awaiting_human` is current *before* both reclaimers run each tick.
- The T-020-03 field doc explicitly states reclaim exemption is "T-020-04, not here"
  and "Deliberately never touches the liveness clock".

## The two reclaimers (the kill paths)

Both gate on the **same hard-silence bar**: `stuck_threshold_secs * 2` of silence
since `last_activity`. Default `stuck_threshold_secs = 1200`, so the bar is 2400s.

### 1. `check_session_timeouts` (`lib.rs` ~1491–1574)
- Runs only if a global (`session_timeout_secs`, default 3600) or per-phase timeout
  is configured.
- For each Running thread, computes whether it `exceeded` its budget (global
  wall-clock since `started_at`, or per-phase since `last_phase_change`).
- If exceeded, splits on silence (line ~1539–1544):
  - `silent_for >= hard_silence` → `timed_out.push(...)` → **killed** (fail, release
    slot, remove thread, push `timeout_alerts`, log `SessionTimedOut`).
  - else → `over_budget_active.push(...)` → **warned once** (`over_budget_warned`
    set guards a single `Warning` log).
- The loop body is `for (tid, t) in &self.threads` — `t.pane_id` is in scope.

### 2. `detect_stale_threads` (`lib.rs` ~1583–1612)
- Always runs. `hard_timeout = stuck_threshold_secs * 2`.
- `stale` = Running threads whose `health(now, hard_timeout) == Stuck` (i.e. silent
  past the bar). Each is failed, slot released, removed, logged as `Error` ("stale").
- No warn path here — it's pure kill. Built as an iterator chain
  `self.threads.iter().filter(...).map(...).collect()`.

### Injection timeouts that must NOT be exempted from running
- `check_transition_timeouts` and `check_review_timeouts` already carry T-020-03
  guards (they *skip the write* when awaiting, without marking sent, so they resume
  once the flag clears). The AC is explicit: only the **kill** is exempt; these
  injectors keep running and self-skip. No change needed here.

## The dashboard

`crates/lisa-plugin/src/ui.rs`:
- `PluginState` (UI mirror, ~line 257) holds `active_threads: Vec<ActiveThread>`,
  `parked_threads`, `slots`, etc. It is a *separate* struct from the plugin `State`,
  built by `State::to_ui_state()` (`lib.rs` ~2686).
- `ActiveThread` (~line 134): `{ ticket_id, phase, started_at, slot_number }`. No
  pane id, no awaiting concept today.
- `to_ui_state` maps each Running `Thread` → `ActiveThread`, resolving `slot_number`
  from `agent_slots` by `pane_id`. This is where `is_pane_awaiting(t.pane_id)` is
  reachable (both `self` and `t.pane_id` in scope).
- `render_threads` (`ui.rs` ~687): builds a SLOT→thread lookup, prints a table with
  columns `SLOT TICKET PHASE STATUS TIME` (widths 6/12/10/14/10). The active branch
  prints STATUS `"Running"` in `GREEN`. Parked → `"Parked"` YELLOW; transitioning →
  "Winding Down"; else "Idle". The STATUS column is 14 wide.
- Color consts (`ui.rs` 15–27): RESET/BOLD/DIM/RED/GREEN/YELLOW/BLUE/MAGENTA/CYAN/…
  available in scope. CYAN is currently used for Research phase + DAG headers.
- `render_dashboard_lines` / `render_operations_view` compose the threads section
  with the attention banner and activity log. Tests call `render_threads` and
  `render_dashboard_lines` directly with hand-built `PluginState`.

## Test conventions

`mod tests` in `lib.rs` (the T-020-03 awaiting tests live ~5440–5580):
- Build `State::default()`, push `AgentSlot`/`Thread` literals, insert into
  `awaiting_human`, call the method, assert on observable state.
- Reclaimer tests can build a `Thread` with `last_activity = now - (hard_silence +
  slack)` and a configured timeout, then assert the thread is/ isn't still in
  `state.threads` after the call.
- Default config: `stuck_threshold_secs = 1200` → hard bar 2400s;
  `session_timeout_secs = 3600`. `Thread::new(id, pane)` sets all clocks to now and
  status Running.
- UI tests (`ui.rs mod tests`) build `PluginState` literals and call render fns,
  asserting on substrings of the joined output.

## Constraints / assumptions

- `awaiting_human` is keyed by **pane id**, threads carry `pane_id`, slots carry
  `pane_id` — all three join on pane id. The exemption and the marker must both key
  off the same set so they can never disagree (the ticket's "exempt-but-invisible"
  bad state).
- The exemption must be **narrow**: only panes in `awaiting_human`. A pane that never
  got the signal is unaffected (preserves v0.2.11: silence still kills).
- Warnings may still fire for an awaiting over-budget pane (visibility is good); only
  the reclamation/removal is suppressed.
- Adding a field to `ActiveThread` touches every literal that constructs it —
  including existing `ui.rs` test fixtures — which must be updated to compile.
