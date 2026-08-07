# T-056-01-01 — Review: say-what-was-run-and-where

## What changed

Six files, no files created or deleted. Four commits, all through `lisa commit-ticket`.

| File | Change |
| --- | --- |
| `crates/lisa-core/src/provenance.rs` | New `CheckOverrideRecord` row (`record_type: "check-override"`), its outcome enum, its writer, and the `ProvenanceLedgerRecord::CheckOverride` arm. `SCHEMA_VERSION` 9 → 10. |
| `crates/lisa-cli/src/unblock.rs` | `CheckResult` gains `Inconclusive` and loses its payload; new `CheckRun` carries the check string, directory, exit code, and both streams out of `run_check`; `decline_message` → `decline_report`; `observed_line` → `observed_lines`; `run_unblock` takes `override_check` and files a receipt. |
| `crates/lisa-cli/src/main.rs` | `lisa unblock --override-check`, passed through to `run_unblock`. |
| `crates/lisa-cli/tests/parked_ux.rs` | Two rewritten tests, six new ones, plus ledger-reading helpers. |
| `crates/lisa-cli/tests/help_surface.rs` | The `unblock --help` snapshot. |
| `docs/knowledge/flag-audit.md`, `README.md` | The flag's audit row and its operator documentation. |

## The three things the ticket asked for

**1. Attribute and show the work.** Every fact was already inside `run_check` and was thrown away
at the return — that was the structural reason no rewording could have fixed this. `CheckRun`
carries them out. A check-caused decline now leads with Lisa's own sentence, then shows `what
ran:`, `ran in:`, `exit code:`, then each stream under its own `the check wrote to stderr/stdout:`
label. `ran in:` is the value handed to `.current_dir`, carried rather than recomputed, so it
stays true when T-056-01-02 moves it.

**2. "Could not look" is not "did not pass."** New `CheckResult::Inconclusive` for exit 2, 126,
127, or death by signal — 2 is the field script's own convention and the long-standing "trouble,
not a verdict" code; 126/127 are what `/bin/sh -c` itself returns when the recorded command could
not be started. Its sentence says plainly that this is not a judgement on the operator's work, and
every decline ends by naming `--override-check`. The `TimedOut` and `ChangedFiles` sentences are
byte-identical to before and still render distinctly (the timeout one is now formatted from
`CHECK_TIMEOUT` so it cannot drift from the constant).

**3. A way through that always exists.** `--override-check` runs the check, reports it in full,
then reopens anyway and files a `check-override` provenance row naming the operator, the check,
the directory, the exit code, and what it printed. The receipt is written *before* the status
flip, so Lisa can never reach a reopened ticket with no record of why. The flag covers exactly the
check gate: the unknown-ticket, not-waiting, missing-remedy, and `already-done` declines are
untouched by it.

## Test coverage

| Criterion | Pinned by |
| --- | --- |
| 1 — command, cwd, exit code, both streams attributed | `a_declined_check_reports_the_command_the_directory_the_code_and_both_streams` (black box, exit 3, output on both streams, compares the reported directory with the one the check itself printed); unit `passing_and_failing_checks_carry_the_command_directory_and_code`, `the_reported_directory_is_the_one_the_check_observed` |
| 2 — distinct inconclusive variant and wording; `TimedOut`/`ChangedFiles` still distinct | unit `exit_two_and_shell_failures_are_inconclusive_not_a_verdict`, `every_decline_header_is_distinct_and_names_the_way_through`, `timeout_is_bounded_and_kills_the_shell_group`, `mutation_inside_disposable_state_is_detected_even_after_chmod` |
| 3 — documented override flag; with it `Reopened`/exit 0, without it unchanged | `override_check_reopens_and_leaves_a_record` (both halves), `override_check_does_not_bypass_the_non_check_declines`, `operator_help_matches_snapshots`, README |
| 4 — durable record naming the override and the actor; none for an ordinary unblock | `override_check_reopens_and_leaves_a_record`, `override_check_records_nothing_when_the_check_passes`; wire format by `check_override_record_round_trips_through_the_mixed_ledger` and `check_override_row_does_not_absorb_or_get_absorbed` |
| 5 — sanitization covers everything shown | `escape_sequences_and_tabs_are_stripped_from_everything_shown` (ANSI + tabs on both streams), unit `observed_lines_strip_controls_fold_tabs_and_cap_length_and_count` |
| 6 — the field line reproduced then improved | `a_check_that_cannot_look_reads_as_inconclusive_not_as_a_verdict`, unit `the_field_line_is_reported_not_asserted_as_lisas_verdict`; both assert the 0.4.4 output `That didn't work yet — No build at dist/` cannot come back |
| 7 — `just check` green | run to exit 0 (fmt, clippy, WASM check, workspace tests) |

Counts: 9 unit tests in `unblock.rs` (was 5), 20 black-box tests in `parked_ux.rs` (was 14), 3 new
in `provenance.rs`. `just check` exits 0.

## Judgement calls a reviewer should look at

**A decline is no longer one line.** `parked_ux.rs` asserted `stderr.lines().count() == 1`. The
ticket requires four facts plus captured output, which cannot honestly be one line, so that
assertion was a test of the old rendering rather than of a property being preserved. It is now a
structural assertion. Exit code and the absence of an `Error:` prefix are unchanged.

**A new display cap.** `MAX_OBSERVED_LINES = 10` per stream, with a `… (N more lines)` tail.
`MAX_CAPTURE_BYTES` and `MAX_OBSERVATION_CHARS` are untouched, as the ticket requires; this only
ever shows *less* than what was captured, because an 8 KiB capture printed whole is a wall.

**Captures are now read on the timeout and changed-files paths too.** Those two used to return
before reading, so they could not show evidence even in principle. The process group is SIGKILLed
before the read on the timeout path.

**Where the classification line falls is a decision, not a discovery.** Exit 2/126/127 and signal
death are inconclusive; every other non-zero is a failure. A check that means "could not look" but
exits 1 will still read as a verdict. The ticket left the line to design and named exit 2; letting
the disposition declare its own inconclusive code would be a change to the check contract, which
belongs to T-056-01-02/03.

**A failed ledger append declines the unblock.** An override that leaves no trace is exactly the
state criterion 4 forbids. This is not a gate with no way through — its only cause is an
unwritable `.lisa/`, which stops every other Lisa command equally, and the message names it.

## Open concerns

1. **`ran in:` currently names a temp directory, and that is the truth.** It reports the snapshot
   path because that is where checks run today. This ticket deliberately does not change it
   (T-056-01-02 owns that), and the reporting is written so it needs no edit when that lands.
2. **`SCHEMA_VERSION` 9 → 10 is a shared bump.** Every new row across the workspace now stamps 10.
   Additive and backwards-compatible: legacy-parse tests for schema 2, 3, and 4 still pass
   untouched, and `status.rs`'s schema-9 fixtures still parse.
3. **Every decline names the override, including a genuine failure.** Deliberate — N2 says a named
   state always has an action that works, and the header still carries the verdict — but it does
   put the escape hatch in front of an operator whose check honestly failed. The receipt is what
   keeps that honest.
4. **The `--override-check` copy uses "check".** That is the word the disposition schema and the
   reviewer's own field use, so it is the word the operator will have read in `lisa status`. It
   passes the banned-jargon gate.

## Not in this ticket

Where the check runs, the 5-second budget, whether a check may write, record-time validation of
checks, and `run_world_rechecks`'s silence — T-056-01-02 and T-056-01-03.
`docs/knowledge/rdspi-workflow.md` was read and left alone; it is T-056-01-03's criterion.

## Environment

`wasm32-wasip1` was missing on this machine, which failed `just check`'s WASM leg and three
`client_autodetect` tests on an empty embedded-WASM placeholder — both pre-existing and unrelated.
Installing the target and building `lisa-plugin` fixed both before any of this work was judged. No
repository file changed for it.
