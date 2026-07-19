# T-051-02-01 — Design: retire the in-place reuse machinery

Research established the evidence gate is met (0.4.4-rc.8 Claude leg, 9 of 9
tickets journal-sealed clean across at most 4 panes, every handoff through
`ExitThenFresh`). This document decides *how much* comes out, and settles the
one question the ticket delegates here: does `ResetStrategy::ClearHandshake`
survive as a documented seam, or go entirely.

---

## The decision, first

**`ClearHandshake` goes entirely, along with `WaitingForStop`,
`WaitingForClear`, both timeout fallbacks, the reuse arm, and
`handle_cleared_signal`. The `.cleared` signal — hook, emission, record variant,
ingest, and its `bump_pane_activity` liveness effect — stays.**

The seam that survives is `AgentAdapter::reset_strategy()` itself: a trait method
returning an enum. A future in-place-reset integration adds its variant and its
machinery together, in one reviewable change, against a scheduler that no longer
pretends to already support it.

---

## Options considered

### Option 1 — Keep everything; mark it unreachable

Add `#[allow(dead_code)]`, sharpen the doc comments, leave the code.

This is roughly where the tree already is. `lib.rs:382` says *"no shipped adapter
reaches them"*; `lib.rs:5216` is a literal `unreachable!`; `lib.rs:6937` and
`lib.rs:7214` both carry the parenthetical *"no shipped adapter reaches this —
removal tracked by T-051-02-01"*. The comments are honest and it still isn't
enough. Every reader of `check_transition_timeouts` still traces three timeout
arms to understand two. Every reader of `handle_stopped_signal` reads a two-case
function to find the one case that runs. P2's complaint is not that the code is
undocumented; it is that it is *there*. **Rejected** — this option is the status
quo the ticket exists to end.

### Option 2 — Delete the machinery, keep `ClearHandshake` as a documented seam

Remove the states, the timeout arms, and the delivery, but keep the enum variant
with a comment explaining what a future adapter would need to build.

Superficially the conservative choice, and it is the one the ticket explicitly
puts on the table. It is the worst of the three.

An enum variant is a *promise the type system makes*: `ResetStrategy` is what an
adapter returns to tell the scheduler how to reset it. A variant with no
scheduler behaviour behind it means an adapter can return `ClearHandshake` and
get — depending on how the remaining `match` is written — either a panic or a
silent no-op where a reset was expected. The next integrator reads the variant,
believes the handshake exists, returns it, and discovers over a debugging session
that they must write the whole state machine anyway. The variant would have cost
them time rather than saved it. Research already recorded the precedent: a test
once asserted "Claude must retain its clear handshake" and everyone believed it.
A vestigial variant is the same trap in enum form. **Rejected.**

### Option 3 — Delete `ClearHandshake` and its machinery entirely (chosen)

`ResetStrategy` keeps `ExitThenFresh` (both shipped adapters) and `FreshExec`
(the declared headless/bridge arm). `TransitionState` keeps `Idle`,
`WaitingForExit`, and `Fenced` — every one of which a real pane occupies.
`check_transition_timeouts` has one timeout arm, and it is the one that runs.

The objection to answer: *doesn't this lose the design knowledge?* No. It moves
it to where design knowledge belongs — git history (`ce1058e` explains why
in-place reuse fails for environment-carried identity; `20473fc` and `1fc57c1`
explain the completion-boundary consequences; this ticket's artifacts explain the
removal) and the surviving comment on `ExitThenFresh`, which states *why* the
process boundary is the only reset that re-exports per-ticket identity. A future
adapter whose identity does **not** live in process environment can read that
comment, see the condition it must violate, and build accordingly. What it must
not do is inherit a half-built state machine it did not test.

**Chosen.** It is the only option where nobody reading the scheduler finds a
state no code path can reach, and where the seam that remains — the trait method
— is a decision rather than leftovers.

---

## The `.cleared` boundary (AC4)

The one place where "delete the reuse machinery" must **not** become "delete
everything named clear".

Research established the split:

- **Emission stays.** `crates/lisa-cli/src/templates.rs` generates
  `on-clear.sh`; `init.rs` installs it. It fires on *any* `/clear` in a Claude
  pane, including one a human types. Native Claude declares `cleared: true`
  (`adapter.rs:328`) and that declaration remains true.
- **Ingest stays.** `SignalRecord::Cleared`, `signal::ingest`, and the
  `check_transition_signals` arm stay — but the arm keeps only
  `bump_pane_activity`. A human `/clear` is genuine pane liveness and should
  restart the wind-down clock. Consuming the file is also load-bearing
  mechanically: signal files are deleted on read, so dropping the ingest would
  leave `.cleared` files accumulating in the signal directory indefinitely.
- **Delivery goes.** `handle_cleared_signal` — the prompt-typing half — is the
  dead part, and only that.

Codex declares `cleared: false` and is untouched by every line of this change
(N4). `codex_signals_do_not_require_clear_handshake` (`adapter.rs:883`) survives
verbatim as the pin on that.

## What `handle_stopped_signal` keeps

The function has two cases (`lib.rs:6385-6460`). Case 1 — `WaitingForStop` →
send `/clear` — is dead and goes. Case 2 — an `Idle` slot whose ticket is in
Review, auto-completed as Done — is the live Codex and Claude completion path
and must survive byte-identical in behaviour. This is the single highest-risk
edit in the ticket: the dead case is the *early return* guarding the live one,
so removing it changes control flow, not just line count. Plan sequences it
alone, with the auto-complete tests as its gate.

## Deadline-evaluator scope

`DeadlineEvaluator::transitions` (`deadline.rs:54-98`) has three arms. Two go
(`WaitingForStop`/`WaitingForClear`), one stays (`WaitingForExit → ExitReady`,
the live recycle path). With them go `TransitionAction::StopTimedOut` and
`ClearTimedOut` (`deadline.rs:254-260`), the `stop_timeout_secs` /
`clear_timeout_secs` fields of `TransitionPolicy` (`deadline.rs:244-245`), and
their two constants in `lib.rs:91,97`.

`deadline.rs`'s cross-policy tests exist to prove that actions from *different*
deadline policies stay distinct — a real invariant that outlives this change.
They are rewritten around the surviving action set, not deleted; Structure names
each one.

---

## Explicitly out of scope

- **`ResetStrategy::FreshExec`.** Also `#[allow(dead_code)]` with no adapter, and
  a tempting rider. It is a different seam with a different justification
  (headless/ACP bridges), it is not named by the ticket, the story, or the epic,
  and N4 is explicit that each ticket isolates its own surface. Left alone.
- **`AgentAdapter::reuse_prompt`.** Both adapters implement it; its text is
  pinned by `codex_reuse_is_bare_prompt_for_live_tui` and
  `pending_delivery_tags_reuse_and_bounded_reference_but_not_launch`; the
  `FreshExec` arm is its remaining caller-in-principle. Removing it would drag
  the Codex assignment-text surface into a scheduler-machinery ticket. Left
  alone.
- **Any behaviour change to the `WaitingForExit` recycle path**, the completion
  boundary, or usage capture. S-051-03 owns the ledger work.

## How this is verified

1. **Compiler as proof of unreachability.** Deleting the enum variants makes
   every remaining reference a hard error. If the machinery were reachable from
   somewhere Research missed, the build says so by name and line. This is
   stronger than any audit and is the reason the enum variants come out *first*
   in the Plan's ordering rather than last.
2. **Gates by exit code** — WASM target build, `cargo test --workspace`, clippy —
   never by reading grepped output.
3. **Coverage mapping (AC3).** Every deleted test is written down in
   `progress.md` against either its successor test or the removed path it
   covered. Structure carries the table; progress records what actually
   happened.
4. **N4 byte check.** `git diff` reviewed for any Codex-touching line. The
   expectation is zero.
