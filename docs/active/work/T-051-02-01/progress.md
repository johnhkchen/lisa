# T-051-02-01 — Progress

**Status: complete.** One source commit, `5a88ea8`, all gates green by exit
code.

---

## What was done

Plan's eight steps, executed in order. The removal landed as a single source
commit because the enum removal cascades across `adapter.rs`, `deadline.rs`, and
`lib.rs` — no smaller subset compiles, and a commit that does not build is not a
meaningful unit. Plan anticipated this and said so.

| Step | Outcome |
| --- | --- |
| 1 — remove enum variants, read the blast radius | **8 compile errors, at exactly the 8 sites Research named.** No surprises. |
| 2 — `adapter.rs` | `ClearHandshake` gone; trait default now `ExitThenFresh` |
| 3 — `deadline.rs` production | two actions, two arms, `quiet`, three policy fields, two input fields gone |
| 4 — `lib.rs` production | timeout loops, `handle_cleared_signal`, stopped-case-1, reuse arm, two constants gone |
| 5 — test surgery | 8 deleted, 4 rewritten, 2 fixtures re-seated |
| 6 — gates | WASM 0, tests 0, clippy 0, fmt 0 |
| 7 — N4 Codex check | zero production Codex lines changed |
| 8 — artifacts | this file + review |

### Step 1's result is the ticket's core claim, verified

Deleting the variants first was chosen precisely so the compiler — not an audit —
would enumerate every reference. It produced errors at `lib.rs:5200, 5204, 6392,
6400, 6917, 7194` and `deadline.rs:75, 84`. Every one was on the machinery
Research had already inventoried. Nothing on a shipped-adapter path referenced
the removed states. That is the strongest available proof the machinery was
unreachable, and it is why AC2 is satisfied by construction rather than by
assertion.

---

## AC3 — every deleted test mapped

**Deleted, mapping to the removed path they covered.** These tested behaviour
that no longer exists; there is no successor because there is no successor
behaviour.

| Deleted test | Covered |
| --- | --- |
| `test_check_transition_signals_stopped_advances_state` | `.stopped` + `WaitingForStop` → `/clear` → `WaitingForClear` |
| `test_check_transition_signals_cleared_advances_state` | `.cleared` + `WaitingForClear` → reuse prompt → `Idle` |
| `test_check_transition_timeouts_stop_timeout` | stop-timeout fallback forcing `WaitingForClear` |
| `test_check_transition_timeouts_clear_timeout` | clear-timeout fallback sending the prompt anyway |
| `test_stopped_signal_skips_when_awaiting` | awaiting-human guard on the `/clear` send |
| `test_cleared_signal_skips_when_awaiting` | awaiting-human guard on the reuse-prompt send |
| `test_transition_timeouts_skip_when_awaiting` | awaiting-human exemption on the stop/clear timeouts |
| `test_check_transition_timeouts_deferred_while_pane_active` | pane activity deferring the clear-timeout fallback |

**A note on the last two, because it is the one place coverage genuinely
narrows.** Their invariants — a pending question and recent pane activity each
exempt a transition timeout — applied only to the stop/clear arms.
`WaitingForExit` is deliberately exempt from neither: a pane that has been told
to `/exit` is leaving, and holding it back on activity would strand the seat.
So these do not transfer to the surviving arm, and re-seating them on
`WaitingForExit` would have asserted a fiction. The equivalent exemptions on the
review, session, and stale policies are real and remain covered by their own
tests in `deadline.rs`. This is a deliberate, recorded reduction, not an
oversight.

**Rewritten, with named successors.**

| Before | After | What the successor pins |
| --- | --- | --- |
| `test_check_transition_signals_cleared_ignored_when_idle` | `test_cleared_signal_is_liveness_only` | a `.cleared` bumps pane activity, is consumed so files cannot accumulate, drives no transition, and types nothing — now the *only* `.cleared` behaviour |
| `test_check_transition_timeouts_within_threshold_no_change` | same name, re-seated on `WaitingForExit` inside the grace | sub-threshold no-op |
| `characterizes_transition_deadline_and_active_session_exemption` | same name, transition half reduced to the two `WaitingForExit` slots | grace elapsed → fresh launch; inside grace → keep waiting |
| `policy_specific_exemptions_are_preserved` / `cross_policy_deadline_actions_remain_distinct` / `cross_policy_activity_and_human_exemptions_remain_distinct` (`deadline.rs`) | transition clauses rebuilt | actions from different policies stay distinct; the transition policy now demonstrably has *no* exemption inputs, stated in a comment rather than asserted by a vanished case |

`cross_policy_deadline_actions_remain_distinct` gained coverage: it now feeds
`Idle` and `Fenced` slots past the deadline alongside `WaitingForExit` and
asserts only one action comes back. That is a stronger statement than the old
three-state version.

**Fixtures re-seated (subject unchanged).**

| Site | Change |
| --- | --- |
| `TestHarness::new` (`hostile_order_regression.rs`) | seeds `Idle` instead of `WaitingForStop` |
| `lost_result_duplicate_stop_fixture_converges_on_single_prior_commit` | asserts the slot stays `Idle` on a duplicate `.stopped`; the completion-effect count assertion is unchanged |

Both hostile-order regression tests still pass. Their subject — that a duplicate
`.stopped` during an in-flight completion launches no second completion — is now
exercised through the live `Idle` path rather than a state no pane occupies,
which makes them more faithful than before.

**Net: 434 tests pass in `lisa-plugin`; `cargo test --workspace` exits 0.**

---

## Deviations from the plan

**One, surfaced by the compiler in Step 2.**

Design and Structure both said `AgentAdapter::reuse_prompt` would be left alone,
reasoning that the `FreshExec` arm was its remaining caller-in-principle. That
reasoning was wrong: `FreshExec` re-execs through `launch_command`, so once
`handle_cleared_signal` and the clear-timeout fallback were gone, `reuse_prompt`
had **zero** callers and rustc's dead-code lint said so.

Resolved by marking it `#[allow(dead_code)]` with a comment stating plainly that
no caller remains, why it is still here, and what removing it would cost. That
follows the idiom already established in `adapter.rs` for `FreshExec`,
`SignalCapabilities`, and `CompletionExit::Immediate`.

Deleting it instead was considered and rejected: it would take
`CodexAdapter::assignment_prompt` with it and delete
`codex_reuse_is_bare_prompt_for_live_tui` plus half of
`pending_delivery_tags_reuse_and_bounded_reference_but_not_launch` — real pins on
Codex delivery text, inside a ticket whose N4 boundary is "Codex unchanged by one
byte" and whose named surface is scheduler machinery. Left as a flagged
follow-up rather than an unrecorded rider. It is carried into `review.md` as an
open concern, not buried here.

**A second, smaller consequence** that Structure did predict: removing the two
dead arms made `TransitionPolicy::wind_down`, `stop_timeout_secs`,
`clear_timeout_secs` and `TransitionInput::last_activity`, `awaiting_human`
unread. They were removed too. Leaving them would have recreated this exact
defect one layer down, and clippy would have failed the gate anyway.

---

## Gates (exit codes, not grepped output)

```
cargo build -p lisa-plugin --target wasm32-wasip1 --release   0
cargo test --workspace                                        0
cargo clippy --workspace --all-targets -- -D warnings         0
cargo fmt --check                                             0
```

`cargo fmt --check` failed once on a stray blank line left by the timeout-loop
removal; `cargo fmt` fixed it and the re-run exits 0. Clippy was re-run after
the format pass and still exits 0.

## Commit

`5a88ea8` — `refactor(scheduler): retire the in-place /clear reuse machinery`,
via `lisa commit-ticket` with four exact `--include` paths:

```
crates/lisa-plugin/src/lib.rs
crates/lisa-plugin/src/adapter.rs
crates/lisa-plugin/src/deadline.rs
crates/lisa-plugin/src/tests/hostile_order_regression.rs
```

Net **−733 / +94** lines. No ticket-owned file is left staged, modified, or
untracked. `crates/lisa-cli/**` was never touched, so T-051-01-01's surface
(`triage_agent.rs`) stayed disjoint as S-051's wave rationale intended.

## N4 verification (performed, not assumed)

`git diff -U0` filtered for Codex-touching lines returned three hits: one new doc
comment that mentions the Codex delivery surface, and two deleted
`last_client: Some(AgentClient::Codex)` lines inside removed *test fixture*
slots. Zero production Codex lines changed. `CodexAdapter::signals()` still
returns `cleared: false`; `codex_signals_do_not_require_clear_handshake` passes
verbatim.
