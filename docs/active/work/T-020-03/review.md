# T-020-03 Review — awaiting-human suppression

Handoff for a human reviewer. The feature makes the plugin consume the
`pane-<id>.awaiting` signal (written by T-020-02) and suppress all injection into a
question-blocked pane, so lisa never types `/clear` or a prompt over an agent's
`AskUserQuestion`.

## What changed

**One file:** `crates/lisa-plugin/src/lib.rs` (additive only — no signatures,
modules, or public interfaces changed; nothing deleted).

| # | Change | Location |
|---|---|---|
| 1 | New field `awaiting_human: HashSet<u32>` | beside `notified_attention` (`State`) |
| 2 | `check_awaiting_signals()` — reads `pane-<id>.awaiting`, inserts pane id, deletes file | beside `check_heartbeat_signals` |
| 3 | `is_pane_awaiting(pane_id) -> bool` accessor | beside `send_line_to_pane` |
| 4 | Clear `awaiting_human.remove(&pane_id)` on heartbeat | in `check_heartbeat_signals` (beside `notified_attention.remove`) |
| 5 | In-method guard: drop write + skip deferred Enter when target pane awaiting | top of `send_line_to_pane` |
| 6 | Five per-caller guards (skip/return before state mutation) | `schedule_ready_tickets`, `handle_stopped_signal`, `handle_cleared_signal`, `check_transition_timeouts` (×2 loops), `check_review_timeouts` |
| 7 | `check_awaiting_signals()` wired into `poll_tick` after heartbeat, before idle | `poll_tick` |
| 8 | 7 native unit tests | `mod tests`, after `test_attention_debounce_*` |

Net: +~110 lines (impl + tests), all additive.

## Acceptance-criteria coverage

- ✅ `awaiting_human: HashSet<u32>` on `State`.
- ✅ `check_awaiting_signals()` reads/inserts/deletes; called in `poll_tick`
  **before** `check_idle_signals`.
- ✅ Cleared in `check_heartbeat_signals` beside `notified_attention` clear.
- ✅ `is_pane_awaiting()` added.
- ✅ Injection guarded in `send_line_to_pane` **and** at all five callers (skip /
  early-return semantics exactly per the Q5 table).
- ✅ **Liveness safety:** no new code touches `last_activity_at` /
  `bump_pane_activity` (verified by diff grep — the only matches are test-fixture
  struct literals). The flag gates writes only; reclaim exemption left to T-020-04.
- ✅ Tests (native): per-guarded-caller no-advance, heartbeat-clears, signal
  insert+delete.
- ✅ `just check` passes (WASM check + full workspace suite).

## Test coverage & how it's verified

7 new tests, all green:
- `test_check_awaiting_signals_inserts_and_deletes` — signal consumed + flag set.
- `test_heartbeat_clears_awaiting` — heartbeat un-sets the flag.
- `test_is_pane_awaiting_accessor` — accessor truth table.
- `test_stopped_signal_skips_when_awaiting` — `WaitingForStop` stays put (no /clear).
- `test_cleared_signal_skips_when_awaiting` — `WaitingForClear` stays put (no prompt).
- `test_transition_timeouts_skip_when_awaiting` — timed-out pane not force-advanced.
- `test_review_timeout_skips_when_awaiting` — no finish-up prompt, not marked sent.

Full suite: lisa-plugin **171**, lisa-cli 172, lisa-core 106 — all pass; clippy clean.

**Useful property:** the four caller tests drive paths that would reach
`send_line_to_pane` (a zellij host call that aborts in native tests) if their guard
were missing. A green run is therefore positive evidence the suppression holds — a
regressed guard would surface as a panic, not a silent pass. This is the de-facto
test for the in-method guard, which can't be exercised directly natively.

## Gaps & open concerns

1. **`schedule_ready_tickets` guard (#1) is untested** — it only targets idle slots
   (no awaiting agent normally present), so it's defensive per the ticket, and its
   non-awaiting happy path reaches `send_line_to_pane` and can't run natively. Left
   to inspection. Low risk: empty-set `contains` is always false, so it can't
   misfire on normal scheduling.
2. **Stale `.awaiting` on an abandoned pane.** If an agent asks a question then the
   session dies before answering, the flag never clears (no heartbeat) and the pane
   stays suppressed until stale-reclaim runs on the silence clock. This is intended
   (gating a dead pane's writes is harmless), and reclaim itself is **T-020-04's**
   job. Flagged here so the reviewer knows it's deliberate, not an oversight.
3. **No dashboard surfacing.** "Awaiting human" isn't shown in the UI (not in AC).
   Reasonable follow-up: a banner/indicator so an operator sees which pane is
   waiting. Out of scope here.
4. **`PaneId` non-Terminal variants** fall through the in-method guard (write
   proceeds). No caller uses anything but `PaneId::Terminal` today, so this is moot,
   but worth noting if plugin-pane injection is ever added.

## Risk assessment

Lowest-risk change class. Worst-case failure mode is a *suppressed* injection that
should have fired — the pane simply waits one more poll tick (fails safe), never a
clobber. No existing behavior changes when `awaiting_human` is empty (the common
case), since every guard is a `HashSet::contains` that returns false on an empty set.

## Recommendation

Ready to merge. Highest-value review target: confirm each per-caller guard sits
**before** its state mutation (it does — the no-advance tests prove it) and that the
liveness invariant (no activity-clock writes) is acceptable as the boundary with
T-020-04.
