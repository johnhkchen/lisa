# T-020-03 Plan — awaiting-human suppression

Ordered, independently-verifiable steps. Single file: `crates/lisa-plugin/src/lib.rs`.
Verification gate per step is `cargo test -p lisa-plugin` (fast) with a final
`just check` (WASM check + full workspace tests).

## Step sequence

### S1 — Add the field (C1)
Add `awaiting_human: HashSet<u32>` after `notified_attention` (`lib.rs:241`) with
doc comment. `HashSet` is already imported (used by `notified_attention`).
**Verify:** `cargo build -p lisa-plugin` compiles (field unused → allow via use in S2).

### S2 — Add `check_awaiting_signals()` + `is_pane_awaiting()` (C2, C4)
Place `check_awaiting_signals` right after `check_heartbeat_signals` (~`lib.rs:785`)
and `is_pane_awaiting` near it. Now the field is read.
**Verify:** compiles; no clippy warnings for the new fns.

### S3 — Clear on heartbeat (C3)
Add `self.awaiting_human.remove(&pane_id);` beside the `notified_attention.remove`
at `lib.rs:783`.
**Verify:** compiles.

### S4 — In-method guard in `send_line_to_pane` (C5)
Add the `PaneId::Terminal(id)` awaiting check + early return at the top of the
method, before `write_chars_to_pane_id`.
**Verify:** compiles.

### S5 — Per-caller guards (C6)
Add the five guards in order: `schedule_ready_tickets`, `handle_stopped_signal`,
`handle_cleared_signal`, `check_transition_timeouts` (two loops), `check_review_timeouts`.
Each guard precedes the relevant state mutation.
**Verify:** compiles; existing tests still green (`cargo test -p lisa-plugin`).

### S6 — Wire into `poll_tick` (C7)
Insert `self.check_awaiting_signals();` between `check_heartbeat_signals` and
`check_artifact_advances` (`lib.rs:1552`).
**Verify:** compiles.

### S7 — Tests (7 new)
Append the seven tests from `structure.md` to `mod tests`.
**Verify:** `cargo test -p lisa-plugin` — all new + existing pass.

### S8 — Full gate
`just check` (WASM `cargo check --target wasm32-wasip1` + `cargo test --workspace`).
**Verify:** green. Run `cargo clippy -p lisa-plugin` for lint cleanliness.

## Testing strategy

**Unit (native), the only feasible layer here** — see `research.md` test
constraints (`send_line_to_pane` can't run natively). Coverage map:

| AC | Test |
|---|---|
| `check_awaiting_signals` inserts + deletes signal file | S7.1 |
| flag clears on heartbeat | S7.2 |
| `is_pane_awaiting` accessor | S7.3 |
| stopped caller: no state-machine advance when awaiting | S7.4 |
| cleared caller: no advance when awaiting | S7.5 |
| transition-timeout caller: no advance when awaiting | S7.6 |
| review-timeout caller: no finish-up sent when awaiting | S7.7 |

**Not unit-testable, verified by construction + review:**
- The in-method `send_line_to_pane` drop (zellij host call) — verified by code
  inspection + the fact that the per-caller tests would *panic on a host call* if
  the guard were missing (they exercise paths that would otherwise reach it). This
  is a useful property: a missing guard surfaces as a test failure, not a silent
  pass.
- `schedule_ready_tickets` skip (#1) — defensive per the ticket; covered by
  inspection. A direct test is impractical (the method reaches `send_line_to_pane`
  on the *non-awaiting* happy path and can't run natively). Documented in review.

**Liveness safety (AC):** asserted structurally — grep confirms no
`bump_pane_activity` / `last_activity_at` write in any new code. Called out in
review.md as the load-bearing invariant.

## Atomic-commit boundary
All steps land as one logical change (a half-implemented guard set is incoherent),
but S1–S6 (impl) and S7 (tests) are a natural two-commit split if incremental
commits are desired. `progress.md` tracks actual landing.

## Rollback / risk
Lowest-risk change class: additive private field + methods + guarded early-returns.
Worst case of a bug is a *suppressed* injection that should have fired — which
fails safe (a pane waits a tick longer) rather than clobbering. The only way to
regress existing behavior is a guard that triggers when `awaiting_human` is empty,
which the `HashSet::contains` check structurally prevents (empty set ⇒ always false).
