# T-020-01 Structure — blueprint for the AskUserQuestion attention feature

This spike merges no production code. This document is the **file-level blueprint** the
implementation tickets (T-020-02..04) would follow, so the design's decisions are concrete and
reviewable. Shapes and signatures only — not code. Anchors are current-working-tree.

## Change map (across the implementation tickets)

| File | Change | Ticket | What |
|------|--------|--------|------|
| `crates/lisa-cli/src/templates.rs` | modify | T-020-02 | new hook command const; 6th binding in `settings_local_json` + `merge_hooks` |
| `crates/lisa-cli/src/init.rs` | modify | T-020-02 | validate the new binding; no new scaffolded *file* (the command is inline, reuses `on-notify`) |
| `crates/lisa-cli/data/hooks-guide.md` | modify | T-020-02 | document the PreToolUse binding + `LISA_REASON=question` |
| `crates/lisa-plugin/src/lib.rs` | modify | T-020-03/04 | `awaiting_human` field; `check_awaiting_signals`; injection guards; reclamation exemption |
| `crates/lisa-plugin/src/ui.rs` | modify | T-020-04 | "awaiting human" pane marker |
| (none) | — | — | **no files created or deleted** |

No new modules, no new crates. The whole feature is one new hook command string, one new
`HashSet<u32>` field, one new signal-scan method, a guard helper, and edits to existing call
sites — deliberately small.

## 1. `templates.rs` (T-020-02)

**New const** next to `NOTIFY_ATTENTION_COMMAND` (`templates.rs:110`). It mirrors that
command's structure exactly (POSIX `sh`, `test -x` guard, `in=$(cat)`, single `case`/`sed`):

```
const NOTIFY_QUESTION_COMMAND: &str =
  // 1) write awaiting signal keyed by $LISA_PANE_ID (mkdir -p .lisa/signals first)
  // 2) if on-notify is executable, extract first-question text via sed and
  //    invoke: LISA_EVENT=attention LISA_REASON=question on-notify attention "<q>"
```

Notes on its body:
- It must write the await signal **unconditionally** (the plugin suppression must work even if
  the user never enabled `on-notify`); only the notify dispatch is `test -x`-gated.
- `$LISA_PANE_ID` is already exported into the agent's env (`lib.rs:55`), so the signal
  filename `pane-$LISA_PANE_ID.awaiting` matches the plugin's scan convention.
- Extraction is best-effort (design Q3); a failed `sed` degrades to a generic detail.

**`settings_local_json()` (`templates.rs:116-173`):** add a `"PreToolUse"` entry. PreToolUse
already exists for the heartbeat (matcher-less). The new entry **has a matcher**
`"AskUserQuestion"`, so it is a distinct array element from the heartbeat entry:

```
"PreToolUse": [
  { "hooks": [ heartbeat ... ] },                      // existing, matcher-less
  { "matcher": "AskUserQuestion", "hooks": [ NOTIFY_QUESTION_COMMAND ] }   // new
]
```

**`merge_hooks()` (`templates.rs:255-299`):** one more `ensure_hook(hooks_obj, "PreToolUse",
Some("AskUserQuestion"), NOTIFY_QUESTION_COMMAND)`. `ensure_hook` dedups a *matchered* entry by
its matcher value (`templates.rs:200-203`), so it coexists with the matcher-less heartbeat and
is idempotent across re-runs.

**Tests (in `templates.rs` `mod tests`):** extend the existing patterns — a
`test_settings_local_json` assertion that PreToolUse now has 2 entries with the
`AskUserQuestion` matcher present; a `merge_hooks` idempotency test (count stays 1); and a
`sed`-extraction unit test fed the captured `pretooluse-payload-sample.json` shape, asserting
the question string is recovered.

## 2. `init.rs` (T-020-02)

No new hook *file* (the command is inline JSON, like the attention catch-all). Changes are to
**validation only**:
- `validate` settings-binding check (`init.rs:654`, the `("on-notify", "Notification[attention]")`
  list) gains a `PreToolUse[AskUserQuestion]` expectation.
- The hook-file existence loop (`init.rs:680-708`) is unchanged — no new file to check.
- Tests near `init.rs:955-973` updated for the new expected binding (the "10 files" count is
  unaffected since no file is added).

## 3. `lib.rs` — plugin (T-020-03)

**State field** (`State`, after `notified_attention`, `lib.rs:241`):
```
/// Panes blocked on an AskUserQuestion call. Set when a `.awaiting` signal is
/// read; cleared on the next heartbeat (agent resumed after a human answered).
/// Suppresses injection and exempts the pane from timeout reclamation.
awaiting_human: HashSet<u32>,
```
`#[derive(Default)]` covers it — no `load()` change.

**New method `check_awaiting_signals(&mut self)`** modeled on `check_heartbeat_signals`
(`lib.rs:760-785`): scan `signal_dir`, match `pane-<id>.awaiting`, `remove_file`,
`self.awaiting_human.insert(pane_id)`. Does **not** bump activity (design Q6 safety). Called in
`poll_tick` **before** `check_idle_signals`/transition handling so the flag is live for this
tick (`lib.rs:1551-1560`).

**Clear** in `check_heartbeat_signals` (`lib.rs:783`, beside the `notified_attention.remove`):
```
self.awaiting_human.remove(&pane_id);
```

**Guard helper:**
```
fn is_pane_awaiting(&self, pane_id: u32) -> bool { self.awaiting_human.contains(&pane_id) }
```

**Injection guards** (design Q5 table):
- `send_line_to_pane` (`lib.rs:268`): early-return + log if `is_pane_awaiting(pane_id)` —
  the fail-safe chokepoint. (Take a `u32` or destructure `PaneId::Terminal`.)
- `schedule_ready_tickets` (`lib.rs:550/559`): skip the slot when awaiting (defensive; idle
  slots normally aren't awaiting).
- `handle_stopped_signal` (`lib.rs:1071`): return early before sending `/clear`.
- `handle_cleared_signal` (`lib.rs:1186`): return early before the prompt.
- `check_transition_timeouts` (`lib.rs:1245/1262`): skip awaiting panes in both fallback loops.
- `check_review_timeouts` (`lib.rs:1306`): drop awaiting candidates (highest clobber risk).

## 4. `lib.rs` — reclamation exemption (T-020-04)

Add `!self.awaiting_human.contains(&t.pane_id)` (or pane id via slot) to the candidate filters:
- `check_session_timeouts` reclaim branch (`lib.rs:1399-1440`) — keep the *warning* path, gate
  only the `timed_out.push` / fail+release.
- `detect_stale_threads` stale filter (`lib.rs:1484-1490`).
Both currently iterate `self.threads`; the thread's `pane_id` is on `Thread` (used at
`lib.rs:1299`), so the pane id is in scope.

## 5. `ui.rs` — surfacing (T-020-04)

Render an "⏸ awaiting human" marker for panes/threads whose pane id is in `awaiting_human`, so
an exempt-from-reclamation pane is never invisible on the dashboard. `ui.rs` already renders
per-thread status; this is one conditional badge. (ui has its own enums separate from
types.rs — per project memory — so this is presentation-only.)

## Ordering & boundaries

1. **T-020-02 first** — the hook must write `.awaiting` before the plugin can read it; its
   step 1 also closes the interactive Q2/Q4 validation gap.
2. **T-020-03** — plugin reads/guards (depends on the signal existing).
3. **T-020-04** — exemption + UI (depends on the flag existing).

Module boundaries respected: hook/CLI concerns stay in `lisa-cli` (`templates.rs`/`init.rs`);
all runtime/state concerns stay in `lisa-plugin` (`lib.rs`/`ui.rs`); `lisa-core` is untouched
(no new shared types needed — a `HashSet<u32>` is local plugin state, like `notified_attention`).
The signal-file contract is the only cross-boundary interface, and it reuses the existing
`pane-<id>.<ext>` convention end-to-end.
