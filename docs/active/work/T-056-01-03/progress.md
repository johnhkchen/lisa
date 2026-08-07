# T-056-01-03 Progress

All eight plan steps executed. `just check` green by exit code (0), full log in the attempt
scratch. Six commits through `lisa commit-ticket`, no ticket-owned file left staged, modified, or
untracked.

## Commits

| Commit | Step | What landed |
| --- | --- | --- |
| `4155a4c` | 1 | Per-check time budget in the disposition schema |
| `4a893d5` | 2, 3, 5 | One shared check runner with its own budget; world non-passes recorded |
| `cc5f908` | 4 | `check-disposition` runs the recorded check |
| `140ac6f` | 6 | `lisa status` names a world remedy that keeps failing |
| `65eecf8` | 7 | The execution contract, written down and pinned to the code |
| `658aa0f` | 8 | End-to-end black-box tests |

## Step by step

**1 — Budget in the schema.** `DEFAULT_CHECK_BUDGET_SECS = 5`, `MAX_CHECK_BUDGET_SECS = 1800`, and
`resolve_check_budget_secs` in `lisa-core::disposition`; `check_timeout_secs` on
`ReviewDisposition::Block` and `ParkedRemedy`. Strict authoring path refuses a non-integer, zero,
over-cap, or check-less budget with a named fix; tolerant path clamps to the cap and degrades a
malformed budget to the operator-owned fallback. Six unit tests.

*Deviation from plan:* the new field also required updating `ReviewDisposition::Block` literals in
`lisa-core/src/completion.rs`, `lisa-core/tests/completion_state_machine.rs`, and the `ParkedRemedy`
literals in `lisa-cli/src/status.rs` — all enumerated in Structure §7, all mechanical.
`lisa-plugin`'s literals turned out to use `..`, so none needed changing.

**2 — Runner extracted.** New `crates/lisa-cli/src/check_run.rs` holds `CheckResult`, `CheckRun`,
`run_check`, `budget_for`, `format_budget`, and the capture/sanitize helpers. `CheckRun` gained
`budget`. Six runner-level tests moved with the code; three new ones added (`format_budget` units,
declared budget enforced, the writing-check contract).

**3 — Both callers resolve the declared budget.** `CHECK_TIMEOUT` is gone. `decline_timed_out` and
`exit_code_line` format from `run.budget`; `decline_header` takes the run rather than the result so
the timeout arm can reach it.

**4 — `check-disposition` runs the check.** `Passed`/`Failed` accepted (a remedy nobody has
performed yet is the ordinary state of a block, and the success line now says the check ran and what
it reported); `Inconclusive`/`TimedOut` refused with the command, directory, exit code, first output
lines, and a fix addressed to the reviewer.

**5 — World non-passes recorded.** `WorldRecheckRecord` / `WorldRecheckOutcome` /
`latest_world_rechecks` in `lisa-core::provenance`; `record_world_non_pass` writes on a doubling
schedule keyed on `(ticket_id, check)`. Automation policy unchanged — a non-pass still never
reopens, never retries, never escalates.

**6 — Visible in `lisa status`.** At the eighth recorded non-pass a world remedy gains two lines
naming the count and the override command. Below it, and for an operator-owned remedy, rendering is
byte-identical to before.

**7 — The contract written down.** `docs/knowledge/rdspi-workflow.md` and
`crates/lisa-cli/data/rdspi-workflow.md` (identical, byte-for-byte enforced by the existing
`test_rdspi_workflow_embedded`) now state where a check runs, what it sees, that it must only look,
its budget, what its exit codes mean, and that `check-disposition` runs it. The new
`the_documented_check_contract_matches_the_code_that_enforces_it` asserts the documented default and
cap through `format!` against the constants.

**8 — End to end.** The story fixture with all three ceilings at once (git repository ignoring
`out/`, real `out/marker`, a check that sleeps past the default budget under a declared 60-second
one) unblocks cleanly: exit 0, ticket open, ready in the DAG, no override receipt.

## Deviations from the plan

1. **The field-case fixture for `check-disposition` changed shape.** The plan used
   `node scripts/check-touch.mjs` expecting exit 127. On a machine with `node` installed that exits
   `1` (module not found), which classifies as `Failed` — a verdict, correctly accepted. The test
   now drives both real shapes of "could not look": `./scripts/check-touch.mjs` (127, not there) and
   the field script's own `exit 2`.
2. **`world_owned_failing_check_stays_parked_without_churn` was not revised.** The plan expected to
   add a ledger assertion to it. Its existing assertions all remain true, and the new
   `a_world_remedy_that_never_clears_becomes_a_durable_record` covers the record properly across
   three invocations; editing the older test would have duplicated it.
3. **One extra test beyond the plan** — `a_world_recheck_that_passes_writes_no_non_pass_record` —
   because "records non-passes" is only half a claim without "and records nothing otherwise".

## Not done, deliberately

The plugin dashboard card (`ui.rs`, "Lisa checks on its own") does not show the stuck count. Carried
into Review as a known limit; the design records why.
