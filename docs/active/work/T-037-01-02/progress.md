# Progress — T-037-01-02 codex-startup-grace-pacing

## Completed (matches plan)

- **Step 1–2 — named grace + arming** (commit `ba5a513`):
  - Added `STARTUP_GRACE_SECS = 8` (lib.rs, beside `MAX_ASSIGNMENT_DELIVERY_RETRIES`).
  - Added `startup_grace_deadline(now)` helper (saturating).
  - `start_assignment_ack_wait` now arms a grace deadline for grace-mode panes and
    the unchanged acceptance-clock deadline for SessionStart-mode panes.
  - Same commit also landed Step 3 (expiry fork + `fail_assignment_delivery`
    widening) so the constant/helper are not momentarily dead.
- **Step 3 — grace-aware expiry** (in commit `ba5a513`):
  - `check_assignment_ack_timeouts_at`'s `Starting { relaunches: 0 }` arm forks on
    `seat_readiness_mode`: grace → `deliver_assignment_to_pane(.., 0, now)` (with
    `fail_assignment_delivery` on submit error); SessionStart →
    `begin_startup_recovery` (unchanged).
  - `fail_assignment_delivery` origin guard widened to accept `Starting`.
- **Step 4–5 — tests** (commit `d6525a7`):
  - New `codex_startup_grace_paces_first_prompt_into_delivering`: Codex grace →
    `Delivering` directly on expiry (never ReadyForAssignment/StartupFailed/
    ResettingStartup/Owned), then `Owned` only on the exact-generation
    `UserPromptSubmit`; a stale generation is rejected.
  - New `session_start_seat_never_paces_on_grace_and_still_requires_the_signal`:
    Claude reaches `ReadyForAssignment` only via `acknowledge_process_start`, and
    a Claude `Starting` deadline enters `ResettingStartup`, never Delivering.
  - Split `same_pane_replacement_requires_start_and_chat_ack_for_both_providers`
    → `…_for_claude` (dropped the provider loop; the Codex primary-expiry branch
    it used to assert is now the grace path covered above).

## Deviations from plan

- Steps 1–3 were committed together rather than as two commits, to avoid a
  transient dead-code/unused-helper state between commits. Behaviourally
  identical to the plan; the test commit remains separate.

## Verification

- `cargo test --workspace` → **288 passed / 0 failed** (286 baseline + 2 new;
  one test renamed, no net-removed coverage).
- `cargo clippy -p lisa-plugin` → silent.
- `cargo build -p lisa-plugin --target wasm32-wasip1 --release` → clean.
- `git status` for `crates/lisa-plugin/src/lib.rs` → clean (all changes committed
  through `lisa commit-ticket`; nothing staged/modified/untracked).

## Not in this ticket (by design)

- The two dedicated delayed-send + prompt-miss regressions → T-037-01-03.
- The live metered two-provider rerun → S-037-02.
- No `acknowledge_process_start` guard and no `PluginConfig` knob — see review.md
  "Open concerns" for the rationale (E-034 recovery contract / file ownership).
