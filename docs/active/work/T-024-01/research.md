# T-024-01 Research — Codex loop parity validation

Descriptive map of the surfaces this validation exercises. What exists, where,
how it connects. No solutions — those are Design.

## What the ticket asks

End-to-end proof that a **Codex** loop gets the same scheduler treatment a Claude
loop gets: correct phase transitions, liveness (stuck detection stays honest under
long tool-free stretches, genuine hangs still reclaimed), prompt failure handling,
Review auto-completion + review-timeout finish-up, and a sane dashboard (no phantom
"awaiting"). It is S-024's "full parity is the bar" made *checkable*, in the spirit
of T-020-05's gate-harness (a scaffold + runbook for what CI cannot reach).

This ticket adds **no product behaviour** — every parity mechanism already exists
and is unit-tested in isolation (T-022-02 `.error`, T-023-01 wrapper, T-023-02
adapter). The gap is that nothing drives the pieces *together* as a Codex loop
lifecycle, and no live-codex run has ever been observed (no `codex` in CI).

## The two producers of Codex signals

- **`crates/lisa-cli/src/agent_exec.rs`** (T-023-01) — `lisa agent-exec`. Runs
  `codex exec --json`, streams JSONL, and writes the signal files:
  - `item.*` events ⇒ `pane-<id>.heartbeat` (best-effort progress).
  - Terminal decision in `Translator::finalize(exit_success)` (the *anchor rule*):
    `turn.completed && !turn.failed && exit 0` ⇒ `Outcome::Success` ⇒ `.stopped`;
    anything else ⇒ `Outcome::Failure` ⇒ `.error` **and** compat `.stopped`.
  - Persists `thread_id`→`.lisa/codex/<key>.thread` (for `--resume`) and `usage`.
  - `SignalWriter` no-ops when `LISA_PANE_ID` is absent (degrade-safely).
  - **Never** writes `.idle`/`.awaiting`/`.cleared` — those are Claude-only.
- **`crates/lisa-plugin/src/adapter.rs`** (T-023-02) — `CodexAdapter`. Builds the
  launch/reuse line (`LISA_PANE_ID=… LISA_TICKET_ID=… <lisa> agent-exec "<prompt>"`),
  the follow-up (`agent-exec --resume "<finish_up_prompt>"` as
  `FollowUp::SpawnCommand`), `reset_strategy() == FreshExec`, and `signals()` all
  `false`. `resolve_adapter[_or_native]` picks it when `config.client == Codex`.

## The consumers under validation (`crates/lisa-plugin/src/lib.rs`)

All are `State` methods driven each `poll_tick` (line ~1807). Every one is
native-testable via `State { config, signal_dir, .. }` + tempdir signal files +
`Thread`/`AgentSlot` literals — the idiom every existing consumer test uses.

| Consumer | Line | Codex-relevant behaviour |
|---|---|---|
| `check_heartbeat_signals` | 823 | `pane-<id>.heartbeat` resets thread stuck/stale clocks + slot wind-down. Codex `item.*` heartbeats keep an active session alive. |
| `check_artifact_advances` | 726 | Advances phase purely on **artifact file presence** (`research.md`…`review.md`); Implement→Review rides `review.md`. **Independent of any idle signal** — this is the parity load-bearer for Codex, which emits no `.idle`. Loops until fixpoint. |
| `handle_stopped_signal` | 1242 | Case 1 (`WaitingForStop`) sends `/clear` — Codex never reaches this (FreshExec ⇒ slot stays `Idle`). Case 2 (`Idle` + Review-phase ticket) ⇒ `auto_complete_review`. This is the `.stopped`→Review path for Codex. |
| `auto_complete_review` | 1298 | Marks Done **only if `all_dependencies_done`** (guard at 1310); updates ticket file, completes thread, releases slot, removes thread. |
| `check_error_signals` | 1175 | `pane-<id>.error` ⇒ fail thread + release slot + `error_alerts` push + `Failed` alert, **immediately**. Resolves via `threads` (authority), read-and-delete. Ordered before `check_transition_timeouts`. |
| `check_review_timeouts` | 1495 | Quiet Review thread past `review_timeout_secs` + `wind_down_secs`, not `finish_up_sent`, not awaiting ⇒ `resolve_adapter_or_native(...).follow_up(...)`. For Codex this yields `SpawnCommand("… agent-exec --resume …")`, delivered via `send_line_to_pane`. |
| `check_session_timeouts` / `detect_stale_threads` | 1630 / 1729 | Hard silence = **2× `stuck_threshold_secs`**. A heartbeating session never trips; genuine silence past the bar is reclaimed (thread `fail()` + slot release + retry). Awaiting-exempt (irrelevant for Codex). |
| `check_awaiting_signals` | 866 | Inserts into `awaiting_human` **only** on a `pane-<id>.awaiting` file. Codex writes none ⇒ set stays empty ⇒ `is_pane_awaiting` false everywhere. |
| `to_ui_state` | 2843 | Projects `awaiting: is_pane_awaiting(pane)` (2879) and `error_alerts`→`AlertType::Failed` (2985). "No phantom awaiting" ⇒ `awaiting` false for every Codex pane. |

## Config surface

`PluginConfig` (`crates/lisa-core/src/types.rs`): `client: AgentClient` (default
`Claude`), `lisa_bin: Option<String>`, `stuck_threshold_secs` (default 1200),
`review_timeout_secs` (600), `session_timeout_secs` (3600), `wind_down_secs` (300).
`AgentClient` (`crates/lisa-core/src/client.rs`): `Claude | Codex`, `parse`,
`as_str`, VALID. A Codex loop is selected loop-wide by `client = "codex"`.

## Key semantic differences the scheduler must not misread

1. **`.stopped` once per exec run**, not per Claude turn. In pre-Review phases a
   Codex `.stopped` lands on an `Idle` slot with a non-Review ticket ⇒
   `handle_stopped_signal` Case 2's `is_review` guard is false ⇒ **no-op**. Phase
   advance rides `check_artifact_advances` instead. Only in Review does `.stopped`
   trigger completion. This is correct by construction — worth a composition test.
2. **No `.idle`/`.awaiting`/`.cleared`.** The awaiting/attention machinery is a
   defined no-op (empty `awaiting_human`); phase advance never depends on `.idle`.
3. **FreshExec reset.** The `WaitingForClear`/`WaitingForStop` handshake machinery
   only arms in the `ClearHandshake` arm, so it never engages for a Codex pane
   (T-023-02 Decision 2). No signal gating needed.
4. **`.error` for prompt failure.** `turn.failed`/non-zero exit ⇒ `.error` ⇒
   thread failed promptly rather than waiting 2× stuck-threshold of silence.

## Test idioms (how this repo validates a scheduler behaviour)

- `State { dag, config: PluginConfig { … , ..PluginConfig::new() }, signal_dir,
  ..State::default() }`; push `AgentSlot { pane_id, ticket_id, has_session,
  transition_state, … }`; `state.threads.insert(id, Thread::new(id, pane))` then
  mutate `current_phase` / `last_phase_change` / `last_activity`.
- Tickets are real markdown scanned from a tempdir (`scan_tickets` → `Dag::from_tickets`)
  so `update_ticket_phase`/`all_dependencies_done` work against disk.
- `send_line_to_pane` is safe to call in native tests (existing review-timeout test
  relies on it); assertions target `finish_up_sent`, `activity_log`, `error_alerts`,
  `threads`, `agent_slots`, and the ticket file on disk — not the pane I/O itself.
- Precedent tests to mirror: `test_auto_complete_review_updates_ticket_and_cleans_up`
  (6783), `test_check_review_timeouts_sends_prompt_after_timeout` (6987),
  `test_check_error_signals_fails_running_thread` (7717), `test_detect_stale_threads`
  (3843), `test_check_artifact_advances_full_catchup` (3596).

## What CI can and cannot reach (the T-023 split)

- **In-process (automatable):** every scheduler consumer, driven with Codex config
  and Codex-shaped signal files. The full loop *lifecycle* can be composed natively
  (artifacts → advance → `.stopped` → auto-complete; heartbeat honesty; `.error`
  fail; review-timeout finish-up line shape; no phantom awaiting; per-pane
  attribution). This is the "recorded-stream native tests" analog at the scheduler
  level — the wrapper's own JSONL→signal translation is already fixture-tested in
  `agent_exec.rs`.
- **Live-codex only (manual):** the actual `codex exec` spawn/stream, real signal
  files landing in a real `.lisa/signals/`, real pane rendering, `--resume`
  re-entering a persisted `thread_id`. No `codex` binary in CI (T-023-01/02
  documented reality) ⇒ these become a **scaffold + PASS/FAIL runbook** (the
  T-020-05 pattern), judged from durable artifacts after an operator run.

## Constraints & assumptions

- **Codex JSON shape is `[PROVISIONAL]`** (T-023-01 Open-concern #1): the wrapper's
  pluck keys are reasoned from doc 05, not captured. Scheduler-side validation is
  insulated (it consumes signal *files*, not JSON), but the live run is also the
  first chance to reconcile the shape — note it, don't fix it here.
- **Loop-wide `client`.** A single loop is all-Claude or all-Codex today;
  per-pane `(provider, model)` routing is S-026. The AC's "one Claude + one Codex
  pane in the same loop" is therefore **not single-loop-achievable yet** — signal
  *attribution* is per-pane (`pane-<id>.*`) and testable, but true in-loop mixing
  is gated on S-026. This is a finding to surface, not a blocker to invent.
- **Shared dirty working tree.** T-023-02 / T-025-01 changes sit uncommitted in the
  tree (lisa's model: many threads, one branch, disjoint files). This ticket's
  footprint must be documented precisely and stay disjoint (test additions only).
</content>
</invoke>
