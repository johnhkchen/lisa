# T-056-01-03 Review — no-check-that-cannot-pass

Three ceilings behind the fixed working directory are now visible at authoring time instead of at
unblock time, and the automation that used to discard non-passes now records them.

## What changed

### `crates/lisa-core/src/disposition.rs` (modified)
`DEFAULT_CHECK_BUDGET_SECS = 5`, `MAX_CHECK_BUDGET_SECS = 1800`, `resolve_check_budget_secs`, and a
`check_timeout_secs: Option<u64>` field on `ReviewDisposition::Block`. The strict authoring path
refuses a non-integer, zero, over-cap, or check-less budget with a named fix. The tolerant path
clamps to the cap — it reads hand-edited and historical files, where a number may not buy unbounded
child-process time — and degrades a malformed budget to the existing operator-owned fallback.

### `crates/lisa-core/src/parking.rs` (modified)
`ParkedRemedy` carries the declared budget through to both callers.

### `crates/lisa-core/src/provenance.rs` (modified)
`WorldRecheckType`, `WorldRecheckOutcome` (`failed` | `inconclusive` | `timed-out` — no `passed`,
because a pass reopens the ticket instead), `WorldRecheckRecord`, `append_world_recheck_record`, the
`ProvenanceLedgerRecord::WorldRecheck` arm, and the `latest_world_rechecks` projection.

### `crates/lisa-cli/src/check_run.rs` (new, 470 lines with tests)
The execution contract, moved out of `unblock.rs` because three callers now share it. `run_check`
is unchanged in behaviour except that `CheckRun` carries the `budget` it was held to. Adds
`budget_for` and `format_budget`.

### `crates/lisa-cli/src/unblock.rs` (modified)
`CHECK_TIMEOUT` deleted; both callers resolve the remedy's declared budget. The timeout decline and
the exit-code line format from `run.budget`. `run_world_rechecks` calls the new
`record_world_non_pass`, which writes a ledger row on a doubling schedule (the 1st, 2nd, 4th, 8th …
non-pass for a `(ticket, check)` pair).

### `crates/lisa-cli/src/check_disposition.rs` (modified)
A block's `check` is now run under the real contract before the command succeeds. `Passed` and
`Failed` are accepted — a remedy nobody has performed yet is exactly why the ticket blocks — and the
success line names that the check ran and what it reported. `Inconclusive` and `TimedOut` are
refused with the command, directory, exit code, first output lines, and a fix written to the
reviewer rather than an override written to the operator.

### `crates/lisa-cli/src/status.rs` (modified)
At the eighth recorded non-pass, a world remedy's Waiting-on-you entry gains two lines: what Lisa
has seen, and the one command that ends the wait.

### `crates/lisa-cli/src/main.rs`, `templates.rs` (modified)
Module registration; the pinned block-shape literal gains `check_timeout_secs`; the new
doc-versus-code test.

### `docs/knowledge/rdspi-workflow.md` + `crates/lisa-cli/data/rdspi-workflow.md` (modified)
The execution contract, stated where checks are authored: where it runs, what it sees, that it must
only look, its budget and cap, what its exit codes mean, and that `check-disposition` runs it.

## Acceptance criteria

| # | Criterion | Evidence |
| --- | --- | --- |
| 1 | Declared budget, documented cap; three assertions | `a_slow_check_reading_a_gitignored_artifact_unblocks_under_a_declared_budget` (sleeps 6 s, passes under a declared 60); `a_check_that_outlives_its_declared_budget_names_that_budget`; `an_over_cap_budget_is_refused_without_running_the_check` (a sentinel `touch` never happens) |
| 2 | Expiry names its budget, two budgets | `timeout_expiry_names_the_budget_that_was_enforced` — 5 seconds and 25 minutes, plus "1 second" through the CLI |
| 3 | Writes decided, implemented, stated; test | `a_writing_check_is_judged_by_its_exit_code_alone`; the **Writes** bullet in the workflow document |
| 4 | The document states the contract; test matches doc to code | `the_documented_check_contract_matches_the_code_that_enforces_it` — default and cap formatted from the constants, not copied |
| 5 | `check-disposition` runs the check at record time | `a_check_that_can_never_run_is_refused_at_record_time` (127 and the field script's exit 2); `a_satisfiable_check_passes_whether_or_not_the_remedy_is_done` |
| 6 | World non-passes become a durable record | `a_world_remedy_that_never_clears_becomes_a_durable_record` (3 rechecks → 2 rows, counts 1 and 2, ticket still blocked); `a_world_recheck_that_passes_writes_no_non_pass_record`; `the_world_recheck_projection_keeps_the_latest_row_per_ticket` |
| 7 | End to end on the story fixture | Same test as criterion 1's first row: gitignored `out/marker` + a slow check + a declared budget, exit 0 |
| 8 | `just check` green | Exit code 0; 26 test suites ok, fmt, clippy, WASM check |

## Test coverage

Added: 6 unit tests in `disposition.rs`, 2 in `provenance.rs`, 5 in `check_run.rs` (plus 6 moved
from `unblock.rs`), 4 in `unblock.rs`, 2 in `status.rs`, 1 in `templates.rs`, 4 black-box in
`parked_ux.rs`, 4 black-box in `check_disposition_cli.rs`.

Two tests spend real wall-clock seconds: the end-to-end unblock (~6 s, it must genuinely outlive the
five-second default) and the pre-existing `automatic_recheck_timeout_is_bounded_and_cannot_reopen`
(4–8 s). Everything else uses millisecond budgets.

**Gaps I know about:**

- The doubling schedule is asserted at counts 1 and 2 through the CLI, and the schedule itself is
  asserted arithmetically (`world_non_passes_are_sampled_on_a_doubling_schedule`). Reaching count 8
  through the real command would take eight invocations for one more data point; the projection test
  covers the read side at count 4.
- `format_budget` is unit-tested at seven points but no test drives a *declared* budget with a
  minutes-and-seconds remainder end to end through the CLI.
- No test covers two world remedies accumulating counts independently in the same ledger; the
  projection test covers two tickets, but not two concurrent recording paths.

## Open concerns

1. **The plugin dashboard still says only "Lisa checks on its own."** `lisa status` names a stuck
   world remedy; the WASM card does not. An operator watching the dashboard during a run sees the
   old text until they run `lisa status`. Deliberate — widening the card's data flow is a larger
   change than criterion 6 asks for — and recorded in `design.md` under "Deliberately out".
2. **`check-disposition` now costs what the check costs.** Bounded by the declared budget and the
   30-minute cap, and only for a block that records a check. A reviewer who declares 1800 seconds
   for a genuinely slow check will wait for it at Review time. That is the trade the ticket asks
   for, stated plainly so it is not a surprise.
3. **The non-pass count does not reset when a ticket unparks and re-parks with the same check.** It
   is keyed on `(ticket_id, check)` and counts every non-pass in the ledger's history. A rewritten
   check starts fresh; an identical one continues. The status line says "at least N", which stays
   true either way, but a reader should know the count is lifetime-per-check rather than
   per-park.
4. **Writes are contract, not enforcement.** A check that runs `npm run build` will build, in the
   live tree, while other threads are working in it. The document says not to; nothing stops it.
   The alternative — detection — cannot distinguish this check's writes from a concurrent agent's,
   which is the same class of false verdict this story exists to remove. `design.md` D3 carries the
   full reasoning.
5. **Pre-existing warning, untouched:** `crates/lisa-core/src/completion_journal.rs:1339`
   (`unused_mut` in test code, from commit `432f3f9`). Not in this ticket's blast radius; `just
   check` is green with it.

## Nothing needs human attention before completion

Every criterion has a test, `just check` exits 0, and the two behaviour changes an operator can see
— the new status lines and the expiry sentence naming a declared budget — are both pinned by tests
against their exact copy.
