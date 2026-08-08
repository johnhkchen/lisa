# Plan — T-057-02-03 lisa-clean

## Order of work

| # | Step | Gate before moving on |
| --- | --- | --- |
| 1 | `crates/lisa-cli/src/clean.rs` — `CleanAction`, `plan_clean_actions`, `run_clean`, litter detection, `board_status`, `reachability`, rendering | compiles |
| 2 | `crates/lisa-cli/src/currency.rs` — `ConfigKey` + `Preserve` → `Remedy::Operator` | `cargo test -p lisa-cli currency` |
| 3 | `crates/lisa-cli/src/main.rs` — `mod clean`, `Commands::Clean`, match arm, `display_order` 7 with `proposal`→8 and `loop`→9 | `cargo build -p lisa-cli` |
| 4 | `docs/knowledge/flag-audit.md` — three rows | `flag_audit_covers_live_cli_config_and_prompts` |
| 5 | `crates/lisa-cli/tests/help_surface.rs` — counts, snapshots, doc wording | `cargo test -p lisa-cli --test help_surface` |
| 6 | **commit 1** — the mechanism | `just check` |
| 7 | Acceptance tests in `clean.rs` (the twelve rows of `structure.md`) | `cargo test -p lisa-cli clean` |
| 8 | **commit 2** — the tests | `just check` |
| 9 | `README.md` — `### lisa clean`, and the trio named beside `lisa init`'s "what it will not remove" | reads correctly |
| 10 | **commit 3** — the docs | `just check` |
| 11 | End-to-end test: fixture → `run_init` → `run_clean(remove)` → `is_current()` + byte-identical human files | that test |
| 12 | **commit 4** — the closing assertion | `just check` |
| 13 | Smoke test with the real binary against a hand-built 0.4.4 fixture in the scratchpad | plan, then removal, then `lisa doctor` |
| 14 | `review.md` + `review-disposition.json`, then `lisa check-disposition T-057-02-03` | clean report |

Steps 1–5 land as one commit for the reason `structure.md` records: `clippy -D warnings` fails on a
module nothing calls, `help_surface` fails the moment a command is registered without its snapshot,
and `flag_audit` fails the moment a flag exists without a row. They are one mechanism.

## Criterion → evidence

| Acceptance criterion | Where it is satisfied | Where it is proven |
| --- | --- | --- |
| Scope decision (a)/(b) recorded, with the litter rule as one sentence | `design.md` §1 | `review.md` — the ticket asks for it in the review artifact specifically |
| `lisa clean` exists, registered beside `init` and `doctor`, covered by `help_surface.rs` | `main.rs` `display_order = 7` | `help_surface::top_level_help_matches_snapshot`, `operator_help_matches_snapshots`, `all_nineteen_subcommands_resolve` |
| **A bare `lisa clean` changes nothing and prints the plan; a test asserts the tree is byte-identical** | `run_clean` returns between printing and executing | `a_bare_run_prints_the_plan_and_changes_not_one_byte` |
| Removal only under an explicit flag; every removed path named in the plan first | plan computed complete before any mutation | `every_removed_path_was_named_in_the_plan_first` |
| Refusal: nothing under `docs/active/tickets/` or `docs/active/stories/` | no class names those directories | `the_board_is_never_a_candidate` |
| Refusal: nothing outside paths Lisa created | the three-shape allowlist | `nothing_outside_lisas_own_directories_is_a_candidate` |
| Refusal: no work artifact for a ticket that is not `done` | `is_done` in the candidate predicate | `an_unfinished_tickets_notes_are_never_candidates` (open, review, and absent) |
| Refusal: no path reachable by symlink out of the project root | `reachability`, at plan time | `a_symlink_out_of_the_project_is_refused` |
| Fresh `lisa init` project reports nothing to remove | nothing done, nothing retired, nothing accumulated | `a_fresh_init_project_has_nothing_to_remove` |
| **End to end: 0.4.4 fixture → init → clean → doctor reports current, every human file byte-identical** | the sequence itself | `the_0_4_4_fixture_ends_current_and_every_human_file_survives` |
| `just check` green | — | exit code read directly, not grepped |

Two tests exist for properties the criteria imply rather than state, and both are cheap:
`every_finding_that_names_clean_is_a_removal_in_cleans_plan` (the doctor↔clean invariant, in both
directions) and `a_config_key_lisa_cannot_lift_out_is_the_operators_edit_not_cleans` (the remedy that
moves in step 2 — without a test, that change is invisible).

## Risks, and what I do about each

**The five filenames are a name-and-location warrant, not a byte warrant.** Unlike `CLAUDE.md`, a
`research.md` carries no frozen generator output to compare against, so clean's evidence is that the
name is the retired workflow's and the location is Lisa's publication target. An operator who wrote
their own `docs/active/work/T-024-01/design.md` by hand loses it. Mitigations, all three already in
the design: the ticket must be `done`, removal needs an explicit flag, and the path is named in the
plan first. Recorded in `review.md` as the sharpest edge in the ticket — not hidden.

**Reordering `display_order` touches two commands this ticket does not own.** `proposal` 7→8 and
`loop` 8→9. Nothing depends on the numbers except the rendered order, and the rendered order is
pinned by `TOP_LEVEL_HELP_SNAPSHOT`, so a mistake fails loudly in the same commit.

**`remove_dir_all` on 168 trees, on a repository where those trees are gitignored.** The blast radius
if `is_done` were wrong is a finished ticket's private scratch files, which are already excluded from
history by `.lisa/.gitignore`. That is the least costly class to be wrong about, which is why it is
the one that removes whole trees rather than named files.

**A concurrent `lisa loop`.** Clean removes attempt folders for done tickets; a running loop does not
hold those. It does hold `.lisa/signals/`, which the litter rule excludes, and the completion journal,
which is not per-ticket. No lock is taken and none is needed for the two classes chosen — a
consequence of the rule, worth stating in `review.md` rather than defending with machinery.

**`triage_agent::bounded_runner_returns_valid_proposal_and_surfaces_failure` is flaky under load.**
T-057-02-02 recorded it: a 2-second wall-clock deadline that has timed out once under parallel test
load. This ticket adds filesystem-heavy tests, so it may surface again. If it does, it is rerun and
reported as flaky with the `git diff` evidence that this ticket touches no byte of it — not
suppressed, and not counted as green without the exit code.
