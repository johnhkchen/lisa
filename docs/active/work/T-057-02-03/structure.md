# Structure — T-057-02-03 lisa-clean

## Files

| File | Change | Role after |
| --- | --- | --- |
| `crates/lisa-cli/src/clean.rs` | **new** | Owns clean's plan, its litter detection, its symlink/root gate, and its rendering. The only module that knows the five retired filenames. |
| `crates/lisa-cli/src/currency.rs` | one remedy changed | A `ConfigKey` retirement Lisa cannot lift out surgically becomes `Remedy::Operator`, not `Remedy::Clean`. |
| `crates/lisa-cli/src/main.rs` | `mod clean;` + `Commands::Clean` + match arm; `display_order` shifted for `proposal`/`loop` | Registers the third command of the trio beside `doctor`. |
| `crates/lisa-cli/tests/help_surface.rs` | four constants, one snapshot, two counts, module doc | Pins clean's help surface exactly as it pins the other nine. |
| `docs/knowledge/flag-audit.md` | +3 rows | `--remove`, `--dry-run`, `--path`. |
| `README.md` | +`### lisa clean`, and the trio named where `init` describes what it will not remove | The reference an operator reads before running it. |

No change to `init.rs`, `doctor.rs`, `config.rs`, or `lisa-core`.

## The seam

```
clean::plan_clean_actions ──> currency::retirements     (the shared detector — currency class)
                          └─> clean::litter             (this module's own — the two litter classes)
                          └─> clean::board_status       (ticket id -> Status, active + archive)

currency::inventory ─────────> init::plan_init_actions  (unchanged)
                    └────────> currency::retirements    (unchanged)

doctor ──────────────────────> currency::inventory      (unchanged)
```

Clean calls `currency::retirements`, **not** `currency::inventory`. Two reasons: `inventory` renders
for doctor's reader and flattens dispositions into strings, while clean needs the `Disposition` to
know which path to remove; and `inventory` calls `init::plan_init_actions`, which clean has no
business dragging in. The call graph stays acyclic: nothing in `currency` or `init` ever calls
`clean`.

The doctor↔clean invariant is therefore asserted rather than structural: a test drives
`currency::inventory` and `clean::plan_clean_actions` over one fixture and checks the two sets
correspond. That is the honest shape — making it structural would mean clean's plan *deriving* from
doctor's rendered strings, which is worse.

## New surface

```rust
// crates/lisa-cli/src/clean.rs

/// One line of clean's plan. Two verbs: it removes, or it says why it did not.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum CleanAction {
    RemoveFile { path: PathBuf, reason: String },
    /// A directory that ends up empty because every entry in it is removed above.
    RemoveEmptyDir { path: PathBuf, reason: String },
    /// A whole tree Lisa created for one finished ticket.
    RemoveTree { path: PathBuf, reason: String },
    /// Considered and declined. `reason` arrives with the `preserved:` prefix.
    Keep { path: PathBuf, reason: String },
}

impl CleanAction {
    fn is_removal(&self) -> bool;
    fn path(&self) -> &Path;
}
// Display, matching init's columns exactly:
//   "  remove  {path} ({reason})"
//   "  remove  {path}/ ({reason})"
//   "  skip    {path} ({reason})"

/// Everything clean would do, computed before anything is touched.
pub(crate) fn plan_clean_actions(root: &Path) -> Vec<CleanAction>;

/// Print the plan, and carry it out when `remove` is true.
pub fn run_clean(root: &Path, remove: bool) -> Result<(), String>;
```

Order inside the returned plan — removals first, grouped currency → retired notes → attempt folders,
then refusals, then the standing statement. Init closes its preview on the destructive group because
that group is small and easily buried; clean's destructive group *is* the output, so it leads and the
refusals close.

### Private helpers, and what each one is for

```rust
/// The five filenames the retired workflow produced. `review.md` and
/// `review-disposition.json` are the current workflow's and are absent from this list
/// deliberately — they are still read.
const RETIRED_WORKFLOW_NOTES: [&str; 5] =
    ["research.md", "design.md", "structure.md", "plan.md", "progress.md"];

/// `status:` for every ticket id found in the configured ticket directory and in
/// `docs/archive/tickets/`. Absence from the map is "the board records nothing",
/// which is not "done".
fn board_status(root: &Path) -> BTreeMap<String, Status>;

fn is_done(board: &BTreeMap<String, Status>, ticket_id: &str) -> bool;

/// The last gate. Applied at plan time so a refusal is visible in the preview.
enum Reachability { Safe, Symlinked(String) }
fn reachability(canonical_root: &Path, path: &Path) -> Reachability;
```

`board_status` reads `status:` out of frontmatter with a small local reader rather than
`lisa_core::ticket::parse_ticket`, for one reason: `parse_ticket` fails the whole file on any
malformed field, and a ticket Lisa cannot parse must read as *not done* rather than vanish from the
map with no distinction. A raw `status:` read gives "the file says done" or "it does not", which is
exactly the question. (`currency::frontmatter_value` exists for the same class of reason and is the
precedent; clean gets its own copy rather than widening that one's visibility, because `currency`'s
version is documented as being about *phase* specifically.)

## Control flow inside `plan_clean_actions`

1. `let canonical_root = root.canonicalize()` — once. Every gate compares against this, so a
   `/var` → `/private/var` platform symlink on the root itself is not mistaken for an escape.
2. **Currency class.** `currency::retirements(root)`, keeping only:
   - `RetirementKind::WorkflowDocument` with `Disposition::Preserve` → `RemoveFile`
   - `RetirementKind::ContextFile { proven_generation: true }` with `Disposition::Preserve` → `RemoveFile`

   Everything else is dropped, including `ContextFile { proven_generation: false }` (the weak mark —
   T-057-02-02's line, inherited unchanged), `ConfigKey` (not a file removal), and `TicketPhase` (the
   board). Dropped rather than reported: init already prints a `skip` line for each of them and
   repeating it here would imply clean had an opinion it does not have.

   A `Disposition::RemoveFile` is *also* dropped — init resolves those itself, so by the time clean
   runs there is nothing there. That is what makes "`lisa clean` after `lisa init` finds nothing"
   true on an upgraded project rather than coincidental.
3. **Retired notes.** For each entry of the configured work directory: read the ticket id from the
   directory name; if `is_done`, every present name in `RETIRED_WORKFLOW_NOTES` that clears
   `reachability` becomes a `RemoveFile`; if not, one `Keep`. Then, if every entry of that directory
   is a planned removal, one `RemoveEmptyDir`.
4. **Attempt folders.** For each entry of `.lisa/attempts/`: if `is_done` and the whole subtree
   clears `reachability`, one `RemoveTree`; otherwise one `Keep`.
5. Refusals sorted by path; removals kept in class order, sorted by path within a class, so two runs
   over the same tree print the same list in the same order.

Steps 3 and 4 both consult the same `board_status` map, read once.

## Rendering, and the counts

```
<summary line>

Planned actions:
  remove  …                       (never capped)
  skip    …                       (five, then one aggregate line)

Never a candidate: your board (docs/active/tickets/, docs/active/stories/), your settings, and
anything Lisa did not write.

Dry run complete. No changes made. Add --remove to carry this list out.
```

Nothing to do:

```
Nothing to remove.
```

After `--remove`:

```
Removed 1140 files and 168 folders. Everything else is as it was.
```

The refusal cap is `RENDER_KEEP_LIMIT: usize = 5`, in the renderer. `reachability` refusals are
exempt from the cap and printed in full — there are never many, and one of them means something is
wrong.

## Execution

```rust
for action in &plan {
    match action {
        RemoveFile { path, .. }     => fs::remove_file(path),
        RemoveTree { path, .. }     => fs::remove_dir_all(path),
        RemoveEmptyDir { path, .. } => fs::remove_dir(path),   // refuses if the prediction was wrong
        Keep { .. }                 => {}
    }
}
```

Failures are collected, printed under `Could not remove:`, and returned as one `Err` at the end.
Clean does not abort on the first failure the way init does: 1,140 independent deletions where the
first one fails on a permission bit should not leave the other 1,139 undone with no report. Every
deletion is independent, so partial completion is a well-defined state.

`remove_dir` rather than `remove_dir_all` for the empty-directory line is the load-bearing choice: if
the prediction was wrong — a file appeared between plan and execution — the call fails and the
directory stays, instead of removing something no plan line named.

## Tests

### `clean.rs` (unit, on tempdir fixtures)

| Criterion | Test |
| --- | --- |
| bare run changes nothing, tree byte-identical | `a_bare_run_prints_the_plan_and_changes_not_one_byte` — recursive `tree_snapshot` before/after, plus dir-set equality so an empty directory removal cannot hide |
| every removed path was named in the plan first | `every_removed_path_was_named_in_the_plan_first` — plan, run, then walk the diff of the two snapshots and require each vanished path to be a removal line's path |
| board is never a candidate | `the_board_is_never_a_candidate` — a done ticket, its story, both byte-identical after `--remove`, and no plan line under either directory |
| nothing outside paths Lisa created | `nothing_outside_lisas_own_directories_is_a_candidate` — `README.md`, `CLAUDE.md` (hand-written), `docs/knowledge/our-notes.md`, an operator note *inside* a done ticket's work directory, and a `harness/` subdirectory: all still there |
| no work artifact for a ticket that is not done | `an_unfinished_tickets_notes_are_never_candidates` — `status: open`, `status: review`, and a ticket absent from the board entirely; each reported as `skip` with its own reason |
| no path reachable by symlink out of the root | `a_symlink_out_of_the_project_is_refused` — `docs/active/work/T-1/plan.md` as a symlink to an outside file, and a done attempt folder containing a symlink to an outside directory; both refused in the plan, both outside files untouched after `--remove` |
| fresh init reports nothing | `a_fresh_init_project_has_nothing_to_remove` |
| doctor↔clean correspondence | `every_finding_that_names_clean_is_a_removal_in_cleans_plan` — both directions |
| the retired config key stops naming clean | `a_config_key_lisa_cannot_lift_out_is_the_operators_edit_not_cleans` |
| voice | `every_removal_line_says_why_and_the_summary_stands_alone` — one line per removal, each with a non-empty parenthesised reason; summary names both counts and both classes |
| the current workflow's artifacts survive | asserted inside the retired-notes tests: `review.md` / `review-disposition.json` are never candidates |
| end to end, closing the story | `the_0_4_4_fixture_ends_current_and_every_human_file_survives` — fixture → `run_init` → `run_clean(remove)` → `inventory().is_current()` **and** every hand-written file byte-identical |

### `currency.rs`

One existing expectation to re-check and one to add: the `NotSurgical` config key must now report
`Remedy::Operator`, and the two `Remedy::Clean` cases (`a_pointer_left_at_agents_md_keeps_claude_md`,
`an_edited_retired_document_needs_removal_rather_than_init`) must still say `Clean`, because they are
what clean removes.

### `tests/help_surface.rs`

`OPERATOR_COMMANDS` 9 → 10, `OWN_COMMANDS` 18 → 19 (and the `assert_eq!(…, 18)`, the test name
`all_eighteen_subcommands_resolve`, and the module doc's "all 17"), `TOP_LEVEL_HELP_SNAPSHOT`, and a
tenth entry in `OPERATOR_HELP_SNAPSHOTS` in canonical order. `about_line_and_operator_help_are_jargon_free`
then covers clean for free.

### The end-to-end fixture

`init.rs`'s `upgrade_fixture` is the right shape but lives in its private `tests` module and carries a
ticket at `phase: structure`, whose `StaleContent` finding would make `is_current()` false forever —
the tension T-057-02-02's review recorded as open concern #3. Clean's fixture is its own, and settles
that row so the end-to-end criterion can assert the strong form (`is_current()` outright):

```
.lisa.toml                              version 0.4.0, [scheduling] auto_advance, an operator comment
.lisa/hooks/on-stop.sh                  a prior shipped generation        -> init updates
docs/knowledge/rdspi-workflow.md        a shipped generation + one appended team line  -> init preserves, clean removes
docs/knowledge/our-notes.md             hand-written                      -> untouched
CLAUDE.md                               byte-exact 0.4.4 generation       -> init removes
AGENTS.md                               hand-written, names CLAUDE.md     -> untouched (and see below)
README.md                               hand-written                      -> untouched
docs/active/tickets/T-024-01.md         status: done, phase: done         -> untouched
docs/active/stories/S-024.md            hand-written                      -> untouched
docs/active/tickets/T-024-02.md         status: open                      -> untouched
docs/active/work/T-024-01/{5 notes}     retired workflow                  -> clean removes
docs/active/work/T-024-01/review.md     current workflow                  -> untouched
docs/active/work/T-024-01/operator-note.md   hand-written                 -> untouched
docs/active/work/T-024-02/{5 notes}     ticket is open                    -> refused, reported
.lisa/attempts/T-024-01/1/work/…        finished attempt                  -> clean removes
.lisa/attempts/T-024-02/1/work/…        unfinished                        -> refused, reported
```

The hand-written `AGENTS.md` naming `CLAUDE.md` is in the fixture on purpose: it is the case where
init preserves `CLAUDE.md` because a pointer survives, so the fixture exercises **both** currency
removals through the real `run_init` → `run_clean` sequence rather than only the workflow document.
`AGENTS.md` itself is the operator's and stays byte-identical, pointing at a file that is now gone —
which clean's plan line says out loud before it happens.

## Commit units

1. `clean.rs` + `currency.rs` remedy + `main.rs` registration + `help_surface.rs` + `flag-audit.md`
   — the mechanism. These cannot be separated: registering the command without the help snapshot
   fails `help_surface`, adding flags without audit rows fails `flag_audit`, and `clippy -D warnings`
   fails on a module nothing calls. T-057-02-02 hit the same wall and recorded it.
2. The acceptance tests in `clean.rs`.
3. `README.md`.
4. The end-to-end test, last, because it is the one that closes the story.
