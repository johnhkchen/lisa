# Progress — T-052-02-01 say-it-once

Status: **complete**. Three commits, `just check` green by exit code.

| Commit | Message | Plan step |
|---|---|---|
| `4df44a0` | `fix(plugin): announce one phase transition once across all detectors` | 1 (partial) + 2 |
| `2865b2a` | `fix(plugin): mint one feed line per phase transition` | 3 |
| `6220fac` | `fix(plugin): demote the scheduling skip into the state dump` | 1 (rest) + 4 + 5 |

Only `crates/lisa-plugin/src/lib.rs` was touched, as `structure.md` predicted.
`ui.rs` and `lisa-core` are untouched.

---

## Deviation from the plan: five steps became three commits

**Planned:** five commits, one per step.
**Actual:** three.
**Why:** `just lint` runs `cargo clippy -- -D warnings`. `plan.md` step 1
flagged this risk and pre-authorized the remedy ("check the workspace lint level
first and fold steps 1 and 4 if needed, rather than leaving an `#[allow]` in the
tree"). Confirmed `-D warnings` in `justfile:79-82`, so:

- Step 1's `logged_transitions` field folded into step 2 (its only consumer).
- Step 1's `DeclineReason` / `SchedulingPass` types folded into step 4, and
  step 5 folded in with them: step 4 *writes* the pass record and step 5 *reads*
  it, so committing them apart would have left every field of `SchedulingPass`
  written-but-never-read — `dead_code`, which `-D warnings` rejects.

The verification boundary the plan actually cared about is intact: steps 2 and 3
remain separate commits, so "how many events are emitted" and "how many lines
they render as" were each verified against the full suite independently.

## Deviation: two extra tests beyond the plan

- `a_repeated_transition_after_other_transitions_is_announced_again` — pins the
  reason `logged_transitions` stores the *previous* transition rather than a
  set. Without it, nothing would catch a future "simplification" to a `HashSet`
  that silently swallows a reset-and-rerun ticket's real second pass through
  Research → Design.
- `only_phase_completed_projects_a_transition_line` — asserts the projection
  invariant directly, rather than only through its consequences.

`reset_ticket_clears_the_transition_memory` from the plan was **not** written as
a standalone test; the `remove` call it would have pinned is exercised by
`a_repeated_transition_...` at the level that matters (the memory is
last-transition-only, so a stale entry cannot outlive one intervening
transition). Declared as a gap in `review.md` rather than quietly dropped.

## Deviation: the pass record covers four more arms than planned

`structure.md` §6 named six admission arms plus two failure arms. Re-grepping
`continue` inside `schedule_ready_tickets` (as step 4's mitigation required)
found **four** launch-failure arms, not two: assignment preparation, and three
separate `prepare_fresh_launch` sites (recycle, fresh-exec reuse, cold launch).
All four now record `SpawnFailed` alongside the `Error` they already logged, so
the dump's account of a pass is complete rather than nearly complete.

---

## Evidence for acceptance criterion 3

> After demotion the state dump still answers "why didn't X spawn?" through its
> own section or debug ring, not the activity ring — shown in progress notes
> (P2: the fact survives; only the feed stops carrying it).

`declined_spawn_survives_in_the_state_dump` asserts all three halves of this in
one fixture: the record exists, the dump renders it, and the feed does not carry
it. The section it renders:

```
=== Last Scheduling Pass ===
at:        1784510049 (unix epoch, 0s ago)
ready:     1
spawned:   (none)
declined:  1
  T-001          thread already running
```

Before any pass has run, the same section reads `(no scheduling pass has run)`
rather than inventing a record — also asserted.

The fact survives in a *better* form than it was demoted from. The old Info
answered "why didn't X spawn?" for exactly one of ten decline arms; the other
nine either logged nothing at all (the two caps, no-slot, pane-awaiting,
durable-Done mask) or logged an error without saying it was a scheduling
outcome. The dump now answers for all ten.

---

## What one transition costs now

Measured by `feed_phase_lines`, which counts through the real projection:

| Path | Feed lines |
|---|---|
| Artifact detector, Research → Design | 1 — `T-001 completed Research` |
| Idle detector, Research → Design | 1 — `T-001 completed Research` |
| Both detectors on the same transition | 1 |
| Two tickets completing Design | 2, distinct |
| Scheduling pass, all candidates live | 0 |

## Verification

- `just check` — exit code **0** (fmt, clippy `-D warnings` on all three
  crates, `cargo check` against `wasm32-wasip1`, `cargo test --workspace`).
  463 tests pass, up from 455 at the start of Implement.
- Judged by exit code, not by reading output.
- One transient failure was observed and chased down rather than accepted:
  `lisa-cli`'s `bounded_runner_returns_valid_proposal_and_surfaces_failure`
  failed once during a `cargo test --workspace` run that overlapped a
  compilation. It spawns a shell script under a 2-second deadline, and it passes
  in isolation and in every subsequent full-gate run. It lives in a crate this
  ticket does not touch. Recorded here as load flakiness, not as a pass.

## Not done, deliberately

- No change to `ui.rs`. The feed sentence is unchanged, which was the point —
  E-052's "Done looks like" quotes `"T-016-01-01 completed Design — 3m ago"` as
  the target, so the work was to make that sentence appear once, not to rewrite
  it.
- `AllTicketsDone` still projects to `"all completed Done"`. Odd copy, one line,
  one real fact, no acceptance criterion reaches it.
- The `rebuild_dag` reconciler still emits its bare `TicketPhaseChanged`. It now
  costs zero feed lines, which was its entire contribution to the defect. Design
  Part B records why it was not routed through the new choke point.
