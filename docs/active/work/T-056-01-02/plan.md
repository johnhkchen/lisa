# T-056-01-02 — Plan: the-check-can-see-the-project

Two commits. Step 0 exists to make the regression provable, and its evidence is recorded before
any source changes.

## Step 0 — Prove the fixture fails today (no commit)

Before touching `unblock.rs`, add the criterion-1 black-box test to a scratch copy and run it
against the current code, or reproduce by hand:

```sh
S=$(mktemp -d) && cd "$S" && git init -q && printf 'out/\n' > .gitignore \
  && mkdir out && printf 'built\n' > out/marker \
  && git ls-files --cached --others --exclude-standard   # out/marker absent
```

Then run the new `unblock_sees_a_gitignored_build_output_the_operator_can_see` test with the
snapshot still in place and record the failure text in `progress.md`. The acceptance criterion is
explicit that this must be "asserted to decline today, so the test fails before the fix and passes
after"; without this step that claim is unverified.

**Verification:** the test fails, and the failure is the decline — not a fixture error. Capture
the actual message.

## Step 1 — Run the check in the project root

Edit `crates/lisa-cli/src/unblock.rs` only.

1. `run_check`: drop `ReadOnlySnapshot::new`, both `fingerprint_tree` calls, and the
   `ChangedFiles` branch; set `let directory = root.to_path_buf();`. Keep the scratch `TMPDIR`,
   the capture files, `process_group(0)`, the poll loop, the SIGKILL, and the capture reads.
2. Write the contract doc comment on `run_check` (text in `structure.md` §1.3).
3. Delete the snapshot block (`ReadOnlySnapshot` through `path_bytes`) and the imports that go
   with it.
4. `CheckResult`: remove `ChangedFiles`; fix `override_outcome`, `decline_header`,
   `run_world_rechecks`'s non-pass arm; delete `DECLINE_CHANGED_FILES`.
5. Unit tests: remove `relative_write_never_reaches_live_project_and_cannot_pass` and
   `mutation_inside_disposable_state_is_detected_even_after_chmod`; edit
   `every_decline_header_is_distinct_and_names_the_way_through`; add
   `a_check_reads_the_project_it_runs_in` and `the_check_runs_in_the_project_root`.
6. `crates/lisa-cli/tests/parked_ux.rs`: remove the two `ChangedFiles` black-box tests
   (`automatic_recheck_write_attempt_is_disposable_and_cannot_reopen`,
   `attempted_write_is_disposable_reported_plainly_and_does_not_reopen`). They must go in this
   commit, not the next, or the commit is not green.

**Verification:** `cargo test -p lisa-cli` and `cargo test -p lisa-core` pass; `cargo clippy -p
lisa-cli -- -D warnings` clean (this is where an orphaned `use` or a now-unused helper shows up);
`cargo fmt --all`.

**Commit:**
`lisa commit-ticket --ticket-id T-056-01-02 --message "run the check where the operator stands"
--include crates/lisa-cli/src/unblock.rs --include crates/lisa-cli/tests/parked_ux.rs`

## Step 2 — The regression fixture and both entry points

Edit `crates/lisa-cli/tests/parked_ux.rs` only.

1. Add `git_init` and `write_ignored_marker` helpers. `git_init` asserts on `git init --quiet`'s
   status and sets `user.email`/`user.name` with `git -C <root> config`, so the fixture is
   independent of machine config and a missing `git` fails loudly.
2. Add the five tests from `structure.md` §2.2, in this order (each is independently runnable):
   1. `unblock_sees_a_gitignored_build_output_the_operator_can_see` — criterion 1
   2. `a_check_reads_a_gitignored_artifact_and_a_tracked_file_in_one_run` — criterion 2
   3. `the_check_runs_where_lisa_says_it_ran` — criterion 4
   4. `world_recheck_sees_the_same_tree_an_operator_unblock_does` — criterion 5
   5. `a_non_git_project_and_a_git_project_agree_about_what_a_check_sees` — criterion 6

**Verification:** each new test run individually by name first, then the whole file. Test 1 must
now pass with the same fixture that failed in step 0 — quote both in `progress.md`.

**Commit:**
`lisa commit-ticket --ticket-id T-056-01-02 --message "pin the gitignored-artifact fixture on both
entry points" --include crates/lisa-cli/tests/parked_ux.rs`

## Step 3 — Full gate

`just check` — `cargo check -p lisa-plugin --target wasm32-wasip1`, `cargo fmt --check`, clippy
`-D warnings` on all three crates, `cargo test --workspace`.

Judged by **exit code**, not by reading output. If the wasm target is missing on this machine,
install it (`rustup target add wasm32-wasip1`) and rebuild `lisa-plugin` before judging — the
T-056-01-01 review records that exact environment gap producing three unrelated
`client_autodetect` failures. That is an environment fix, not a repository change.

**Verification:** `just check; echo "exit=$?"` → `exit=0`.

If step 3 turns up a needed source fix, it is committed as its own `lisa commit-ticket` unit with
its own `--include` paths, and recorded in `progress.md` as a deviation.

## Testing strategy

| Criterion | Level | Test |
| --- | --- | --- |
| 1 — `out/marker` fixture reopens, exit 0 | black box | `unblock_sees_a_gitignored_build_output_the_operator_can_see`, with the step-0 before/after evidence |
| 2 — one run sees a gitignored output and a tracked source | black box + unit | `a_check_reads_a_gitignored_artifact_and_a_tracked_file_in_one_run`; `a_check_reads_the_project_it_runs_in` |
| 3 — option and tradeoff recorded | artifact | `design.md` (option (a), what is traded, what replaces the materialisation rule) |
| 4 — reported cwd == observed cwd | black box + unit | `the_check_runs_where_lisa_says_it_ran`; `the_reported_directory_is_the_one_the_check_observed` (kept), `the_check_runs_in_the_project_root` (new) |
| 5 — both entry points, same fixture | black box | `world_recheck_sees_the_same_tree_an_operator_unblock_does` + criterion-1 test |
| 6 — non-git agrees with git | black box | `a_non_git_project_and_a_git_project_agree_about_what_a_check_sees`; deviation on `copy_small_tree` stated in design and review |
| 7 — `just check` green | gate | exit code 0 |

## What could go wrong, and the response

**A black-box test asserts a temp path in `ran in:`.** Expected; it becomes an assertion that the
reported directory is the project root. That is the same criterion-4 property, strengthened.

**`git` writes to the fixture tree during `git init` and a later test reads it.** Fixtures are
per-test tempdirs; no sharing.

**A test's check now writes into the fixture project.** None of the surviving checks writes; the
two that did are removed. If a kept test turns out to write, that is a real signal about the
traded guarantee and goes in `review.md`, not a silent fix.

**`sha2` or `tempfile` becomes an unused dependency of `lisa-cli`.** `tempfile` certainly stays
(scratch dir, capture files). If `sha2` goes unused, the manifest is left alone deliberately —
`Cargo.toml` is shared with tickets outside this story and dropping a dependency is not this
ticket's ownership. Clippy does not fail on an unused dependency, so the gate stays green either
way. Noted in `review.md` if it happens.

**Clippy flags something in the reduced file.** Fix in place, same commit unit.

## Out of scope, restated so it does not creep in

The 5-second budget, whether a check may write (beyond removing the run-time gate that could not
be honest), record-time validation of checks, `run_world_rechecks`'s silence on repeated
non-passes, and the `docs/knowledge/rdspi-workflow.md` execution-contract paragraph. All are
T-056-01-03 acceptance criteria.
