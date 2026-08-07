# T-056-01-03 Design — no-check-that-cannot-pass

Four decisions, one theme: the constraints on a check become visible while the reviewer who writes
it is still on the ticket, instead of in the operator's terminal weeks later.

---

## D1 — The time budget

### Options

**A. Raise the constant.** `CHECK_TIMEOUT = 30 min`. One line. Rejected by the ticket itself, and
correctly: the next check will be longer, and it makes every trivial check able to hang the
scheduler's recheck for half an hour.

**B. Project-level config key** (`config.toml`, alongside `session_timeout_secs`). Rejected: the
budget is a property of *one check*, not of the project. The 2130-page sweep and `test -f out/marker`
live on the same board. A project-wide value must be sized for the slowest check, which is option A
with extra steps.

**C. Per-check declared budget, capped.** ← chosen. The disposition that records the check records
its budget: `"check_timeout_secs": 1500`. Absent means the current default. A documented cap bounds
it, and exceeding the cap is refused *at record time*.

### Decision

- `DEFAULT_CHECK_BUDGET_SECS = 5` — unchanged. The default is right for the overwhelming majority
  of checks (`test -f`, `curl -fsS`), and every existing disposition keeps its current behaviour.
- `MAX_CHECK_BUDGET_SECS = 1800` (30 minutes). Sized from the field case: a ~20-minute headless
  sweep is a real verification and must fit, with headroom, while still bounding how long the
  scheduler's every-5-second recheck can have one child alive. Both constants live in
  `lisa-core::disposition`, next to the schema that carries the field, so the doc, the strict
  authoring check, the parser, and both call sites read one source.
- Strict authoring check (`check-disposition`): a non-integer, zero, or over-cap budget is refused
  with a named fix; a budget without a `check` is refused. This is the criterion-1 rejection —
  *when it is recorded, not when it is run*.
- Tolerant parser (`parse_review_disposition`): clamps a declared budget to the cap rather than
  honouring it. It reads hand-edited and historical files, and a number in a file may never buy
  unbounded child-process time. A malformed budget degrades the block the same way a malformed
  `ask` does — to the operator-owned unstructured fallback — because that is the established
  fail-safe and it drops the check entirely rather than running it wrong.

### Rejected sub-option: a budget on `lisa unblock`

`--timeout 1200` on the command line was considered and rejected. It puts the knowledge in the
operator's hands — exactly the person the story says cannot fix this — and leaves the automated
`recheck-world` path, which has no operator, still at five seconds.

---

## D2 — The expiry sentence

`CheckRun` gains `budget: Duration`, carried out of `run_check` the same way and for the same
reason `directory` already is: the report names what actually happened rather than recomputing it
from a constant that may not have been the one in force.

`decline_timed_out(budget)` and `exit_code_line` format from that field through one
`format_budget` helper: `"5 seconds"`, `"25 minutes"`, `"20 minutes 30 seconds"`. Plain units, no
jargon; a reviewer who declared 1500 seconds reads "25 minutes", which is the number they were
thinking in when they wrote it. The existing pinned sentence for the default budget is unchanged
byte for byte, so criterion 2's "not the literal string" is satisfied by construction — the string
was already derived; what changes is that it is derived from *this run's* budget.

---

## D3 — May a check write?

### The decision: **no — and Lisa says so where checks are written, without pretending to enforce it.**

T-056-01-02 already removed the fingerprint, and its reasoning is load-bearing here rather than
merely historical: checks run in the live tree while agent sessions edit the same files, so a
before/after comparison attributes other writers' changes to the check. Re-adding detection would
manufacture exactly the false verdict this story exists to remove. The options were therefore:

**A. Sandbox the tree again** (copy-on-write / overlay). Rejected: it is how the field failure
happened. Any copy is a tree the operator is not standing in, and the story's whole finding is that
a check must see the project the operator sees.

**B. Detect and refuse writes.** Rejected: cannot be done correctly under concurrency, per above.

**C. Contract, stated where checks are authored, unenforced by the runner.** ← chosen. The workflow
document states plainly that a check must only look, that Lisa runs it in the live project and
cannot stop it writing, and that `npm run build && npm run verify` is therefore not a check —
record the verifying half alone. The runner's behaviour is the documented one: a writing check is
run like any other and judged only by its exit code, and its writes land in the project.

This is the honest shape. The alternative — a rule Lisa claims to enforce and enforces wrongly — is
what a reviewer is entitled to trust and what would fail them.

### Why not a heuristic in `check-disposition` (reject checks containing `build`)?

Considered, rejected. `npm run build` in a check is bad; `test -f build/index.html` is fine, and
`cargo build --dry-run`-shaped checks exist. A string heuristic here would produce false refusals
in the one command whose entire purpose is to stop false refusals.

---

## D4 — `check-disposition` runs the check

The highest-leverage part, per the ticket. Design questions and answers:

**Which outcomes are "cannot pass"?** A block's check is *expected to fail* at record time — the
remedy has not been performed yet. So:

| Outcome | At record time | Why |
| --- | --- | --- |
| `Passed` (0) | accept | the remedy is already true; unblock will clear it |
| `Failed` (1, 3, …) | accept, unchanged | the ordinary state of a remedy not yet done |
| `Inconclusive` (2, 126, 127, signal) | **refuse** | the check never reached its question — this is the field case, byte for byte |
| `TimedOut` | **refuse** | it cannot pass under the budget it declared |
| budget over the cap | **refuse** | shape, caught before anything runs |

**What does the refusal say?** The same facts the operator-facing decline shows, in the reviewer's
idiom: what ran, where, the exit code, and the first lines the check printed — prefixed by the
existing `Fix review-disposition.json:` lead so it lands in the same place as every other authoring
fix. The reviewer is the one person who can rewrite the check, and they are still on the ticket.

**Where does the runner live?** `run_check` and its result types move from `unblock.rs` into a new
`crates/lisa-cli/src/check_run.rs`. It is now shared by three callers (unblock, world recheck,
check-disposition) and it *is* the execution contract — the thing the document describes — so it
gets its own module rather than being reached into from a sibling. Decline rendering stays in
`unblock.rs`, because that is operator-facing copy, not contract.

**Cost.** `check-disposition` now costs what the check costs, up to the declared budget. That is
the point: it is the difference between a bad check costing one review turn and costing an
afternoon. A pass or note disposition, and a block with no check, are unaffected.

---

## D5 — The world recheck stops discarding non-passes

### The constraint that decides this

`recheck-world` fires on the plugin's 5-second poll for as long as a world remedy stays parked. A
ledger row per non-pass is 720 rows an hour, forever. "Surface it" cannot mean "log every one".

### Options

**A. Row per non-pass.** Rejected — unbounded, and the signal drowns in it.

**B. Row only when the observation changes.** Rejected as the primary rule: a check that prints a
timestamp changes every time (unbounded again), and one whose output never changes writes exactly
one row, which shows *that* it failed but never that it keeps failing.

**C. Row on a doubling schedule.** ← chosen. Record the 1st, 2nd, 4th, 8th, 16th … consecutive
non-pass for a remedy. Bounded (logarithmic in poll count: ~10 rows an hour, ~15 a day), and the row
*count itself* renders the repetition. Each row carries the running total at the moment it was
written, so a reader never has to count rows to know how bad it is.

### The record

A new `WorldRecheckRecord` in the provenance ledger, shaped like `CheckOverrideRecord` and for the
same reason (no live lease to attribute it to): `ticket_id`, `check`, `directory`,
`result: WorldRecheckOutcome` (`failed` | `inconclusive` | `timed-out`), `exit_code`, `observed`,
`non_pass_count`, `occurred_at`. A separate outcome enum rather than reusing
`CheckOverrideOutcome`: the sets genuinely differ — no `changed-files` here — and the absent
`passed` variant states in the type that a pass is never recorded this way (a pass reopens the
ticket, which is already durable and already visible).

Counting is keyed on `(ticket_id, check)`: a disposition rewritten with a different check starts a
fresh count, because it is a different claim about the world.

### The named actionable state (N2)

The ledger is durable but not *visible*. `lisa status` gains one thing: when a world remedy has
reached the eighth recorded non-pass, its Waiting-on-you entry says so and names the way through —

```
T-WORLD  Wait for the release link. — Lisa checks on its own.
       Lisa has checked at least 8 times and it still isn't passing.
       If you have checked this yourself, run: lisa unblock T-WORLD --override-check
       Reviewer's note: release absent
```

"at least" is exact, not hedging: recording is sampled, so the last recorded total is a floor.
Below the threshold nothing changes — a world remedy failing twice in ten seconds is the ordinary
state of waiting for the world, not a fault.

**Automation policy is untouched**, per the story's out-of-slice list: a non-pass still never
reopens anything, still never retries differently, still never escalates by itself. Only the
silence changes.

### Deliberately out

The plugin dashboard card (`ui.rs:685`, "Lisa checks on its own") is left as it is. It is a live
view an operator watches during a run; `lisa status` is where they go to read what happened, and
widening the WASM card's data flow is a larger change than this criterion asks for. Recorded in
Review as a known limit.

---

## D6 — What is *not* changed

- The default budget stays 5 seconds. Every disposition already written keeps its behaviour.
- `CheckOverrideOutcome::ChangedFiles` stays on the wire (old ledgers must keep parsing).
- `lisa validate` is not given a second execution path; one command runs checks, for one purpose.
- The `already-done` route, the ask floor, the note shape, and the parking schema beyond the one
  new field are untouched.
