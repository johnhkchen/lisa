# Review — T-037-01-02 codex-startup-grace-pacing

## What changed

For grace-mode providers (Codex), the bounded named startup grace on the
existing `Starting` state now **paces the first prompt**: when the grace elapses,
Lisa submits the bounded attempt-tagged assignment and enters `Delivering`
directly, without ever treating elapsed time as readiness, startup failure, or
ownership. Claude's SessionStart-gated path is byte-for-byte unchanged. The whole
change is one file, `crates/lisa-plugin/src/lib.rs`.

### Source (commit `ba5a513`)
- **`STARTUP_GRACE_SECS = 8`** — a named, bounded startup grace constant
  (mirrors the `AGENT_EXIT_GRACE_SECS = 8` "let the TUI settle" precedent).
- **`startup_grace_deadline(now)`** — saturating absolute grace deadline.
- **`start_assignment_ack_wait`** — the `Starting { start_deadline: None }` arm
  now arms `startup_grace_deadline(now)` for grace-mode panes
  (`seat_readiness_mode == Grace`) and the unchanged `assignment_ack_deadline`
  for SessionStart-mode panes. `Some(_)` presence is unchanged.
- **`check_assignment_ack_timeouts_at`** — the `Starting { relaunches: 0 }` arm
  forks: grace mode → `deliver_assignment_to_pane(pane_id, generation, 0, now)`
  (→ `Delivering { retries: 0 }`), routing a submit error to
  `fail_assignment_delivery`; SessionStart mode → `begin_startup_recovery`
  (unchanged).
- **`fail_assignment_delivery`** — origin guard widened to accept `Starting` so a
  grace send that cannot be submitted resolves in a named `DeliveryFailed`
  instead of silently remaining `Starting`.

### Tests (commit `d6525a7`)
- **`codex_startup_grace_paces_first_prompt_into_delivering`** (new) — the ticket
  AC in one injected-time test.
- **`session_start_seat_never_paces_on_grace_and_still_requires_the_signal`**
  (new) — the Claude contrast: signal-gated `ReadyForAssignment`, deadline →
  `ResettingStartup`, never a paced `Delivering`.
- **`same_pane_replacement_requires_start_and_chat_ack_for_claude`** (renamed
  from `…_for_both_providers`, Codex branch removed) — the SessionStart same-pane
  replacement contract that stays intact.

## Acceptance-criteria mapping

> An injected-time native test shows a Codex seat in Starting with a named grace
> deadline attempting the bounded assignment when the grace elapses and entering
> Delivering directly on successful submission — never ReadyForAssignment,
> StartupFailed, or Owned merely because time passed — while a Claude seat still
> requires a matching process-start signal to reach ReadyForAssignment; no path
> publishes Owned without exact prompt acknowledgement.

- ✅ Codex Starting with a **named grace deadline**:
  `codex_startup_grace_paces…` reads `Starting { start_deadline: Some(grace),
  relaunches: 0 }` classified `Grace`.
- ✅ **Grace elapse → Delivering directly**: `check_assignment_ack_timeouts_at(
  grace_deadline)` yields `Delivering { retries: 0 }`; the test also asserts the
  UI status is `Delivering`, proving it is not `ReadyForAssignment`/`StartupFailed`.
  It is not `Owned` and not `ResettingStartup`.
- ✅ **Never Owned from time**: `!seat_is_owned` after grace elapse; `Owned` only
  after an exact-generation `acknowledge_assignment`, and a stale generation is
  rejected.
- ✅ **Claude still requires the signal**:
  `session_start_seat_never_paces…` reaches `ReadyForAssignment` only via
  `acknowledge_process_start` and shows a Claude deadline → `ResettingStartup`.
- ✅ **No Owned without exact ack**: the sole `Delivering → Owned` edge remains
  `acknowledge_codex_assignment` (unchanged), asserted directly.

## Test coverage

- The new transition is exercised through the real production entry point
  (`check_assignment_ack_timeouts_at`) at injected time — no sleeping, no PTY,
  no tokens.
- Full workspace suite: **288 passed / 0 failed**. Claude's SessionStart path,
  the Codex recycle/recovery contract (E-034/E-035), same-pane dquote recovery,
  and no-inline-prompt launch all remain green.
- Clippy silent; WASM release build clean.

### Gaps (by design, deferred to T-037-01-03)
- The **prompt-miss** lifecycle past first delivery (grace → Delivering → bounded
  retry → `DeliveryFailed`, never Owned, stale-attempt rejection at each step) is
  reused-but-unchanged machinery here; its dedicated deterministic regression is
  T-037-01-03. This ticket proves the *entry* transition and the ownership gate.
- The grace **submit-error → DeliveryFailed** branch (e.g. pane awaiting human at
  grace elapse) is wired but not separately asserted; the happy path and the
  Owned gate are. A targeted assertion is a natural addition in T-037-01-03.

## Open concerns / handoff notes

- **`acknowledge_process_start` was deliberately NOT guarded** against grace-mode
  seats. The ticket says "reserve ReadyForAssignment for Claude," and a structural
  guard would enforce that literally — but the recovery-fresh Codex `Starting`
  (from `begin_assignment_recovery`'s post-exit relaunch) is grace-classified and
  four existing E-034/E-035 recovery tests drive it to `Owned` through a
  *synthetic* `acknowledge_process_start`. Guarding would force altering the
  E-034 recovery contract, which the story explicitly does not reopen. The
  reservation holds **in practice** because Codex emits no pre-prompt
  process-start signal, so grace expiry is the only route a primary Codex seat
  leaves `Starting`; and *time* never triggers `acknowledge_process_start` (a
  signal does), so "never ReadyForAssignment merely because time passed" is
  satisfied without the guard. If a future ticket wants the structural guarantee,
  the seam is `acknowledge_process_start` (early-return on `Grace`) plus rewriting
  those four recovery tests to the grace path — a bonus is that the recovery-fresh
  Codex would then also grace-pace in the real world (it already does on expiry).
- **Grace duration is a `lib.rs` constant, not a `PluginConfig` knob.** Config
  parsing lives in `types.rs`, outside this ticket's file ownership. Promoting
  `STARTUP_GRACE_SECS` to a configurable field is a clean, isolated follow-up.
- **`start_deadline` now carries two meanings** (SessionStart-wait bound vs.
  startup grace) discriminated by `seat_readiness_mode`. Documented in the arming
  arm; no new state variant was added to keep the change within N4's "no broad
  rewrite" boundary.

## Risk assessment

Low. One behavioural fork keyed on a mode already recorded for every `Starting`
seat, plus one widened match guard and a named constant/helper. No new state
variant, no config or adapter surface change, no cross-crate ripple. Reverting the
expiry fork restores the prior uniform behaviour. The full suite passing —
including every Claude and Codex recovery test — is the proof the shared ownership
boundary and E-034 fencing are intact.
