# T-051-02-01 — Review

**Disposition: pass.** One source commit (`5a88ea8`), −733/+94 lines across four
files, all gates green by exit code.

---

## What changed

Four files, all in `crates/lisa-plugin/`. Nothing created, nothing deleted,
`crates/lisa-cli/**` and `lisa-core` untouched.

**`adapter.rs`** — `ResetStrategy::ClearHandshake` removed. The
`AgentAdapter::reset_strategy()` trait default flipped from `ClearHandshake` to
`ExitThenFresh`, so an implementor who forgets to override now inherits the
boundary that works instead of a handshake nothing implements.
`ExitThenFresh`'s doc gained the design knowledge the deleted variant used to
carry: what an in-place reset would require, and that rebuilding it is a
deliberate change adding strategy and machinery together.

**`deadline.rs`** — `TransitionAction::StopTimedOut` and `ClearTimedOut` removed
along with their match arms and the `quiet` computation. `TransitionPolicy`
collapsed from four fields to one (`exit_grace_secs`); `TransitionInput` lost
`last_activity` and `awaiting_human`. `transitions()` is now a single-arm
filter_map over `WaitingForExit`.

**`lib.rs`** — `TransitionState::WaitingForStop` and `WaitingForClear` removed;
`STOP_SIGNAL_TIMEOUT_SECS` and `CLEAR_SIGNAL_TIMEOUT_SECS` removed;
`handle_cleared_signal` removed entirely; `handle_stopped_signal`'s
mid-transition case removed (the Review auto-complete case survives); the
`ClearHandshake` arm removed from `schedule_ready_tickets`; both timeout loops
removed from `check_transition_timeouts`. The `.cleared` ingest arm keeps
`bump_pane_activity` and drops the delivery call.

**`tests/hostile_order_regression.rs`** — two fixture states re-seated from
`WaitingForStop`/`WaitingForClear` to `Idle`. Assertions about completion
convergence unchanged.

## Acceptance criteria

| AC | Status |
| --- | --- |
| 1 — research cites the ExitThenFresh field evidence by run before any code is removed | **Met.** `research.md` §1 cites the 0.4.4-rc.8 Claude leg of 2026-07-18: 9 of 9 tickets journal-sealed clean (`20473fc`), 9-ticket standing demo board, at most 4 panes (`DEFAULT_MAX_THREADS = 2`, layout `2 × max_threads`) — so at least one pane carried 3 consecutive Claude tickets. Written before the first edit. |
| 2 — no scheduler path can send `/clear` as a scheduling transport; the states and fallbacks are removed | **Met, no remnant.** Both states and both fallbacks deleted outright; no `/clear` send remains in the scheduler. |
| 3 — every deleted test maps in writing to its successor or the removed path | **Met.** `progress.md` carries the full table: 8 deleted (each mapped to its removed path), 4 rewritten with named successors, 2 fixtures re-seated. |
| 4 — `.cleared` survives where a shipped adapter consumes it; Codex unchanged by one byte | **Met.** Hook, emission, `SignalRecord::Cleared`, ingest, and `bump_pane_activity` all retained; only the prompt-delivery half removed. N4 verified by diff read: zero production Codex lines changed. |
| 5 — WASM check, workspace tests, clippy all green | **Met**, plus `cargo fmt --check`. All by exit code. |

## Test coverage

434 tests pass in `lisa-plugin`; `cargo test --workspace` exits 0.

The strongest verification here is not a test. Design chose to delete the enum
variants *first* so the compiler would enumerate every remaining reference by
name and line. It produced exactly 8 errors, at exactly the 8 sites Research had
inventoried, none on a shipped-adapter path. That is a proof of unreachability
rather than an assertion of it.

**Where coverage genuinely narrows, and why it is correct.** Two deleted tests —
`test_transition_timeouts_skip_when_awaiting` and
`test_check_transition_timeouts_deferred_while_pane_active` — pinned invariants
(a pending question exempts a transition timeout; recent pane activity defers
one) that applied only to the stop/clear arms. `WaitingForExit` is deliberately
exempt from neither: a pane told to `/exit` is leaving, and holding it back would
strand the seat. Re-seating those assertions on the surviving arm would have
asserted something false. The equivalent exemptions on the review, session, and
stale policies are live and stay covered by their own `deadline.rs` tests.

**Where coverage improves.** `cross_policy_deadline_actions_remain_distinct` now
feeds `Idle` and `Fenced` slots past the deadline alongside `WaitingForExit` and
asserts exactly one action returns — a stronger claim than the old version. And
the two hostile-order fixtures now exercise duplicate-`.stopped` convergence
through the live `Idle` path instead of a state no pane occupies, so they test
what actually happens in the field.

## Open concerns

**1. `AgentAdapter::reuse_prompt` now has zero callers and is marked
`#[allow(dead_code)]`.** This is the one piece of residue and it deserves a
reviewer's attention rather than a footnote.

Design predicted `reuse_prompt` would survive because `FreshExec` was its
caller-in-principle. That was wrong — `FreshExec` re-execs via `launch_command` —
and the compiler said so once the last real caller was deleted. Deleting it
would cascade into `CodexAdapter::assignment_prompt` and remove
`codex_reuse_is_bare_prompt_for_live_tui` plus part of
`pending_delivery_tags_reuse_and_bounded_reference_but_not_launch`, which pin
Codex delivery text — inside a ticket whose N4 boundary is "Codex unchanged by
one byte." So it was marked and documented rather than removed as an unrecorded
rider.

Honest framing: this ticket's thesis is that code reading as live while never
running is a defect. A `#[allow(dead_code)]` method with a comment saying "no
caller remains" is a materially smaller and *labelled* version of that, using the
idiom `adapter.rs` already applies to `FreshExec`, `SignalCapabilities`, and
`CompletionExit::Immediate`. It is not zero. **A follow-up ticket scoped to the
adapter's delivery surface should decide whether `reuse_prompt` and the
`FreshExec` arm live or die together** — that is the right shape for it, and it
is a different surface from this one.

**2. The evidence is cited from commit messages, not a collected leg artifact.**
The rc.8 leg's "9 of 9 tickets journal-sealed clean" is recorded in `20473fc`'s
message and corroborated by `adapter.rs`'s own comments; the leg's journals were
not collected into `docs/active/work/` the way the T-046-06-03 legs were. The
pane-reuse conclusion is arithmetic from the board size (9 tickets) and pane
count (≤4), both verifiable in the tree today, not from a pane-by-pane
transcript. I consider this sufficient — the seal count is the load-bearing
fact and it is recorded by the person who ran the leg — but a reviewer who wants
a pane-level record should know it is derived, not transcribed.

**3. `ResetStrategy::FreshExec` remains `#[allow(dead_code)]` with no adapter.**
Pre-existing, explicitly out of scope per N4's one-surface-per-ticket rule, and
untouched. Named here so it is not mistaken for something this change
introduced.

## For a human reviewer

The one edit worth reading closely is `handle_stopped_signal`. Its dead case was
an *early return* guarding the live Review auto-complete, so removing it changed
control flow rather than just line count. The result is a function with a single
`if transition_state == TransitionState::Idle` block; the auto-complete tests
(`test_auto_complete_review_condition_*`,
`test_codex_stopped_auto_completes_review_respecting_deps`) and both
hostile-order regressions pass unchanged.

Everything else is subtraction the compiler verified.
