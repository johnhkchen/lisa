# T-022-01 · Plan — implementation steps

Ordered, independently-verifiable steps. Testing strategy per step. Each step is
small enough to commit atomically. The through-line is: **prove no-op at every
step** by keeping the existing suite green.

## Testing strategy

- **No-op anchors** (must pass unmodified at every step):
  `test_build_claude_command`, `test_build_claude_command_includes_env_vars`,
  `test_build_claude_command_includes_rdspi_reference`,
  `test_check_transition_signals_stopped_advances_state`,
  `test_check_transition_signals_cleared_advances_state`,
  `test_stopped_signal_skips_when_awaiting`,
  `test_cleared_signal_skips_when_awaiting`,
  `test_transition_timeouts_skip_when_awaiting`,
  `test_check_review_timeouts_sends_prompt_after_timeout`,
  `test_check_review_timeouts_idempotent`.
- **New tests** (in `adapter.rs`): equality of adapter output vs free functions,
  reset strategy, follow-up variant, signal capabilities, resolver returns a
  usable adapter. These *characterize* the seam without changing behaviour.
- **Commands** per step: `cargo test --workspace` (native) and, at the end,
  `cargo build -p lisa-plugin --target wasm32-wasip1 --release` +
  `cargo clippy --workspace` (no new warnings). `just check` as the umbrella.

## Step 1 — Make shared free functions `pub(crate)`

- Change `fn ticket_prompt`, `fn build_claude_command`, `fn finish_up_prompt` to
  `pub(crate) fn` in `lib.rs`.
- Verify: `cargo test --workspace` — all green (pure visibility change).
- Commit: `refactor(plugin): make launch/prompt helpers pub(crate) for adapter`.

## Step 2 — Add `adapter.rs` (trait + native impl + resolver), unwired

- Create `crates/lisa-plugin/src/adapter.rs` per structure.md:
  - `use std::path::Path; use lisa_core::types::Ticket; use crate::{build_claude_command, ticket_prompt, finish_up_prompt};`
  - Trait `AgentAdapter` with the AC-4 doc comment (native Codex + ACP fit).
  - `SpawnContext<'a>`, `FollowUpContext<'a>`, `ResetStrategy`
    (`#[derive(Debug, PartialEq, Eq, Clone, Copy)]`), `FollowUp`
    (`#[derive(Debug, PartialEq, Eq, Clone)]`), `SignalCapabilities`
    (`#[derive(Debug, PartialEq, Eq, Clone, Copy)]`).
  - `ClaudeCodeAdapter` + impl delegating to the free functions.
  - `resolve_adapter(ticket: &Ticket) -> Box<dyn AgentAdapter>` → `_ = ticket;
    Box::new(ClaudeCodeAdapter)`.
  - `#[cfg(test)] mod tests`:
    - `native_launch_matches_free_fn`: `ClaudeCodeAdapter.launch_command(&ctx)
      == build_claude_command(dir, id, pane)`.
    - `native_reuse_prompt_matches_free_fn`: same for `ticket_prompt`.
    - `native_follow_up_is_type_into_pane`: matches
      `FollowUp::TypeIntoPane(finish_up_prompt(...))`.
    - `native_reset_is_clear_handshake`, `native_signals_all_true`.
    - `resolver_returns_claude`: resolved adapter's reset == `ClearHandshake`.
      (Build a minimal `Ticket` via its constructor/`Default` — confirm the
      construction path in `types.rs`; if no `Default`, build the literal.)
- Add `mod adapter;` + the `use adapter::{…}` glob-ish import in `lib.rs`. Because
  nothing calls the imports yet, gate with `#[allow(unused_imports)]` **only if**
  the compiler complains — prefer wiring in the same commit to avoid dead-code
  warnings (see Step 3 note).
- Verify: `cargo test --workspace` — existing green, new adapter tests pass.
- Commit: `feat(plugin): add AgentAdapter trait, ClaudeCodeAdapter, resolver`.

## Step 3 — Wire fresh-launch through the adapter

- In `schedule_ready_tickets`, in the `!has_session` branch, fetch the ticket
  (`self.dag.get_ticket(&ticket_id)`), resolve the adapter, and build the launch
  command via `adapter.launch_command(&SpawnContext { ticket_dir:
  &host_ticket_dir, ticket_id: &ticket_id, pane_id })`.
  - Borrow care: `resolve_adapter` needs `&Ticket` from `self.dag`; the branch
    also mutates `self.agent_slots` / calls `self.send_line_to_pane`. Resolve the
    adapter and produce the `String` *before* the `&mut self` calls (the command
    is an owned `String`, so no borrow overlap). Clone `ticket.phase` etc. as
    already done at `:594` — reuse that fetch to also drive resolution.
- Verify: `test_build_claude_command*` + full suite green.
- Commit: `refactor(plugin): route fresh launch through AgentAdapter`.

## Step 4 — Wire reuse + `.cleared` + clear-timeout prompt

- Reuse branch (`has_session`): `match adapter.reset_strategy()` →
  `ClearHandshake` executes today's code (stash prompt =
  `adapter.reuse_prompt(&ctx)`); `FreshExec => unreachable!(...)`.
- `handle_cleared_signal`: replace `ticket_prompt(...)` with
  `resolve_adapter(ticket).reuse_prompt(&ctx)`; if `self.dag.get_ticket` returns
  `None`, fall back to `ClaudeCodeAdapter` directly (R2). Preserve the
  `is_pane_awaiting` guard exactly.
- `check_transition_timeouts` clear-timeout arm: same `reuse_prompt`
  substitution.
- Verify: `test_check_transition_signals_*`, `test_cleared_signal_skips_when_awaiting`,
  `test_transition_timeouts_skip_when_awaiting` green.
- Commit: `refactor(plugin): route session reuse + cleared prompt through adapter`.

## Step 5 — Wire follow-up through the adapter

- `check_review_timeouts`: resolve adapter, `match adapter.follow_up(&ctx)` →
  `TypeIntoPane(s)` calls `send_line_to_pane(&s, …)` (unchanged path);
  `SpawnCommand(_)` is the documented Codex seam (native never returns it).
  Keep the `is_pane_awaiting` skip, `finish_up_sent` insert, and
  `mark_phase_change` exactly as-is.
- Verify: `test_check_review_timeouts_sends_prompt_after_timeout` +
  `_idempotent` green.
- Commit: `refactor(plugin): route review follow-up through adapter`.

## Step 6 — Verification sweep

- `cargo test --workspace` (all green, count matches pre-change + new adapter
  tests).
- `cargo build -p lisa-plugin --target wasm32-wasip1 --release` (trait objects
  compile under WASM).
- `cargo clippy --workspace --all-targets` — no new warnings (watch for
  `dead_code` on `FreshExec`/`SpawnCommand` — silence with a targeted
  `#[allow(dead_code)]` + a comment naming T-023-02, *not* a blanket allow).
- `just check`.
- Commit: `test(plugin): verify adapter refactor is a no-op` (if any test/lint
  tweaks), else fold into Step 5.

## Risks & mitigations

- **R1 — dead-code warnings on unused enum arms** (`FreshExec`, `SpawnCommand`,
  `SignalCapabilities` fields). Mitigate with a scoped `#[allow(dead_code)]` on
  those items, each commented `// consumed by T-023-02 (Codex) / T-022-02`.
  Do not blanket-allow the module.
- **R2 — ticket missing from DAG at `.cleared`/timeout time.** The reuse-prompt
  sites currently call `ticket_prompt` directly with just the id and never need
  the `Ticket`. Resolution needs `&Ticket`; if `get_ticket` is `None` (rare: DAG
  mid-rebuild), fall back to `ClaudeCodeAdapter` so the prompt still goes out —
  behaviour identical to today. Encode as a small helper
  `resolve_adapter_or_native(opt_ticket)`.
- **R3 — borrow checker on `resolve_adapter(&Ticket)` vs `&mut self`.** Adapter
  output is owned (`String` / small enums); compute all adapter results into
  locals before any `&mut self` call. No `&self.dag` held across mutation.
- **R4 — no-op regression slips in.** Every step re-runs the anchor tests; steps
  are per-call-site so a failure bisects to one site. If an anchor test needs
  editing, stop — that means it is *not* a no-op and the design is violated.
- **R5 — WASM build breaks on `Box<dyn>`.** Trait objects are `alloc`-only and
  already used elsewhere; Step 6 builds the wasm target explicitly to catch any
  `?Sized`/object-safety issue early. The trait is object-safe (no generics, no
  `Self`-by-value, no associated consts).

## Definition of done (maps to AC)

- Trait + native impl covering launch, reuse/reset, follow-up, expected-signal-set ✓
- Spawn-time per-ticket resolver, MVP → native Claude unconditionally ✓
- Existing tests pass unmodified; strings/signals/transitions identical ✓
- Trait doc comment shows native-Codex + ACP fit without redesign ✓
- No Codex behaviour implemented (seams are `unreachable`/unused) ✓
