# T-020-03 Research — awaiting-human suppression

Descriptive map of the codebase regions this ticket touches. No solutions here.

## The problem this ticket addresses

When an agent calls `AskUserQuestion`, the T-020-02 PreToolUse hook fires and
writes `.lisa/signals/pane-<id>.awaiting` (plus pings `on-notify attention`). At
that moment the agent is **blocked waiting for a human** — it is not idle, not
stopped, and emits no heartbeat. The plugin, however, has several code paths that
inject text into a pane (`/clear`, a next-ticket prompt, a finish-up prompt). If
any of those fire while the agent is showing its question UI, lisa clobbers the
question — types over it, advances a state machine, or worse `/clear`s the whole
session. The `.awaiting` signal exists but nothing consumes it yet. This ticket
makes the plugin read it and gate all injection for the affected pane.

## Signal-file architecture (confirmed)

Signal files are **written by shell hooks, read by the WASM plugin** — never the
reverse (obs 23043). All live in `self.signal_dir` = `.lisa/signals/` under the
host mount. Naming is uniform: `pane-<LISA_PANE_ID>.<kind>`.

Existing kinds and their plugin consumers (all in `crates/lisa-plugin/src/lib.rs`):

| Kind | Writer (templates.rs) | Plugin reader |
|---|---|---|
| `.heartbeat` | on every tool call (PostToolUse) | `check_heartbeat_signals()` `lib.rs:760` |
| `.idle` | session goes idle | `check_idle_signals()` `lib.rs:798` |
| `.stopped` | session stop | `check_transition_signals()` → `handle_stopped_signal` `lib.rs:1057` |
| `.cleared` | after `/clear` | `check_transition_signals()` → `handle_cleared_signal` `lib.rs:1169` |
| `.awaiting` | **new, T-020-02** PreToolUse[AskUserQuestion] | **none yet — this ticket** |

The `.awaiting` writer is `NOTIFY_QUESTION_COMMAND` (templates.rs:121). It writes
`date > ".lisa/signals/pane-$LISA_PANE_ID.awaiting"` **unconditionally** (only the
on-notify ping is `test -x` gated). So the file is reliably present whenever the
agent asks a question, regardless of whether the user installed an on-notify hook.

## The `notified_attention` pattern (the template to mirror)

`notified_attention: HashSet<u32>` (`lib.rs:241`) is the closest existing analog —
a per-pane debounce flag keyed by pane id:

- **Field:** plain `HashSet<u32>`, `#[derive(Default)]` covers it (no init in `load()`).
- **Set:** inserted at `lib.rs:983` when an idle-without-artifact alert fires.
- **Clear:** `self.notified_attention.remove(&pane_id)` at `lib.rs:783`, **inside
  `check_heartbeat_signals`** — a heartbeat proves the agent resumed real work.
- **Test:** `test_attention_debounce_add_skip_and_clear` (`lib.rs:5304`) exercises
  insert/skip/remove directly on the field — no zellij calls.

This is exactly the lifecycle `awaiting_human` needs: set when blocked, cleared on
the next heartbeat (any tool call after the question resolves). Crucially, the
clear rides the **heartbeat**, not AskUserQuestion's own PostToolUse — so it is
robust to the spike's Q4 uncertainty about whether AskUserQuestion emits a
PostToolUse heartbeat of its own.

## The injection chokepoint and its callers

All pane input flows through one method:

```
fn send_line_to_pane(&mut self, text: &str, pane_id: PaneId)   // lib.rs:268
    write_chars_to_pane_id(text, pane_id);   // zellij host fn
    self.pending_enters.push_back(pane_id);  // deferred Enter
    set_timeout(ENTER_DELAY_SECS); pending_timer_count += 1;
```

`PaneId` is the zellij enum; every caller passes `PaneId::Terminal(pane_id)` with a
`u32`. The five callers (verified line numbers in current working tree):

1. `schedule_ready_tickets` `lib.rs:550` (`/clear` on reuse) and `:559` (launch cmd).
   Targets an **idle slot** (`find_idle_slot`), i.e. one with no active agent — so an
   awaiting agent is not normally here. Defensive only.
2. `handle_stopped_signal` `lib.rs:1071` — `/clear` after a `.stopped`, mid-transition.
3. `handle_cleared_signal` `lib.rs:1186` — next-ticket prompt after a `.cleared`.
4. `check_transition_timeouts` `lib.rs:1245` (`/clear`) and `:1262` (prompt) — fallback
   force-advance when expected signals never arrive.
5. `check_review_timeouts` `lib.rs:1306` — finish-up prompt to a parked Review thread.
   **Most acute clobber risk**: a Review agent legitimately asks a question, and the
   review-timeout fallback types a finish-up prompt over it.

## Poll ordering (`poll_tick`, lib.rs:1548)

```
check_heartbeat_signals  (1551) → clears flags on real activity
check_artifact_advances  (1554)
check_idle_signals       (1557)
check_transition_signals (1560) → stopped/cleared handlers
check_transition_timeouts(1563)
check_review_timeouts    (1566)
... health, session timeouts, stale detection, rebuild_dag ...
```

A new `check_awaiting_signals()` must run **before** `check_idle_signals` (ticket
AC) so the flag is set before any consumer in the same tick can act on the pane.
Placing it right after `check_heartbeat_signals` is natural: heartbeats clear,
then awaiting sets — and since the writer hook emits `.awaiting` and the agent
emits no heartbeat while blocked, the two never race destructively.

## Liveness model — the constraint that must not break (obs 13214)

v0.2.11 replaced phase-based stall detection with **heartbeat liveness**
([[liveness-heartbeat-design]]): a pane is "alive" only while heartbeats reset its
`last_activity_at` / thread `last_activity`. Timeouts, stale-reclaim, and pane reuse
all gate on **silence on that clock**. A blocked AskUserQuestion agent emits no
heartbeat, so its silence clock keeps ticking — correct, because a genuinely dead
pane and a question-blocked pane look identical on the activity clock until the
human answers. Therefore `awaiting_human` must **never** touch `last_activity_at`
or `bump_pane_activity`; it only gates *writes*. Reclamation/timeout exemption is
explicitly out of scope (deferred to T-020-04).

## Native test constraints

`send_line_to_pane` calls `write_chars_to_pane_id`, a zellij host function that
cannot be invoked in native tests (see the comment at `lib.rs:3666`). So tests
**cannot** call any path that reaches an unguarded `send_line_to_pane`. The proven
pattern (heartbeat/idle/attention tests) is pure `std::fs` + direct state
assertions against a `tempfile::tempdir()` `signal_dir`. The guards must therefore
short-circuit *before* `send_line_to_pane`, and tests assert the **absence of
state-machine advance** (transition_state unchanged, `finish_up_sent` empty, slot
ticket_id untouched) plus signal-file insert/delete — never the write itself.

## Open questions / assumptions

- Assumes `.awaiting` is never stale-left: if an agent asks a question, then the
  session dies without ever answering, the file is already consumed (deleted on
  read) and the flag clears only on the next heartbeat — which never comes, so the
  pane stays flagged until stale-reclaim runs on the silence clock. That is the
  intended belt: gating writes on a dead pane is harmless; T-020-04 handles reclaim.
- `is_pane_awaiting` keys on the **terminal** pane id; all callers use
  `PaneId::Terminal`, so extracting the inner `u32` in the in-method guard is total
  for the cases that occur.
