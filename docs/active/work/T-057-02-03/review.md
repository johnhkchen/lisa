# Review — T-057-02-03 lisa-clean

`lisa clean` exists. It is the third command of the trio, it removes nothing until an operator says
so twice, and on a project a fresh `lisa init` created it finds nothing at all.

## The scope decision, as the first acceptance criterion requires

**Chosen: (b) — currency plus litter**, on this rule:

> **Lisa's litter is what Lisa wrote for one ticket that your board records as done, inside a
> directory Lisa created for that ticket — and nothing else is ever a candidate.**

One sentence, which was the ticket's condition for taking (b) at all. It is in `clean.rs`'s module
doc, so the next person to widen this command reads it before they can.

Two classes satisfy it: the five filenames the retired workflow produced under
`docs/active/work/{ticket}/`, and a finished ticket's tree under `.lisa/attempts/{ticket}/`.

**The rule also refuses two things the ticket's (b) sketch listed, and I took that as the rule
working rather than as a gap to paper over.** Both are recorded here because they are a narrowing of
what the ticket sketched:

- **Pane signal files** (`.lisa/signals/pane-N.*`). Pane-scoped, not ticket-scoped, so the rule
  excludes them. Independently true: a signal file is live state whenever a run is in progress, and
  clean cannot prove no run is in progress. Four files, all gitignored. Excluding them is what keeps
  the rule one sentence instead of one sentence plus a liveness caveat.
- **Work directories for tickets that are not on the board at all.** "The board records nothing" is
  not "the board records it done". A lookup fails whenever a ticket was renamed, filed elsewhere, or
  is still being drafted — and destroying published work product on a failed lookup is exactly the
  silent destruction P1 forbids. They are refused **and reported**, with the reason
  `nothing on your board records T-024-99 finished`, so the operator can see Lisa looked and decide.
  I considered reading `.lisa/completion-journal.jsonl` for a second opinion on doneness and did not:
  it would turn a one-sentence rule into a two-source rule ("the board, or Lisa's journal, and if
  they disagree…") in order to delete files nobody has complained about.

## What changed

| File | Change |
| --- | --- |
| `crates/lisa-cli/src/clean.rs` | **New, 1,465 lines with tests.** `CleanAction` / `CleanVerb` / `CleanClass`, `plan_clean_actions`, `run_clean`, the two litter detectors, `board_status`, `reachability`, and the renderer. |
| `crates/lisa-cli/src/currency.rs` | One remedy moved: a `[scheduling] auto_advance` Lisa cannot lift out surgically now reports `Remedy::Operator` naming the one-line edit, not `Remedy::Clean`. |
| `crates/lisa-cli/src/main.rs` | `mod clean`, `Commands::Clean` at `display_order = 7` (beside `doctor`), the match arm; `proposal` 7→8 and `loop` 8→9. |
| `crates/lisa-cli/tests/help_surface.rs` | `OPERATOR_COMMANDS` 9→10, `OWN_COMMANDS` 18→19, the top-level snapshot, a tenth operator help snapshot, and the count assertions and wording that carried "18". |
| `docs/knowledge/flag-audit.md` | Three rows: `--remove`, `--dry-run`, `--path`. |
| `README.md` | New `### lisa clean` — what is on the list, what is never on it — plus a pointer from `lisa init`'s "what it will not remove" and one line under `lisa doctor` about the three remedies. |

Four commits, each `just check`-green (exit code read directly):

- `255cc05` the mechanism
- `2aff8fb` the acceptance tests
- `a3cec39` the README
- `0eec175` the closing end-to-end assertion

Nothing deleted. No change to `init.rs`, `doctor.rs`, `config.rs`, or `lisa-core`.

## The decisions worth a reviewer's attention

**Two verbs, and the second one is why a remedy moved.** Clean removes files; it never edits the
contents of one. `RetiredKeyRemoval::NotSurgical` means Lisa has already concluded it cannot edit that
`.lisa.toml` without reformatting bytes the operator wrote — so pointing a deleting command at it
would ship exactly the defect `currency.rs` was built to prevent ("no window where doctor names a
command that does nothing"). The honest remedy is the operator's one-line edit, stated in words. That
is the one behaviour change outside the new module, and it is asserted
(`a_config_key_lisa_cannot_lift_out_is_the_operators_edit_not_cleans`).

**The doctor↔clean invariant is asserted, not structural, and that is deliberate.**
`every_finding_that_names_clean_is_a_removal_in_cleans_plan` drives `currency::inventory` and
`plan_clean_actions` over one fixture and checks both directions: every finding whose remedy is
`run \`lisa clean\`` is a removal in clean's plan, and every currency removal in clean's plan is such
a finding. Making it structural would mean clean's plan deriving from doctor's rendered strings, which
is worse. Clean calls `currency::retirements` — the shared detector — and never `inventory`, so the
call graph stays acyclic and nothing in `currency` or `init` ever calls `clean`.

**The weak byte mark still stops before this command.** T-057-02-02 kept an edited-but-marked
`CLAUDE.md` out of the inventory precisely because the remedy for a retired-but-preserved file is
*this* command. Clean inherits that line unchanged: `ContextFile { proven_generation: false }` is
dropped from the plan entirely, under any flag. A prefix match is enough to print `skip` beside a
file; it is never enough to delete somebody's standing instructions to every model that reads their
repository.

**Removals are never capped; refusals are.** The ticket's "the one-line summary should let a reader
decide without reading the list" is permission for the list to be long, so every removal gets its own
line — 1,013 lines on this repository. A preview of 1,140 deletions that hides 1,100 of them is not a
preview. Refusals are capped at five plus an aggregate, in the renderer rather than the detector, for
the same reason `init::plan_retirements` caps retired-phase tickets.

**Paths print repository-relative.** Init prints whatever it joined
(`/Users/…/lisa/./docs/active/work/…`), which is tolerable across thirty lines and unreadable across a
thousand. The verbs, columns, `preserved:` prefix and `Dry run complete. No changes made.` closer are
init's exactly; only the path rendering differs.

**`--dry-run` is not inert.** It is the default said out loud, and it `conflicts_with = "remove"`, so
`lisa clean --dry-run --remove` exits 2 with `the argument '--dry-run' cannot be used with '--remove'`
rather than deleting. That conflict is what earns it a flag-audit row.

**Directories that end up empty are predicted, not discovered.** A work directory is removed only when
every entry in it was a planned removal, and the removal uses `remove_dir`, never `remove_dir_all` —
so if a file appeared between plan and execution the call fails and the directory stays, instead of
clean destroying something no line named.

## Test coverage

Twelve new tests in `clean.rs`. Every acceptance criterion maps to one:

| Criterion | Test |
| --- | --- |
| Scope decision recorded with the rule as one sentence | this document, §1, and `clean.rs`'s module doc |
| Registered in help beside `init` and `doctor`, covered by `help_surface.rs` | `help_surface::{top_level_help_matches_snapshot, operator_help_matches_snapshots, all_nineteen_subcommands_resolve, about_line_and_operator_help_are_jargon_free}` |
| **Bare run changes nothing; tree byte-identical** | `a_bare_run_prints_the_plan_and_changes_not_one_byte` — recursive snapshot of every file's bytes *and* every directory and symlink name, so an empty-directory removal cannot hide |
| Removal only under an explicit flag; every removed path named in the plan first | `every_removed_path_was_named_in_the_plan_first` — diffs the before/after snapshots and requires each vanished path to be a planned removal or under a planned tree, then requires every planned removal to actually be gone |
| Refusal: nothing under `docs/active/tickets/` or `docs/active/stories/` | `the_board_is_never_a_candidate` — no plan line under either directory *at all*, plus both files byte-identical after `--remove` |
| Refusal: nothing outside paths Lisa created | `nothing_outside_lisas_own_directories_is_a_candidate` — twelve paths including `README.md`, `.lisa/completion-journal.jsonl`, `.lisa/signals/pane-0.lease`, and — inside a done ticket's own work directory — `review.md`, `review-disposition.json`, an operator note and a `harness/` subdirectory |
| Refusal: no work artifact for a ticket that is not `done` | `an_unfinished_tickets_notes_are_never_candidates` — `open`, `review`, and absent-from-the-board, each refused with its own reason naming what the board actually records |
| Refusal: no path reachable by symlink out of the project root | `a_symlink_out_of_the_project_is_refused` — a symlinked note and a done attempt tree containing a symlink to an outside directory; both refused in the preview, both outside files intact after `--remove` |
| Fresh `lisa init` project reports nothing to remove | `a_fresh_init_project_has_nothing_to_remove` — empty plan, `Nothing to remove.` from both a bare run and `--remove`, tree unchanged |
| **End to end, closing the story** | `the_0_4_4_fixture_ends_current_and_every_human_file_survives` — fixture → real `run_init` → real `run_clean(remove)` → `inventory().is_current()` is true, and nine hand-written files are byte-identical, including the open ticket's notes and attempt folder |
| `just check` green | exit code 0, read directly, 1,470 tests |

Three more cover properties the criteria imply: the doctor↔clean correspondence (both directions), the
moved config-key remedy, and the voice (`every_removal_line_says_why_and_the_summary_stands_alone` —
every line carries a non-empty reason, refusals keep init's `preserved:` prefix, the summary is one
line naming both counts and both classes).

**Also smoke-tested with the real binary** against a hand-built 0.4-shaped fixture in the scratchpad:

- bare run listed 5 files and 1 folder with a reason on each, and two refusals; a `shasum` of every
  file before and after was identical;
- `--dry-run --remove` exited 2 with Clap's conflict message;
- `--remove` printed `Removed 5 files and 1 folder. Everything else is as it was.`, and what survived
  was exactly the board, `README.md`, `docs/knowledge/our-notes.md`, `T-1`'s `review.md` and operator
  note, and every one of open `T-2`'s notes and its attempt folder;
- `lisa doctor` on that fixture routed the surgically-removable `auto_advance` to `lisa init`, which
  is the correct half of the remedy split.

## Open concerns

**1. The five filenames are a name-and-location warrant, not a byte warrant — the sharpest edge in
this ticket.** Unlike `CLAUDE.md`, a `research.md` carries no frozen generator output to compare
against, so clean's evidence is that the name is the retired workflow's and the location is Lisa's
publication target. An operator who hand-wrote `docs/active/work/T-024-01/design.md` themselves would
lose it. Three things stand between that and a bad outcome, all asserted: the ticket must be `done`,
removal needs `--remove`, and the path is printed first with its reason. I judged that sufficient
because the ticket names those files first among the residue it wants cleared, and because the
alternative (a content heuristic over agent-authored prose) would be guesswork dressed as proof. It is
the one place where clean's warrant is weaker than init's, and a reviewer should agree with that trade
before 0.5.0 ships.

**2. No automated test drives the `lisa clean` binary end to end.** The end-to-end test calls
`run_init` and `run_clean` as functions, which is every line of behaviour except Clap parsing and
`require_lisa_project` — and Clap parsing is covered by `help_surface` against the real binary. The
gap was closed manually (transcript summarised above). This matches T-057-02-01's disposition for
`lisa doctor` and its stated reason: a binary-level test of these commands ends up asserting the host
machine.

**3. The end-to-end criterion says "reported fully current by `lisa doctor`"; the test asserts
`inventory().is_current()`.** Those are the same fact — `doctor::format_project_currency` renders the
inventory and derives no judgment of its own, and `test_fresh_init_project_gets_one_current_line`
already pins that rendering. I ran the real `lisa doctor` on the smoke fixture to see it, but the
committed assertion is at the inventory.

**4. Clean takes no lock, and does not need one for the two classes chosen.** A concurrent `lisa loop`
holds `.lisa/signals/` (excluded by the rule as pane-scoped) and the completion journal (excluded as
project state). It does not hold a done ticket's attempt folder. This is a consequence of the rule
rather than a guarantee I built, so it is worth stating rather than defending with machinery — but it
also means widening the rule later to cover signals would need a lock, and that is the moment to
remember it.

**5. Retirement copy still names 0.5.0 while a dev build reports 0.4.4.** Inherited verbatim from
T-057-02-02's concern #2: clean's plan lines say "stopped running in 0.5.0" two lines from a doctor
that says "this Lisa is 0.4.4". Correct once 0.5.0 is cut, momentarily odd before it. Cosmetic, and it
resolves at the version bump.

**6. `docs/active/work/{ticket}/progress.md` was a real artifact of the retired workflow and is on the
removal list.** 192 of this repository's 194 work directories have one. Worth a reviewer's eye only
because it is the file most likely to contain something an operator went back and added by hand; see
concern 1.

**7. Untouched, pre-existing, and reported for continuity.** `upsert_missing_config_keys` still writes
duplicate commented stubs when `[scheduling]` is the last section of a `.lisa.toml`
(T-057-02-02 concern #1 — the fix is one `else` in `insert_after`), and the removed key's comment still
survives the line it described (concern #2 there). Neither is this ticket's, and neither moved.

## Deviation from the plan

None in substance. `plan.md` step 11 put the end-to-end test in its own commit and it is there
(`0eec175`); the only change is that the second-run assertion in that test asserts
`starts_with("Nothing to remove.")` rather than string equality, because a second clean correctly
still reports what it left alone — the open ticket's work — and saying so is the behaviour the voice
section asked for.

## What a human should look at first

Concern 1, and the plan output. Run `lisa clean` in this repository: it names 833 files and 167
folders across 1,013 lines, all of it Lisa's own, for tickets the board records as done. That is the
deletion this ticket makes possible, and reading the first screen of it is the fastest way to agree or
disagree with the warrant behind it.
