# Plan — T-037-01-02 codex-startup-grace-pacing

Ordered, independently-verifiable steps. One file: `crates/lisa-plugin/src/lib.rs`.
Verify each with `cargo test -p lisa-plugin` (native) and, at the end, the WASM
build + full workspace suite.

## Step 1 — Constant + grace-deadline helper

- Add `STARTUP_GRACE_SECS: u64 = 8` near `MAX_ASSIGNMENT_DELIVERY_RETRIES`
  (structure §1).
- Add `startup_grace_deadline(&self, now)` beside `assignment_ack_deadline`
  (structure §2).
- **Verify:** `cargo build -p lisa-plugin --target wasm32-wasip1` compiles;
  `cargo clippy -p lisa-plugin` silent (helper is used in step 2, so no
  dead-code warning once step 2 lands — sequence steps 1+2 in one build).
- **Commit unit:** fold into step 2's commit (a lone unused const/helper would
  warn).

## Step 2 — Grace-aware deadline arming

- In `start_assignment_ack_wait`, rewrite the `Starting { start_deadline: None }`
  arm to pick `startup_grace_deadline(now)` for grace-mode panes and the existing
  `deadline` otherwise (structure §3).
- **Verify:** existing arming tests still see `start_deadline: Some(_)`:
  `test_pane_title_fresh_launch_uses_actual_fallback_route`,
  `scheduler_records_provider_readiness_mode_at_dispatch`,
  `test_missing_shell_readiness_fences_without_relaunch` (Claude) still green.
- **Commit:** `lisa commit-ticket --ticket-id T-037-01-02 --message
  "T-037-01-02: name a bounded startup grace for grace-mode seats" --include
  crates/lisa-plugin/src/lib.rs` (constant + helper + arming).

## Step 3 — Grace-aware expiry + fail-delivery widening

- Rewrite the `Starting { relaunches: 0 }` arm in `check_assignment_ack_timeouts_at`
  to fork on `seat_readiness_mode`: grace → `deliver_assignment_to_pane(.., 0,
  now)` with `fail_assignment_delivery` on error; else `begin_startup_recovery`
  (structure §4).
- Widen `fail_assignment_delivery`'s origin guard to include `Starting`
  (structure §5).
- **Verify:** this is where the behavioural change lands. Expect
  `same_pane_replacement_requires_start_and_chat_ack_for_both_providers` to now
  FAIL on its Codex branch — that is corrected in step 5. All Codex *recovery*
  tests and every Claude test must stay green (they ack/recover before expiry or
  are SessionStart-mode).
- **Commit:** `lisa commit-ticket --ticket-id T-037-01-02 --message
  "T-037-01-02: pace grace-mode first prompt from Starting into Delivering"
  --include crates/lisa-plugin/src/lib.rs`.

## Step 4 — New grace transition test

- Add `codex_startup_grace_paces_first_prompt_into_delivering` (structure §6):
  Codex grace → `Delivering` directly on expiry (never `ReadyForAssignment`/
  `StartupFailed`/`ResettingStartup`/`Owned`), then `Owned` only on exact-
  generation `UserPromptSubmit`; Claude arm still requires `acknowledge_process_
  start` for `ReadyForAssignment` and its expiry goes to `ResettingStartup`.
- **Verify:** `cargo test -p lisa-plugin codex_startup_grace_paces` passes.

## Step 5 — Split the divergent shared test

- Restructure `same_pane_replacement_requires_start_and_chat_ack_for_both_
  providers` to Claude-only, rename to `…_for_claude`, drop the provider loop,
  update the doc comment to point at step 4 for the Codex grace path (structure
  §7).
- **Verify:** `cargo test --workspace` — full suite green (was 286 + tests added
  by T-037-01-01; now +1 net new test, one renamed).
- **Commit:** `lisa commit-ticket --ticket-id T-037-01-02 --message
  "T-037-01-02: cover grace transition; scope shared replacement test to Claude"
  --include crates/lisa-plugin/src/lib.rs` (tests §6 + §7).

## Final verification

- `cargo build -p lisa-plugin --target wasm32-wasip1 --release` — clean.
- `cargo test --workspace` — all green.
- `cargo clippy -p lisa-plugin` — no new warnings.
- Confirm no ticket-owned file left staged/modified/untracked outside
  `lisa commit-ticket` (git status clean for `lib.rs`).

## Testing strategy summary

- **Unit/integration (native, injected time):** the grace transition
  (`Starting → Delivering` on grace elapse), the ownership gate
  (`Delivering → Owned` only on exact ack), and the Claude-unchanged path — all
  via `check_assignment_ack_timeouts_at(injected_deadline)`, no sleeping.
- **Regression:** full workspace suite proves Claude's SessionStart path, the
  Codex recovery contract, E-034 fencing, and E-035 no-inline-prompt/dquote
  recovery are intact.
- **Out of this ticket:** the two dedicated delayed-send + prompt-miss
  regressions (T-037-01-03) and the live two-provider rerun (S-037-02).

## Risk / rollback

Low. The behavioural fork is one match arm keyed on a mode that is already
recorded for every `Starting` seat. If grace pacing regressed, reverting steps
3–5 restores the prior uniform `begin_startup_recovery` behaviour. No state
variant, config, or adapter surface changes, so no cross-crate ripple.
