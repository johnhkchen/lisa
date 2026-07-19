# T-051-02-01 — Structure: the shape of the removal

Four files change. No file is created. No file is deleted. Two crates are
touched; `lisa-core` is not touched at all.

```
crates/lisa-plugin/src/lib.rs          production + tests   (the bulk)
crates/lisa-plugin/src/deadline.rs     production + tests
crates/lisa-plugin/src/adapter.rs      production (enum variant + doc only)
crates/lisa-plugin/src/tests/hostile_order_regression.rs   fixture states only
```

`crates/lisa-cli/**` is untouched: the `on-clear.sh` hook and its installer stay
exactly as they are (design's `.cleared` boundary).

---

## The cascade, and why ordering matters

Removing the two `TransitionState` variants is not a local edit. Both dead
timeout arms are the *only* consumers of three inputs the evaluator carries:

| Input | Used by | After removal |
| --- | --- | --- |
| `TransitionPolicy::wind_down` | the `quiet` computation, read only by the stop/clear arms | dead |
| `TransitionPolicy::stop_timeout_secs` / `clear_timeout_secs` | those arms | dead |
| `TransitionInput::last_activity` | feeds `quiet` | dead |
| `TransitionInput::awaiting_human` | the two arms' guards | dead |

The surviving `WaitingForExit → ExitReady` arm reads only `started`,
`pane_id`, `ticket_id`, and `exit_grace_secs`. So `TransitionPolicy` collapses to
a single field and `TransitionInput` loses two. This is the honest consequence of
the deletion, not scope creep: leaving unread fields behind would recreate the
exact "reads as live, never runs" defect one layer down, and clippy would flag
them anyway.

**Ordering follows from this.** The enum variants come out *first*, so the
compiler enumerates every remaining reference by name and line. That is the
verification method design chose over auditing.

---

## `crates/lisa-plugin/src/adapter.rs`

**Removed**

- `ResetStrategy::ClearHandshake` (lines 80-87) — the variant and its doc block.

**Modified**

- The `ResetStrategy` enum doc (line 77) — restate what the enum is *for* now
  that it describes two live-ish strategies rather than three.
- `ExitThenFresh`'s doc comment (lines 88-94) — this is where the design
  knowledge lands. It already explains that the process boundary is the only
  reset that re-exports fresh per-ticket identity; it gains one sentence naming
  the condition a future in-place-reset adapter would have to satisfy (identity
  not carried in process environment), so the seam is legible without a
  vestigial variant.

**Unchanged, deliberately**

- `AgentAdapter::reset_strategy()`'s default body (line 216) currently returns
  `ClearHandshake`. It must return something; it becomes `ExitThenFresh` — the
  safe default, since a trait implementor who forgets to override now gets the
  boundary that works rather than a handshake nothing implements.
- `reuse_prompt`, `SignalCapabilities`, `CompletionExit`, `ReadinessMode`, both
  adapters' `signals()`, and every Codex line. `cleared: true` for Claude stays
  true; `cleared: false` for Codex stays false.

---

## `crates/lisa-plugin/src/deadline.rs`

**Removed**

- `TransitionAction::StopTimedOut` and `TransitionAction::ClearTimedOut`
  (lines 254-260).
- The two match arms in `transitions()` (lines 75-93) and the now-unread `quiet`
  local (lines 65-67).
- `TransitionPolicy::wind_down`, `stop_timeout_secs`, `clear_timeout_secs`
  (lines 242-245) — leaving `exit_grace_secs` alone.
- `TransitionInput::last_activity`, `TransitionInput::awaiting_human`
  (lines 237-238).

**Result:** `transitions()` becomes a single-arm filter_map over
`WaitingForExit`. `AcknowledgementInput`, `ReviewInput`, `SessionInput`,
`HealthInput` and their policies are untouched — the `awaiting_human` and
wind-down exemptions on *those* policies are live and stay.

**Tests modified in this file**

| Test | Change | Why |
| --- | --- | --- |
| `policy_specific_exemptions_are_preserved` (456) | drop its `transitions(...)` clause; keep the review + session exemption clauses | the transitions policy no longer *has* a human exemption — asserting one would be fiction |
| `cross_policy_deadline_actions_remain_distinct` (526) | rebuild around `ExitReady` vs. the other policies' actions | the invariant (actions from different policies stay distinct) is real and survives; only the action set shrank |
| `cross_policy_activity_and_human_exemptions_remain_distinct` (678) | drop its transition-policy inputs, keep review/session/health | same reason as above |

---

## `crates/lisa-plugin/src/lib.rs`

### Production

**Removed**

- `STOP_SIGNAL_TIMEOUT_SECS` (91) and `CLEAR_SIGNAL_TIMEOUT_SECS` (97) with
  their doc comments.
- `TransitionState::WaitingForStop` and `WaitingForClear` (389-392).
- `handle_stopped_signal` case 1 (6397-6413) — the `WaitingForStop` early
  return. **Case 2 (Review auto-complete) survives unchanged**; the function
  keeps its name and signature and its doc comment is rewritten to describe one
  case rather than two.
- `handle_cleared_signal` entirely (6916-6980) and its call site in
  `check_transition_signals` (6297).
- The `ClearHandshake` arm in `schedule_ready_tickets` (5206-5214). With it, the
  `match adapter.reset_strategy()` loses a third of its arms; the
  `ExitThenFresh => unreachable!(...)` arm (5215-5217) stays — it still documents
  a real invariant (`lib.rs:5019-5020` routes those seats to the recycle branch).
- Both timeout loops in `check_transition_timeouts`: `stop_timeouts`
  (7191-7203) and `clear_timeouts` (7205-7250), their `Vec` declarations
  (7012-7013), and their `match` arms (7019-7022).

**Modified**

- `TransitionState`'s enum doc (379-383) — drop the "documented `ClearHandshake`
  seam" sentence; every remaining variant is one a live pane occupies.
- `check_transition_signals` (6286-6302) — the `SignalRecord::Cleared` arm keeps
  `bump_pane_activity(pane_id)` and loses `handle_cleared_signal(pane_id)`. Its
  doc comment (6279-6285) is rewritten: `.stopped` drives Review auto-complete;
  `.cleared` is consumed as liveness (a human `/clear` in a Claude pane) and
  drives no scheduling.
- `check_transition_timeouts` (6991-7009) — the `TransitionPolicy` literal
  reduces to `exit_grace_secs`, and the `TransitionInput` map drops
  `last_activity` / `awaiting_human`. `exit_ready` handling (7026-7189) is
  untouched.

**Not touched**

`AgentSlot::last_activity_at` (still read by session/health deadlines and
`bump_pane_activity`), `awaiting_human` the set (still read by
`is_pane_awaiting`, review timeouts, and the scheduling guard at 5030),
`prompt_artifact_dir`, `publish_prompt_lease_marker`, `SignalRecord::Cleared`,
`signal.rs` in its entirety.

### Tests

**Deleted (map to the removed path they covered — AC3)**

| Test | Line | Covered |
| --- | --- | --- |
| `test_check_transition_signals_stopped_advances_state` | 19221 | `.stopped` + `WaitingForStop` → `/clear` → `WaitingForClear` |
| `test_check_transition_signals_cleared_advances_state` | 19299 | `.cleared` + `WaitingForClear` → prompt → `Idle` |
| `test_check_transition_timeouts_stop_timeout` | 19396 | stop-timeout fallback |
| `test_check_transition_timeouts_clear_timeout` | 19430 | clear-timeout fallback |
| `test_stopped_signal_skips_when_awaiting` | 15147 | awaiting guard on the `/clear` send |
| `test_cleared_signal_skips_when_awaiting` | 15176 | awaiting guard on the reuse prompt |
| `test_transition_timeouts_skip_when_awaiting` | 15202 | awaiting exemption on the stop/clear timeouts |
| `test_check_transition_timeouts_deferred_while_pane_active` | 21303 | activity defers the clear-timeout fallback |

The last two deserve a note in progress: their invariants (human-exemption and
activity-deferral on *transition* timeouts) do not transfer to `WaitingForExit`,
which is deliberately exempt from neither. They map to the removed path, not to
a successor. The equivalent exemptions on review/session/health policies remain
covered by their own tests.

**Rewritten (successor named)**

| Test | Line | Successor |
| --- | --- | --- |
| `test_check_transition_signals_cleared_ignored_when_idle` | 19344 | renamed to `test_cleared_signal_is_liveness_only`: a `.cleared` bumps pane activity, consumes the file, and changes no transition state — now the *only* `.cleared` behaviour |
| `test_check_transition_timeouts_within_threshold_no_change` | 19482 | re-seated on `WaitingForExit` inside the exit grace; keeps the sub-threshold no-op invariant |
| `characterizes_transition_deadline_and_active_session_exemption` | 21401 | keeps the session-exemption half; the transition half re-seats on `WaitingForExit` |

**Fixture edits only**

| Site | Line | Change |
| --- | --- | --- |
| `TestHarness::new` | `hostile_order_regression.rs:148` | seed `Idle` instead of `WaitingForStop` |
| `passing_review_hostile_order_converges_once_and_schedules_dependent` | `hostile_order_regression.rs:569` | assert on the state the live path actually produces |

Both are hostile-ordering regression tests about completion convergence. Their
subject is unrelated to reuse; they merely borrowed a state name. Their
assertions about convergence and dependent scheduling must not change.

---

## Commit units

Five, each independently buildable and testable, in this order:

1. `deadline.rs` — actions, arms, policy/input fields, and this file's three tests.
2. `adapter.rs` — the `ClearHandshake` variant, the default `reset_strategy`, docs.
3. `lib.rs` production — states, consts, the reuse arm, the timeout loops, the
   `check_transition_signals` arm, `handle_cleared_signal`, `handle_stopped_signal`
   case 1.
4. `lib.rs` + `hostile_order_regression.rs` tests — deletions, rewrites, fixtures.
5. `progress.md` coverage map (AC3) — the written record.

Steps 1-3 will not compile in isolation from each other (the enum removal
cascades across files), so they land as one `lisa commit-ticket` unit if the
compiler forces it; Plan resolves that and records what actually happened.
