# T-056-01-02 — Progress: the-check-can-see-the-project

Plan followed as written. Two commits, one deviation (recorded in §Deviations), gate green.

## Step 0 — Before-fix evidence ✅

Built the fixture by hand and ran the *pre-change* `lisa unblock` against it:

```
$ git -C $S ls-files --cached --others --exclude-standard
.gitignore
CLAUDE.md
docs/active/tickets/T-OUT.md
docs/active/work/T-OUT/review-disposition.json        # out/marker ABSENT

$ lisa unblock T-OUT --path $S
That didn't work yet — the check ran and did not pass.

  what ran:  test -f out/marker
  ran in:    /var/folders/kn/7f93dn8n1wb51m_jydvylncw0000gn/T/.tmporllVN
  exit code: 1

  the check printed nothing.

If you have done this and checked it yourself, run:
  lisa unblock T-OUT --override-check
$ echo $?
1
```

`ran in:` naming a `/var/folders/...T/.tmp*` path is the bug in one line: the check was standing
somewhere the operator has never been.

Later re-run at the *test* level, which is what criterion 1 actually asks for — see Step 2.

## Step 1 — Run the check in the project root ✅

Commit `8feeee0` — *run the check where the operator stands*.

`crates/lisa-cli/src/unblock.rs`:

- `run_check` takes `let directory = root.to_path_buf()` and spawns there. `ReadOnlySnapshot::new`
  and both `fingerprint_tree` calls are gone.
- New doc comment on `run_check` stating the execution contract and, explicitly, why mutation
  detection was not moved to the live tree.
- Deleted (253 lines): `ReadOnlySnapshot` + `Drop`, `snapshot_project`, `git_visible_paths`,
  `os_string_from_bytes`, `is_safe_relative`, `copy_visible_path`, `copy_entry`, `copy_small_tree`,
  `should_skip`, `set_tree_read_only`, `fingerprint_tree`, `collect_entries`, `path_bytes`.
- Imports dropped: `BTreeSet`, `OsString`, `Component`, `sha2::{Digest, Sha256}`,
  `tempfile::TempDir`, and `std::fs` (moved into the test module, which still uses it).
- `CheckResult::ChangedFiles` and `DECLINE_CHANGED_FILES` removed; `override_outcome`,
  `decline_header`, and `run_world_rechecks`'s non-pass arm updated.
  `CheckOverrideOutcome::ChangedFiles` left on the wire with a comment saying why.
- Unit tests: removed `relative_write_never_reaches_live_project_and_cannot_pass` and
  `mutation_inside_disposable_state_is_detected_even_after_chmod`; added
  `a_check_reads_the_project_it_runs_in` and `the_check_runs_in_the_project_root`; edited
  `every_decline_header_is_distinct_and_names_the_way_through`.

`crates/lisa-cli/tests/parked_ux.rs`: removed the two `ChangedFiles` black-box tests, in this
commit rather than the next so the commit is green on its own (the plan called this out).

Verification: `cargo test -p lisa-cli` exit 0 (385 unit + 18 black box);
`cargo clippy -p lisa-cli --all-targets -- -D warnings` exit 0; `cargo fmt --all`.

Manual re-run of the Step 0 fixture with the rebuilt binary:

```
$ lisa unblock T-OUT --path $S
T-OUT can run again.
$ echo $?
0
$ grep '^status:' $S/docs/active/tickets/T-OUT.md
status: open
```

## Step 2 — Regression fixtures on both entry points ✅

Commit `b21261f` — *pin the gitignored-artifact fixture on both entry points*.

Helpers added to `parked_ux.rs`: `git_init` (asserts every git step, sets local identity),
`write_ignored_marker`, and `assert_hidden_from_git` — the last one so a passing unblock is
evidence about the working directory and not about a `.gitignore` that quietly stopped applying.

Five tests, one per remaining criterion:

| Test | Criterion |
| --- | --- |
| `unblock_sees_a_gitignored_build_output_the_operator_can_see` | 1 |
| `a_check_reads_a_gitignored_artifact_and_a_tracked_file_in_one_run` | 2 |
| `the_check_runs_where_lisa_says_it_ran` | 4 |
| `world_recheck_sees_the_same_tree_an_operator_unblock_does` | 5 |
| `a_non_git_project_and_a_git_project_agree_about_what_a_check_sees` | 6 |

**Fails-before / passes-after, at the test level.** The criterion-1 test was run against the
pre-change `unblock.rs` (restored from `469638d` into the working tree, run, then restored from
`HEAD` — verified byte-identical afterwards by `git status`):

```
BEFORE_EXIT=101
thread 'unblock_sees_a_gitignored_build_output_the_operator_can_see' panicked at parked_ux.rs:668:
That didn't work yet — the check ran and did not pass.
  ran in:    /var/folders/kn/7f93dn8n1wb51m_jydvylncw0000gn/T/.tmpFJZqEU
  exit code: 1
test result: FAILED. 0 passed; 1 failed
```

After: `test result: ok. 23 passed; 0 failed` for the whole file.

## Step 3 — Gate ✅

```
$ just check; echo "exit=$?"
exit=0
```

Judged by exit code, not by reading output — `cargo check -p lisa-plugin --target wasm32-wasip1`,
`cargo fmt --check`, clippy `-D warnings` on all three crates, `cargo test --workspace`. The
`wasm32-wasip1` target was already installed on this machine this time, so the environment gap
T-056-01-01 recorded did not recur.

## Deviations from the plan

1. **`git_init` first draft contained leftover scaffolding** (a dead first loop from an editing
   slip). Caught before any commit and removed; the committed helper is the single loop the
   structure artifact describes. No behavioural effect.
2. **`sha2` did not become an unused dependency.** The plan flagged it as a possibility to leave
   alone if it happened; `lisa-cli` still uses it elsewhere, so `Cargo.toml` was untouched as
   planned. Nothing to report.

Nothing else deviated. No step needed a third commit.

## Ticket-owned tree state at end of Implement

```
$ git status --porcelain
 M .lisa/completion-journal.jsonl      # Lisa's own bookkeeping
 M .lisa/provenance.jsonl              # Lisa's own bookkeeping
?? docs/active/stories/S-056-01.md     # hand-authored story, not this ticket's
?? docs/active/tickets/T-056-01-02.md  # this ticket's own file — Lisa publishes it
?? docs/active/tickets/T-056-01-03.md  # next ticket's file
?? docs/active/work/T-056-01-02/       # Lisa's published copies of these artifacts
```

No ticket-owned **source** file is staged, modified, or untracked: both
`crates/lisa-cli/src/unblock.rs` and `crates/lisa-cli/tests/parked_ux.rs` are committed through
`lisa commit-ticket` and clean against `HEAD`.
