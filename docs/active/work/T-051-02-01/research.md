# T-051-02-01 — Research: the in-place reuse machinery

Descriptive map of what exists today, plus the field evidence the ticket requires
before any of it may be deleted.

---

## 1. The evidence gate (AC1)

The ticket and story both require, **before any code is removed**, at least one
ExitThenFresh field leg: 0.4.4-rc.8 or later, multiple consecutive Claude
tickets on one pane, journal-sealed clean.

**That leg exists. It is the 0.4.4-rc.8 Claude leg of 2026-07-18.**

| Fact | Value | Where it is recorded |
| --- | --- | --- |
| Version under test | 0.4.4-rc.8 | release commit `fc81b57`, 2026-07-18 |
| Reset strategy in that build | `ExitThenFresh` for native Claude | `ce1058e` "fix(scheduler): end every Claude session after its ticket" (2026-07-18 12:50:10 -0700) — the flip itself; rc.8 is the first release carrying it |
| Agent | Claude | `crates/lisa-plugin/src/adapter.rs:114-117` — "the 0.4.4-rc.8 **Claude** leg lost 8 of 9 usage records" |
| Tickets completed | **9 of 9, journal-sealed clean** | `20473fc` "fix(capture): never let a session exit destroy its own usage record": *"The rc.8 leg proved the seals and broke the ledger: 9 of 9 tickets journal-sealed clean, but only 1 of 9 usage captures landed (the rc.7 control leg on the same board captured 9 of 9)."* |
| Board | the standing demo board | `docker/chromebook-test/bin/prepare` seeds `/cbt/board/tickets/*.md` into `~/demo`; that directory holds exactly **9** ticket files (`T-001`…`T-009`) |
| Panes available | **4** | `Config::DEFAULT_MAX_THREADS = 2` (`crates/lisa-core/src/types.rs:725`); the generated layout is `2 × max_threads` panes (`crates/lisa-cli/src/loop_cmd.rs:534` pins `6 panes for max_threads=3`, `:702` pins `4 panes for max_threads=2`) |

**Consecutive tickets on one pane is arithmetic, not inference.** Nine tickets
ran to sealed completion across at most four panes, so at least one pane carried
⌈9/4⌉ = **3 consecutive Claude tickets**, every handoff through
`ExitThenFresh`. The leg's one defect was usage *attribution* (`tokens_in`/
`tokens_out` null — the Stop-hook capture race), which `20473fc` fixed in rc.9
and which S-051-03 is still closing out. Attribution is a ledger column; it is
not a seal. The seals were clean, 9 of 9, and the stale-identity failure that
motivated the flip did not recur.

**Corroborating, same boundary, current candidate:** this repository's own loop
provenance at 0.4.4-rc.10 — `.lisa/provenance.jsonl` last line, `T-051-01-02`,
method `claude`, pane 1, `outcome: done`, sealed by
`.lisa/completion-journal.jsonl` (`state: confirmed`, commit
`64ca71498c67876ace0776c50240cf3082a27cdd`). One ticket, so it does not carry
the gate on its own; it confirms the boundary is the one running right now.

**Conclusion: the gate is met. This ticket proceeds to removal; it does not
park.**

---

## 2. What the machinery is

Three states, two signals, four code sites. All in `crates/lisa-plugin/`.

### The state enum — `lib.rs:374-393`

```
TransitionState::Idle           live, every adapter
TransitionState::WaitingForExit live, the ExitThenFresh recycle path
TransitionState::WaitingForStop unreachable — ClearHandshake only
TransitionState::WaitingForClear unreachable — ClearHandshake only
TransitionState::Fenced         live, split-brain fencing
```

The doc comment at `lib.rs:382` already concedes the point: *"the documented
`ClearHandshake` seam; no shipped adapter reaches them"*.

### The strategy enum — `adapter.rs:77-99`

```
ClearHandshake   /clear in place, wait for .cleared, then prompt.  NO ADAPTER.
ExitThenFresh    /exit, bounded grace, fresh launch.  Claude (rc.8+) and Codex.
FreshExec        reuse is a fresh exec; no handshake.  #[allow(dead_code)] — no adapter.
```

Both shipped adapters return `ExitThenFresh`: `adapter.rs:311` (Claude),
`adapter.rs:441` (Codex).

### The four reachable-looking-but-dead code sites

1. **`schedule_ready_tickets` reuse arm** — `lib.rs:5196-5246`. The
   `ResetStrategy::ClearHandshake` match arm sends `/clear`, sets
   `WaitingForClear`, and stashes `adapter.reuse_prompt(&ctx)` as `launch_cmd`.
   The sibling arm at `lib.rs:5215-5217` is already an explicit
   `unreachable!("exit-then-fresh sessions enter the recycle branch")` — because
   `lib.rs:5019-5020` routes any reused seat whose adapter is `ExitThenFresh`
   into the recycle branch above (`/exit` → `WaitingForExit`) before this
   `else if` can be reached.

2. **`handle_stopped_signal` case 1** — `lib.rs:6397-6413`. On `.stopped`, if the
   slot is `WaitingForStop`, send `/clear` and advance to `WaitingForClear`. No
   slot can be `WaitingForStop`: nothing assigns that state outside tests. Case 2
   of the same function (idle slot + Review-phase ticket → auto-complete) is
   live and must survive untouched.

3. **`handle_cleared_signal`** — `lib.rs:6916-6980`. On `.cleared`, if the slot
   is `WaitingForClear`, publish a prompt lease marker and type
   `adapter.reuse_prompt(&ctx)` into the pane. Gated entirely on
   `WaitingForClear`; for every real pane the guard at `lib.rs:6923` is false and
   the function returns having done nothing.

4. **The two timeout fallbacks in `check_transition_timeouts`** —
   `lib.rs:7191-7203` (`StopTimedOut` → send `/clear` anyway) and
   `lib.rs:7205-7250` (`ClearTimedOut` → send the reuse prompt anyway). Both are
   fed by `DeadlineEvaluator::transitions` (`deadline.rs:75-93`), whose
   `WaitingForStop`/`WaitingForClear` arms can only fire for slots in those
   states. The third arm, `WaitingForExit → ExitReady` (`deadline.rs:69-74`,
   consumed at `lib.rs:7026-7189`), is the live recycle path and is load-bearing.

`STOP_SIGNAL_TIMEOUT_SECS` and `CLEAR_SIGNAL_TIMEOUT_SECS` feed
`TransitionPolicy` (`lib.rs:7003-7008`) and exist only for those two arms.

---

## 3. The `.cleared` signal — who writes it, who reads it

This is the AC4 boundary, and it is narrower than "delete everything named
clear".

**Written by:** the Claude `on-clear.sh` hook, generated by
`crates/lisa-cli/src/templates.rs:156-178` and installed by `init.rs:1920`. It
fires whenever `/clear` is processed in a Claude pane — **including when a human
types `/clear` themselves**, which has nothing to do with scheduling.

**Declared by:** `ClaudeCodeAdapter::signals()` → `cleared: true`
(`adapter.rs:328`). `CodexAdapter::signals()` → `cleared: false`
(`adapter.rs:458`), pinned by the test
`codex_signals_do_not_require_clear_handshake` (`adapter.rs:883`).

**Ingested by:** `signal::ingest(.., SignalRequest::Transitions)`
(`signal.rs:138,194`) → `check_transition_signals` (`lib.rs:6295-6298`), which
does two things with a `Cleared` record:

- `self.bump_pane_activity(pane_id)` — **live and useful.** A human `/clear` is
  genuine pane liveness and restarts the wind-down clock.
- `self.handle_cleared_signal(pane_id)` — the dead prompt delivery.

So the hook, the emission, the `SignalRecord::Cleared` variant, and the ingest +
`bump_pane_activity` all have a live consumer for a shipped adapter. Only the
prompt-delivery half is dead. Consuming the file also matters mechanically:
signal files are deleted on read, so dropping the ingest would leave `.cleared`
files accumulating in the signal directory forever.

---

## 4. Test inventory (feeds AC3)

Tests that touch the machinery, and which half they cover.

**Purely dead-path — cover only `WaitingForStop`/`WaitingForClear` behaviour:**

| Test | Location | Covers |
| --- | --- | --- |
| `test_check_transition_signals_stopped_advances_state` | `lib.rs:19221` | `.stopped` + `WaitingForStop` → `/clear` → `WaitingForClear` |
| `test_check_transition_signals_cleared_advances_state` | `lib.rs:19299` | `.cleared` + `WaitingForClear` → prompt → `Idle` |
| `test_check_transition_timeouts_stop_timeout` | `lib.rs:19396` | stop-timeout fallback forces `WaitingForClear` |
| `test_check_transition_timeouts_clear_timeout` | `lib.rs:19430` | clear-timeout fallback sends the prompt anyway |
| `test_stopped_signal_skips_when_awaiting` | `lib.rs:15147` | a `WaitingForStop` pane blocked on a question is not `/clear`-ed |
| `test_cleared_signal_skips_when_awaiting` | `lib.rs:15176` | a `WaitingForClear` pane blocked on a question gets no prompt |

**Mixed — assert dead states alongside live behaviour; need surgery, not
deletion:**

| Test | Location | Live part that must survive |
| --- | --- | --- |
| `test_transition_timeouts_skip_when_awaiting` | `lib.rs:15202` | the awaiting-human exemption itself |
| `test_check_transition_signals_cleared_ignored_when_idle` | `lib.rs:19344` | a `.cleared` on a non-handshake pane changes nothing — this becomes the *only* behaviour |
| `test_check_transition_timeouts_within_threshold_no_change` | `lib.rs:19482` | sub-threshold slots are untouched |
| `test_check_transition_timeouts_deferred_while_pane_active` | `lib.rs:21303` | activity defers timeouts |
| `characterizes_transition_deadline_and_active_session_exemption` | `lib.rs:21401` | deadline/exemption characterization across policies |
| `policy_specific_exemptions_are_preserved` | `deadline.rs:456` | per-policy exemption behaviour |
| `cross_policy_deadline_actions_remain_distinct` | `deadline.rs:526` | actions from different policies stay distinct |
| `cross_policy_activity_and_human_exemptions_remain_distinct` | `deadline.rs:678` | activity vs. human exemptions stay distinct |
| `passing_review_hostile_order_converges_once_and_schedules_dependent` | `tests/hostile_order_regression.rs:546` | hostile-order convergence; seeds `WaitingForClear` incidentally |
| `TestHarness::new` seeding | `tests/hostile_order_regression.rs:148` | harness fixture sets `WaitingForStop` on a primary slot |

**Live, unaffected:** `native_reset_exits_then_fresh` (`adapter.rs:619`),
`codex_signals_do_not_require_clear_handshake` (`adapter.rs:883`),
`native_signals_all_true` (`adapter.rs:653`), `native_reuse_prompt_matches_free_fn`
(`adapter.rs:588`), `codex_reuse_is_bare_prompt_for_live_tui` (`adapter.rs:822`),
and the whole `WaitingForExit` recycle suite.

The story's own line — *"down to a test asserting 'Claude must retain its clear
handshake'"* (E-051) — no longer matches the tree: `ce1058e` already replaced
that assertion with `native_reset_exits_then_fresh`. The belief it encoded is
gone; the machinery it defended is not.

---

## 5. Constraints and assumptions surfaced

- **N4 isolation.** Codex must not change by one byte. Codex already declares
  `cleared: false` and takes `ExitThenFresh`; nothing here is on a Codex path.
  The check is a byte-level diff review of Codex-touching lines, not a claim.
- **`reuse_prompt` outlives this ticket.** Both adapters implement it and the
  `FreshExec` reuse path does not call it — but `codex_reuse_is_bare_prompt_for_live_tui`
  and `pending_delivery_tags_reuse_and_bounded_reference_but_not_launch` pin its
  text, and it is the natural entry point for a future in-place integration.
  Whether it stays is a Design question, not a Research finding.
- **`FreshExec` is a separate dead-ish arm** (`#[allow(dead_code)]`, no adapter).
  It is *not* in this ticket's scope: the ticket names `WaitingForStop`,
  `WaitingForClear`, the timeout fallbacks, the `ClearHandshake` reuse arm, and
  `handle_cleared_signal`'s delivery. Touching `FreshExec` would violate N4's
  one-surface-per-ticket discipline.
- **`TransitionState` is serialized nowhere.** It is in-memory scheduler state
  only, so removing variants has no on-disk or cross-version compatibility cost.
- **The `unreachable!` at `lib.rs:5215` is load-bearing documentation.** If the
  `ClearHandshake` arm goes, that `unreachable!` and its match go with it — the
  match itself disappears rather than becoming a one-armed match on a
  two-variant enum.
- **Gates.** WASM build (`cargo build -p lisa-plugin --target wasm32-wasip1
  --release`), `cargo test --workspace`, and clippy must all be green. Per the
  0.4.4 rc train's own lesson (E-051) and prior guidance, gates are judged by
  exit code, never by grepping output.
