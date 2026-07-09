# T-022-01 · Progress

## Completed

- **Step 1** — `ticket_prompt`, `build_claude_command`, `finish_up_prompt` made
  `pub(crate)` (visibility only). `lib.rs`.
- **Step 2** — Added `crates/lisa-plugin/src/adapter.rs`: `AgentAdapter` trait
  with AC-4 module/trait docs, `SpawnContext`/`FollowUpContext`,
  `ResetStrategy`, `FollowUp`, `SignalCapabilities`, `ClaudeCodeAdapter`
  (delegates to the free functions), `resolve_adapter` (per-ticket, MVP→Claude)
  + `resolve_adapter_or_native` (None → native). 7 module unit tests.
  Declared `mod adapter;` and imported the seam types into `lib.rs`.
- **Step 3** — Fresh launch in `schedule_ready_tickets` routed through
  `adapter.launch_command(&SpawnContext{..})`.
- **Step 4** — Session reuse routed through `adapter.reset_strategy()` +
  `reuse_prompt`; `handle_cleared_signal` and the `check_transition_timeouts`
  clear-timeout fallback routed through `resolve_adapter_or_native(..).reuse_prompt`.
  `FreshExec` arm is `unreachable!` (documented Codex seam).
- **Step 5** — `check_review_timeouts` follow-up routed through
  `adapter.follow_up(..)`; `TypeIntoPane` preserves the exact `send_line_to_pane`
  path, `SpawnCommand` is `unreachable!` (documented Codex seam).

## Deviations from plan

- None structural. Added `resolve_adapter_or_native` (foreseen as R2 in plan.md)
  as a real helper in `adapter.rs` rather than inline, so all three reuse sites
  share one fallback path.
- The three free functions are now referenced only from `adapter.rs` (+ tests) in
  non-test `lib.rs` code; kept `pub(crate)` (not moved) so the string-anchor
  tests in `lib.rs` resolve them unchanged via `super::*`.

## Verification

- `cargo test --workspace` → **all green**: lisa-core 175, lisa-cli 106,
  lisa-plugin **184** (177 pre-existing + 7 new `adapter::tests`). No existing
  test modified.
- Anchor tests confirmed passing unmodified: `test_build_claude_command*`,
  `test_check_transition_signals_*`, `test_*_skips_when_awaiting`,
  `test_check_review_timeouts_*`.
- `cargo clippy -p lisa-plugin --lib` → **0 warnings**. `--all-targets` shows only
  pre-existing `field_reassign_with_default` warnings in `ui.rs` / existing
  `lib.rs` tests (not touched by this change).
- `cargo build -p lisa-plugin --target wasm32-wasip1 --release` → **Finished**
  (trait objects `Box<dyn AgentAdapter>` compile object-safe under WASM).

## Post-clippy tidy

- Dropped the unused `resolve_adapter` re-import from `lib.rs` (only
  `resolve_adapter_or_native` is called there).
- Scoped `#[allow(dead_code)]` (each commented with the consuming ticket) on the
  three declared-for-future seams with no MVP consumer: `FollowUpContext.pane_id`,
  `SignalCapabilities`, and `AgentAdapter::signals`. `ResetStrategy::FreshExec`
  and `FollowUp::SpawnCommand` were already annotated.
