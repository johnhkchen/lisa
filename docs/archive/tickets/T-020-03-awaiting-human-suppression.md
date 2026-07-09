---
id: T-020-03
story: S-020
title: awaiting-human-suppression
type: feature
status: open
priority: high
phase: done
depends_on: [T-020-02]
---

## Context

Make the plugin consume the `pane-<id>.awaiting` signal (written by T-020-02) and
suppress auto-injection into a question-blocked pane, so lisa never types `/clear`
or a next prompt over an agent's `AskUserQuestion`. This is the correctness fix
S-019 deferred and the heart of S-020. Touches `crates/lisa-plugin/src/lib.rs`
(and a helper); per the spike, it must **not** fake activity or destabilize the
v0.2.11 heartbeat liveness model (see [[liveness-heartbeat-design]]).

Key anchors (verify before editing; from spike design Q5):
- New state field beside `notified_attention` — `lib.rs:241` (`#[derive(Default)]`,
  no init needed).
- Model `check_awaiting_signals()` on `check_heartbeat_signals()` — `lib.rs:760`.
- Clear point (where `notified_attention` is already cleared) — `lib.rs:783`.
- `poll_tick` ordering — call before `check_idle_signals` (`lib.rs:1551-1557`).
- Injection chokepoint — `send_line_to_pane`; five callers in the Q5 table below.

## Acceptance Criteria

- Add `awaiting_human: HashSet<u32>` to `State` (`lib.rs:241`).
- New `check_awaiting_signals()` (modeled on `check_heartbeat_signals`, `lib.rs:760`):
  reads `pane-<id>.awaiting` from the signal dir, inserts the pane id into
  `awaiting_human`, and deletes the file. Called in `poll_tick` **before**
  `check_idle_signals` (`lib.rs:1551-1557`) so the flag is set before idle handling runs.
- **Clear:** in `check_heartbeat_signals` where `notified_attention` is cleared
  (`lib.rs:783`), also `self.awaiting_human.remove(&pane_id)` — a real tool call means the
  agent resumed and is no longer blocked.
- Add `fn is_pane_awaiting(&self, pane_id: u32) -> bool`.
- **Guard injection** — drop the write inside `send_line_to_pane` when the target pane is
  awaiting (log it), **and** early-return / skip at the five callers so their state machines
  don't advance mid-question (design Q5 table):

  | Caller (`lib.rs`) | Inject | Guard action when awaiting |
  |---|---|---|
  | `schedule_ready_tickets:550/559` | `/clear` / launch | skip slot this tick (stays assigned, retried next poll) |
  | `handle_stopped_signal:1071` | `/clear` | return early |
  | `handle_cleared_signal:1186` | prompt | return early |
  | `check_transition_timeouts:1245/1262` | `/clear` / prompt | skip that pane in the fallback loop |
  | `check_review_timeouts:1306` | finish-up prompt | skip candidate (most acute clobber risk) |

  Belt-and-suspenders: the in-method guard makes a missed caller fail safe (no clobber);
  the per-caller returns keep slot/transition state coherent. Callers #1/#2 only target idle
  slots (no awaiting agent) — guarding them is defensive, not load-bearing.
- **Liveness safety (must hold):** the flag never touches `last_activity_at` / never fakes
  activity, so a genuinely dead pane still trips stale detection on the normal silence clock.
  This ticket only *gates writes*; it does **not** exempt reclamation (that's T-020-04).
- Tests (native): per-guarded-caller — set `awaiting_human` for a pane, assert no
  `send_line_to_pane` write / no state-machine advance; assert the flag clears when a
  heartbeat signal for that pane is processed; assert `check_awaiting_signals` inserts +
  deletes the signal file.
- `just check` passes (WASM check + tests).

## Implementation notes

- Reuse the exact `HashSet<u32>` + heartbeat-clear pattern already used by
  `notified_attention` (`lib.rs:241,783`) — the awaiting flag rides the same path, which is
  what makes it robust to the Q4 uncertainty (it does not depend on AskUserQuestion's own
  PostToolUse; any subsequent tool call clears it).
