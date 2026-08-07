# T-056-01-02 — Structure: the-check-can-see-the-project

Three files change. Nothing is created; nothing outside `crates/lisa-cli` is touched.

| File | Change |
| --- | --- |
| `crates/lisa-cli/src/unblock.rs` | Modified — `run_check` runs in `root`; ~250 lines of snapshot machinery deleted; `CheckResult::ChangedFiles` removed; unit tests reworked |
| `crates/lisa-cli/tests/parked_ux.rs` | Modified — git-repo fixture helper added; the story's regression fixture added for both entry points; three snapshot-premised tests removed |
| `crates/lisa-core/src/provenance.rs` | Unchanged (stated here because it was a candidate) — `CheckOverrideOutcome::ChangedFiles` stays on the wire |

## 1. `crates/lisa-cli/src/unblock.rs`

### 1.1 Deletions

Everything below is removed, in one block, because it exists only to build and police the
snapshot:

```
ReadOnlySnapshot (struct, impl, Drop)      :538-559
snapshot_project                           :561-571
git_visible_paths                          :573-601
os_string_from_bytes (both cfg arms)        :603-612
is_safe_relative                            :614-619
copy_visible_path                           :621-628
copy_entry                                  :630-664
copy_small_tree                             :666-693
should_skip                                 :695-709
set_tree_read_only                          :711-733
fingerprint_tree                            :735-766
collect_entries                             :768-778
path_bytes (both cfg arms)                  :780-789
```

Imports that go with them: `std::collections::BTreeSet`, `std::ffi::OsString`, `std::path::
Component`, `sha2::{Digest, Sha256}`, `tempfile::TempDir`, and `std::fs::{self}`'s `fs` alias if
nothing else uses it. `std::io::{Read, Seek, SeekFrom}` and `File` stay — `read_capture` needs
them. `std::path::{Path, PathBuf}` stays.

`sha2` and `tempfile` remain workspace dependencies of `lisa-cli` for other modules; only the
`use` lines in this file change. (Verified at implementation time, not assumed: if `sha2` becomes
unused by the crate, the manifest is left alone — trimming a dependency is outside this ticket's
ownership and would touch a file other tickets share.)

### 1.2 `CheckResult`

```rust
enum CheckResult {
    Passed,
    Failed,
    Inconclusive,
    TimedOut,
}
```

`ChangedFiles` is removed. Callers to fix, all in this file:

- `override_outcome` (`:195-203`) — drop the arm. Its return type
  `CheckOverrideOutcome` keeps its own `ChangedFiles` variant; this function simply never yields
  it now. A short comment says why the wire variant outlives the classification.
- `decline_header` (`:291-299`) — drop the arm; `DECLINE_CHANGED_FILES` (`:52`) is deleted.
- `run_world_rechecks` (`:245-248`) — the non-pass match arm loses `| CheckResult::ChangedFiles`.
- `exit_code_line` (`:302-311`) — unaffected; it matches on `TimedOut` and the code.

### 1.3 `run_check`

New body shape (the wait loop, capture reading, and `CheckRun` construction are unchanged):

```rust
/// Run one recorded check against the project itself.
///
/// The check runs in `root` — the tree the operator changed and the only tree
/// whose state they can act on. It therefore sees every file that is there,
/// gitignored build output included; a relative path in a check resolves the
/// way it would in the operator's own shell.
///
/// Lisa neither prevents nor detects a check that writes. Isolation was traded
/// for that visibility deliberately (see the ticket's design artifact): a
/// before/after fingerprint of a live tree cannot tell the check's writes from
/// a concurrent agent thread's, and reporting someone else's writes as the
/// check's would be a false gate of exactly the kind this story closes. The
/// read-only requirement lives in the check contract instead.
fn run_check(root: &Path, check: &str, timeout: Duration) -> Result<CheckRun, String> {
    let scratch = tempfile::tempdir()...;
    let mut stdout = tempfile::tempfile()...;
    let mut stderr = tempfile::tempfile()...;

    // Cloned out rather than read back later: this is the directory the check is
    // about to be given, and it is what the report names.
    let directory = root.to_path_buf();

    let mut command = Command::new("/bin/sh");
    command.arg("-c").arg(check)
        .current_dir(&directory)
        .env("TMPDIR", scratch.path()) // + TMP, TEMP
        .stdin(Stdio::null())
        .stdout(...).stderr(...);
    // process_group(0) unchanged

    // spawn / poll / timeout / kill — unchanged
    // captures read on every path — unchanged

    let (result, exit_code) = if timed_out {
        (CheckResult::TimedOut, None)
    } else {
        classify_exit(status.code())
    };

    Ok(CheckRun { result, check: check.to_string(), directory, exit_code, ... })
}
```

The two `fingerprint_tree` calls and the `ReadOnlySnapshot::new` call are the only removals inside
the function. `directory` keeps its existing role and comment, so `CheckRun::directory` and every
report built from it stay correct with no change at the reporting layer — which is what
T-056-01-01 built them for.

`root` is passed to `run_check` today and was used only to build the snapshot; it becomes the
directory itself. No signature change, so `run_unblock` (`:146`) and `run_world_rechecks`
(`:233`) are untouched.

### 1.4 Unit tests in `unblock.rs`

The module's `run()` helper takes a root and runs a check there; every test builds a
`tempfile::tempdir()` as that root. After the change those tempdirs *are* the working directory
rather than a source to copy from, so most tests keep working unchanged — but their meaning
shifts, and two need real edits.

Removed:

- `relative_write_never_reaches_live_project_and_cannot_pass` — its assertion is the isolation
  property being traded.
- `mutation_inside_disposable_state_is_detected_even_after_chmod` — `ChangedFiles` is gone.

Edited:

- `every_decline_header_is_distinct_and_names_the_way_through` — drop `ChangedFiles` from the
  header list and drop the writing check from the report loop; three headers, still distinct,
  still ending in `--override-check`.

Added:

- `a_check_reads_the_project_it_runs_in` — write `tracked.txt` and `out/marker` under the root,
  run `test -f out/marker && test -f tracked.txt`, assert `Passed`. The unit-level form of
  criterion 2 (one run, one gitignored-shaped artifact, one ordinary file).
- `the_check_runs_in_the_project_root` — assert `CheckRun::directory == root` exactly, alongside
  the existing `pwd -P` test which already asserts reported == observed.

Kept as-is: `passing_and_failing_checks_carry_the_command_directory_and_code`,
`the_reported_directory_is_the_one_the_check_observed`,
`exit_two_and_shell_failures_are_inconclusive_not_a_verdict`,
`timeout_is_bounded_and_kills_the_shell_group`,
`the_field_line_is_reported_not_asserted_as_lisas_verdict`,
`observed_lines_strip_controls_fold_tabs_and_cap_length_and_count`.

Note on `timeout_is_bounded_and_kills_the_shell_group`: it runs `sleep 5 & wait` in the root. With
no snapshot to build, the pre-spawn work drops to two tempdir creations, so the test gets faster,
not slower.

## 2. `crates/lisa-cli/tests/parked_ux.rs`

### 2.1 New fixture helpers

```rust
/// The story's regression fixture: a git repository that ignores `out/`, with a
/// real `out/marker` on disk. Without `git init` the bug is invisible — the
/// snapshot's non-git arm copied ignored files, so every existing black-box
/// check took the arm the field failure did not.
fn git_init(root: &Path);                 // git init -q; user.email/user.name; no commit needed
fn write_ignored_marker(root: &Path);     // .gitignore := "out/\n"; out/marker := "built\n"
```

`git_init` must set `user.email`/`user.name` locally so the fixture does not depend on the
machine's global git config. It runs `git init --quiet` with `-C root`. No commit is made — the
research established `git ls-files` succeeds in a fresh repository, and the *bug* reproduces
without one, but the assertion that matters is post-fix behaviour, which is commit-independent.

`project()` is left alone. Tests that want a repository call `git_init` explicitly, so the
existing 20 tests keep their current (non-git) shape and nothing else in the file shifts.

Guard: if `git` is unavailable the git-fixture tests would misreport. They assert on `git init`'s
exit status, so an absent `git` fails loudly rather than silently passing on the non-git arm.

### 2.2 New tests

1. **`unblock_sees_a_gitignored_build_output_the_operator_can_see`** — criterion 1, the named
   regression fixture. `git_init` + `write_ignored_marker`; blocked ticket `T-OUT` with an
   operator-owned block whose `check` is `test -f out/marker`; `lisa unblock T-OUT` exits 0,
   stdout names the ticket as able to run again, ticket status is `Open`, and no `check-override`
   row was written (it passed on its merits, it was not forced).
2. **`a_check_reads_a_gitignored_artifact_and_a_tracked_file_in_one_run`** — criterion 2. Same
   fixture plus a tracked `README.md`; check is
   `test -f out/marker && test -f README.md && test -f docs/active/tickets/T-BOTH.md`. Passing
   requires the ignored artifact, an ordinary file, and a file git knows about, resolved from the
   same cwd in one run.
3. **`the_check_runs_where_lisa_says_it_ran`** — criterion 4, black box. A failing check that
   prints `pwd -P` and exits 1; parse `ran in:` out of the decline and compare it with the
   check's own stdout line, with the same `/private` tolerance the unit test uses; additionally
   assert the reported directory is the project root, not a temp path.
4. **`world_recheck_sees_the_same_tree_an_operator_unblock_does`** — criterion 5. Same fixture,
   world-owned block with the `out/marker` check, driven through `lisa recheck-world`; asserts
   `Reopened`-equivalent (exit 0, status `Open`, ticket named in stdout). Paired with the
   operator-side test above, this covers "a test covers both entry points against the
   `out/marker` fixture".
5. **`a_non_git_project_and_a_git_project_agree_about_what_a_check_sees`** — criterion 6. The
   `out/marker` fixture built twice, once with `git_init` and once without, same check, both
   reopen. This is the surviving form of "the two paths agree"; there is one path now.

### 2.3 Removed tests

- `automatic_recheck_write_attempt_is_disposable_and_cannot_reopen` (`:551`)
- `attempted_write_is_disposable_reported_plainly_and_does_not_reopen` (`:614`)

Both assert the `ChangedFiles` decline. Their subject no longer exists. Their neighbours
(`automatic_recheck_ignores_operator_owned_passing_checks`,
`automatic_recheck_timeout_is_bounded_and_cannot_reopen`) still pin that automation acts only on a
pass and that the timeout still bounds it, so the automation side keeps non-pass coverage.

### 2.4 Tests that must keep passing untouched

All 18 remaining black-box tests. The riskiest are the ones whose checks write or read relative
paths under the old copy semantics — `failing_check_declines_plainly_and_leaves_the_ticket_
waiting`, `a_declined_check_reports_the_command_the_directory_the_code_and_both_streams`,
`escape_sequences_and_tabs_are_stripped_from_everything_shown`,
`passing_check_reopens_and_the_next_schedule_sees_the_ticket`,
`world_owned_passing_check_self_clears_without_an_operator_command`,
`world_owned_failing_check_stays_parked_without_churn`. Their checks are `exit N`/`printf` forms
with no filesystem dependency, so they are cwd-insensitive; any that turn out to assert a temp
path in `ran in:` are fixed to assert the project root, which is the same criterion-4 property.

## 3. Ordering

The changes are one coherent unit and are committed in two:

1. **The behaviour change.** `unblock.rs` in full (spawn in root, deletions, `CheckResult`, unit
   tests) — self-contained and green on its own: `cargo test -p lisa-cli` passes because the
   black-box tests removed in step 2 are the only ones that depend on `ChangedFiles`… which is
   false. So step 1 must include the two black-box deletions to stay green.

   Corrected: **commit 1** = `unblock.rs` + the two obsolete `parked_ux.rs` tests removed. Green.
2. **The regression coverage.** The five new `parked_ux.rs` tests and their fixture helpers. Green,
   and failing-before-the-fix is demonstrated by running the new criterion-1 test against the
   pre-change binary during implementation and recording the result in `progress.md`.

Both commits go through `lisa commit-ticket --ticket-id T-056-01-02 --include <exact paths>`.
