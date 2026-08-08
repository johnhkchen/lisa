# Review — T-057-02-02 init-retires-what-it-once-wrote

`lisa init` can now retire three things an older Lisa left behind: the workflow document under its
old name, the `CLAUDE.md`/`AGENTS.md` Lisa used to generate, and the dead `scheduling.auto_advance`
key. All three are driven off one shared detector rather than three sets of conditionals, and all
three appear in `--dry-run` before anything happens.

## What changed

| File | Change |
| --- | --- |
| `crates/lisa-cli/src/currency.rs` | New `retirements(root)` — the shared, `init`-free detector — plus `Retirement`, `RetirementKind`, `Disposition`. `inventory` now maps over it instead of re-reading the filesystem four times; the four `retired_*`/`stale_*` helpers collapse into one `retirement_findings`. |
| `crates/lisa-cli/src/init.rs` | `plan_retired_template` deleted. New `InitAction::RetireConfigKey`. New `plan_retirements` maps dispositions to plan lines. `plan_init_actions` calls `currency::retirements` once and appends the retirement group at the end of the plan. |
| `crates/lisa-cli/src/config.rs` | New `remove_retired_scheduling_key` / `RetiredKeyRemoval` — line surgery with a parse-equivalence post-condition, and a refusal when it cannot be done cleanly. |
| `crates/lisa-cli/src/legacy_context.rs` | New `bears_lisa_claude_marks` / `bears_lisa_agents_marks` — the weak, report-only signal. |
| `README.md` | Three stale passages: two still claimed `lisa init` writes your `CLAUDE.md`/`AGENTS.md` (untrue since T-057-01-03), and the CLI reference did not mention removal at all. |

Four commits, each `just check`-green:

- `ae45c80` the mechanism (the four source files)
- `a4b7440` the acceptance tests
- `021a4b1` the README
- `2ff32ba` the closing end-to-end assertion

## The decisions worth a reviewer's attention

**The seam.** `plan_init_actions → currency::retirements`, and `currency::inventory →
plan_init_actions` *and* `→ retirements`. The literal reading of "use the inventory" —
`plan_init_actions` calling `inventory` — recurses forever, because `inventory` reads init's plan to
decide every remedy. Splitting detection out is the only shape that satisfies "does not re-derive
staleness on its own". The module reference is cyclic; the call graph is not.

**The mixed case.** `AGENTS.md` is decided first and `CLAUDE.md` reads that decision, so file order
decides nothing. The rule: **a pointer target is retired only when nothing is left pointing at it.**
`CLAUDE.md` goes when `AGENTS.md` is absent, does not contain the string `CLAUDE.md`, or is itself
being removed in the same plan; otherwise it is preserved with the reason naming the pointer. The
pointer test runs against the real file, not against Lisa's marks — a hand-written `AGENTS.md` that
names `CLAUDE.md` dangles exactly as badly as a generated one. Five cases are asserted on disk after
a real run, including the invariant checked directly: no run leaves an `AGENTS.md` mentioning a
`CLAUDE.md` that is gone.

**The weak signal stops at init's preview — a narrowing beyond what the design spelled out.** An
edited `CLAUDE.md` that still bears a frozen preamble gets an init `skip` line and *no* `lisa doctor`
finding. Doctor's contract is that every finding names a command, and the command for a
retired-but-preserved file is `lisa clean` (T-057-02-03) — one that deletes. As a preview line a
false positive costs one sentence; as a doctor finding it would put somebody's own writing two
keystrokes from deletion. Design §2 says the weak signal "never authorizes removal"; keeping it out
of the inventory makes that true of the whole system rather than only of init. Asserted in
`currency::an_edited_context_file_is_reported_to_init_and_never_to_doctor`.

**The 0.2 `CLAUDE.md` preamble is not evidence.** It is 25 bytes — `# CLAUDE.md`, a blank line,
`## Project` — which is also exactly what a person writing the file by hand produces. It is excluded
from the weak signal, so an edited 0.2 generation is silently left alone. The strict matcher still
recognises an *unedited* one, which is the only signal that removes anything. Found by a test
failure, not by review.

**Init never removes the config key on its own judgment.** The `.lisa.toml` rewrite calls
`remove_retired_scheduling_key` only when the retirement already authorized it, so "a key disappeared
with no preview line naming it" is impossible by construction rather than merely unlikely. The
reverse — a line whose key survives — is reported as a `skip`, not silently.

## Test coverage

Ten new tests in `init.rs`, three in `currency.rs`, five in `config.rs`, two in `legacy_context.rs`.
Every acceptance criterion maps to one:

| Criterion | Test |
| --- | --- |
| Retirements planned from the inventory, distinct action, no re-derivation | `plan_init_actions` calls `currency::retirements`; `InitAction::RetireConfigKey`; the old `plan_retired_template` is gone |
| Generated context file removed, edited one preserved and reported — both files | `init::a_generated_context_file_goes_and_an_edited_one_stays` |
| Mixed case, no dangling pointer | `init::a_pointer_target_is_retired_only_when_nothing_points_at_it` (5 cases) |
| `auto_advance` removed, comments/keys/order exact | `init::the_dead_config_key_goes_and_every_other_byte_stays`, `config::removing_the_retired_key_keeps_every_other_byte` |
| Unremovable `.lisa.toml` left and reported | `init::a_config_that_cannot_be_edited_surgically_is_left_alone_and_reported`, `config::a_shape_no_line_deletion_can_express_is_refused` |
| Retired-phase tickets reported, not rewritten | `init::a_ticket_at_a_retired_phase_is_reported_and_never_rewritten` |
| `--dry-run` names every retirement, changes nothing | `init::dry_run_names_every_retirement_and_changes_nothing` (recursive byte snapshot) |
| End to end through one `lisa init` | `init::one_init_brings_a_0_4_4_project_current` |
| Already-current project is all `NoOp` | `init::init_on_an_already_current_project_is_all_no_op`, `init::a_second_consecutive_run_changes_nothing` |
| `just check` green | exit code 0, verified |

Also smoke-tested with the real binary against a hand-built 0.4.4 fixture: the preview lists the
three removals and the preserved ticket, each with its reason; one run removes both context files
and the key; a second run reports `Files changed: none` with the `.lisa.toml` hash unchanged; and
`lisa doctor` afterward reports one finding — the board row, with an operator remedy.

## Open concerns

**1. A pre-existing `upsert_missing_config_keys` bug, found by this ticket's smoke test and
deliberately not fixed here.** When `[scheduling]` is the *last* section of a `.lisa.toml`, every
commented stub init appends is written twice. Cause: `insert_after` pushes `new_lines` both inside
its loop (`i + 1 == after_line`) and again in its `after_line >= lines.len()` tail — and when the
section runs to end of file those two conditions are the same condition. Reproduced at `54077f2`,
the commit before this ticket, on a config with no `auto_advance` in it at all, so it is unrelated
to this work. It loses nothing an operator wrote and converges after one run (the duplicate stub
then counts as "present"), which is why it has gone unnoticed. Worth its own small ticket; the fix
is one `else` in `insert_after`.

**2. The removed key's comment stays behind.** `# Left over from 0.4 — nobody remembers turning it
on.` survives the line it described, and can end up sitting above unrelated stubs. This is the
criterion working as written — "every comment ... preserved exactly" — and the alternative (guessing
which comment lines belong to which key) is exactly the kind of judgment about somebody's file that
this ticket says not to make. Recorded because it is visible in real output, not because it is wrong.

**3. The end-to-end criterion's residue is a board row, not `lisa clean`'s.** The criterion asks that
`lisa doctor` afterward report the project "current apart from what T-057-02-03 owns". On a fixture
carrying a ticket at `phase: structure`, what one `lisa init` leaves is that ticket row — which
T-057-02-03 does not own either, because `lisa clean` never touches the board. The same ticket
mandates this ("Report them; leave them"), so the two sentences pull against each other. The test
asserts the strong form both ways: no `Behind` and no `Retired` finding survives one init, every
remaining finding carries an `Operator` remedy, and once the board row is settled the way its owner
would settle it, `inventory().is_current()` is true outright.

**4. A flaky test in the gate, untouched by this ticket.**
`triage_agent::bounded_runner_returns_valid_proposal_and_surfaces_failure` spawns a script under a
2-second deadline and timed out once under parallel load, then passed on rerun; the final `just
check` was green (exit 0). `git diff` confirms this ticket changes no byte of `triage_agent.rs`. The
ten new filesystem tests add load, so it may surface a little more often. Its sibling test carries a
comment about having been de-flaked for exactly this reason; this one still has a hard wall-clock
budget.

## Deviation from the plan

The plan called for six independently green commits. Commits 1–3 could not be green on their own:
`cargo clippy -D warnings` fails on `dead_code` for a `pub(crate)` helper nothing consumes yet, and
the config helper, the marks and the detector each have their only consumer in the commit after.
They shipped as one mechanism commit instead. The test and docs commits are unchanged from the plan.
