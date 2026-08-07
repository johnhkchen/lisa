# T-056-01-02 — Research: the-check-can-see-the-project

Descriptive map of the code the ticket names, the surrounding contracts, and the constraints any
fix has to hold. No solution here.

## 1. The one call site the ticket is about

`crates/lisa-cli/src/unblock.rs` is the whole surface. After T-056-01-01 landed, the line numbers
in the ticket have moved; the current shape is:

| Concern | Where (current file) |
| --- | --- |
| `run_check` — builds snapshot, spawns, classifies | `unblock.rs:360-451` |
| `.current_dir(&directory)` where `directory` is the snapshot | `unblock.rs:374-380` |
| `ReadOnlySnapshot` (tempdir + copy + chmod) | `unblock.rs:538-559` |
| `snapshot_project` (git path list, else small-tree copy) | `unblock.rs:561-571` |
| `git_visible_paths` — the `--exclude-standard` call | `unblock.rs:573-601` |
| `copy_visible_path` / `copy_entry` | `unblock.rs:621-664` |
| `copy_small_tree` + `should_skip` (non-git fallback) | `unblock.rs:666-709` |
| `set_tree_read_only` | `unblock.rs:711-733` |
| `fingerprint_tree` / `collect_entries` / `path_bytes` | `unblock.rs:735-789` |
| `run_unblock` — operator entry point | `unblock.rs:108-167` |
| `run_world_rechecks` — automation entry point | `unblock.rs:212-253` |

Both entry points call the same `run_check(root, &check, CHECK_TIMEOUT)` — `unblock.rs:146` and
`unblock.rs:233`. There is one code path, so a change in `run_check` reaches both by construction;
what does *not* exist today is a test that asserts that shared behaviour from the automation side
against a gitignored artifact.

## 2. What the snapshot actually contains

`ReadOnlySnapshot::new` (`unblock.rs:543-548`) does three things in order: `tempfile::tempdir()`,
`snapshot_project`, `set_tree_read_only(.., true)`.

`snapshot_project` has two arms:

- **git arm.** `git_visible_paths` runs `git -C <root> ls-files -z --cached --others
  --exclude-standard`. Every returned path is copied. `--exclude-standard` applies `.gitignore`,
  `.git/info/exclude`, and the global excludes file, so *every ignored path is absent by
  construction*. Anything a project builds (`dist/`, `.astro/`, `target/`), anything it fetches
  (`node_modules/`, `.venv/`), and Lisa's own `.lisa/` are all normally ignored, so none of them
  exists in the snapshot. The arm is taken whenever `git` exits 0 — including in a repository with
  no commits.
- **non-git arm.** `copy_small_tree` walks the real tree and copies everything except a hardcoded
  skip list (`should_skip`, `unblock.rs:695-709`): first component `.git`, `target`, or
  `node_modules`, plus `.lisa/attempts`. Note the asymmetry — this arm *does* copy build output,
  because it knows nothing about `.gitignore`.

So the two arms disagree today about what a check can see: in a git project `out/marker` is
missing; in the same tree without `.git` it is present. Criterion 6 of the ticket is aimed at that
disagreement.

`set_tree_read_only` clears the write bits (`mode & !0o222`) over the copied tree, and `Drop`
restores them so the `TempDir` can be removed.

## 3. What the check is handed

`run_check` (`unblock.rs:360-400`):

- `before = fingerprint_tree(snapshot.path())` — SHA-256 over sorted relative paths, type byte,
  permission mode, and full file contents.
- a second `tempfile::tempdir()` as `scratch`, exported as `TMPDIR`/`TMP`/`TEMP`.
- `Command::new("/bin/sh").arg("-c").arg(check).current_dir(&directory)` where `directory` is the
  snapshot path, cloned out so the report names the value actually handed to `current_dir`.
- stdin null; stdout/stderr to two `tempfile::tempfile()` handles.
- `process_group(0)` on unix, so the timeout can SIGKILL the whole group.

After the wait loop: captures are read on every path; then
- timed out → `TimedOut`,
- else `after = fingerprint_tree(snapshot.path())`; mismatch (or an `Err`) → `ChangedFiles`,
- else `classify_exit` → `Passed` (0) / `Inconclusive` (2, 126, 127, or signal death) / `Failed`.

`CheckRun` (`unblock.rs:83-97`) carries `result`, `check`, `directory`, `exit_code`, and up to
`MAX_OBSERVED_LINES` sanitized lines per stream out to the reporter. Its doc comment states that
`directory` is carried rather than recomputed specifically so it stays true when this ticket moves
where checks run.

## 4. Every other spawn in Lisa

`grep current_dir` over `crates/`:

- `proposal.rs:249` — `.current_dir(root)`
- `triage_agent.rs:44` — `.current_dir(&args.root)`
- `loop_cmd.rs:487` — `.current_dir(root)`
- `lib.rs:12915`, `13032`, `templates.rs`, `init.rs` — test fixtures only
- `unblock.rs:380` — the snapshot

The ticket's claim holds against the current tree: this is the only production spawn that
redirects cwd away from the project root. The plugin's own launcher
(`run_command_with_env_variables_and_cwd`, `lib.rs:1776-1781`) passes `self.project_root` for the
recheck command, so even the process that *invokes* `lisa recheck-world` sets cwd to the project —
the redirect happens one level deeper, inside `run_check`.

## 5. What tells a reviewer where a check runs

Nothing.

- `crates/lisa-core/src/disposition.rs:272-277` (strict authoring path) and `:402-406` (tolerant
  parser) both treat `check` as an opaque non-empty string. The only rule enforced is
  non-emptiness; the strict path's error text says "make the block disposition check a non-empty
  read-only command, or omit check".
- `crates/lisa-core/src/parking.rs:84` carries it through as `Option<String>` on the remedy.
- `docs/knowledge/rdspi-workflow.md:59` says "Supply a `check` whenever the remedy is externally
  observable… The check verifies the remedy but must never perform it." No cwd, no file
  visibility, no time budget, no writability.
- `disposition.rs:629-655` has a test asserting check content is stored and never executed during
  parsing — recording a check never tries it. (Making the recording path try it is an explicit
  criterion of T-056-01-03, not this ticket.)

So a reviewer writing `node scripts/check-touch.mjs` has read a doc that promises "read-only
verification command" and nothing else. The relative path is the natural thing to write.

## 6. The concurrency fact that constrains any mutation gate

`run_world_rechecks` is not an operator-quiescent command. The plugin scheduler builds
`lisa recheck-world --path <project_root>` (`lib.rs:1722-1743`) and fires it asynchronously
whenever a world-owned park with a check exists and none is in flight
(`request_world_recheck`, `lib.rs:1761-1783`), at the scheduler's ordinary cadence — i.e. while
other threads' Claude/Codex sessions are actively editing files in the same tree.

`lisa unblock` has the same exposure: an operator runs it while a loop session is live.

Today that is harmless because the fingerprint is taken over a private frozen copy. Any design
that fingerprints the *live* tree before and after inherits every concurrent write by an unrelated
agent thread, `.lisa/provenance.jsonl` append, or ticket status flip. That would attribute another
writer's changes to the check — structurally the same error T-056-01-01 just removed from the
message layer (someone else's words reported as Lisa's finding).

## 7. Existing tests that encode the current behaviour

Unit tests in `unblock.rs:791-1008` (9 tests). The ones that touch this ticket's subject:

- `the_reported_directory_is_the_one_the_check_observed` (`:838`) — runs `pwd -P`, compares the
  check's own output with `CheckRun::directory`, tolerating the macOS `/private` prefix because
  `pwd -P` resolves symlinks and the reported path does not. This is criterion 4's unit half and
  is written to survive a move.
- `relative_write_never_reaches_live_project_and_cannot_pass` (`:902`) — asserts a `touch
  must-not-exist` does not create the file in the real root *and* does not pass. Its first half is
  a property of the snapshot, not of the check contract.
- `mutation_inside_disposable_state_is_detected_even_after_chmod` (`:913`) — `chmod u+w fixture &&
  printf after > fixture` classifies `ChangedFiles`, and the live file is unchanged.
- `every_decline_header_is_distinct_and_names_the_way_through` (`:933`) — includes a
  `ChangedFiles`-producing check in its loop.

Black-box tests in `crates/lisa-cli/tests/parked_ux.rs` (20 tests, 674 lines). Fixture helpers:
`project()` builds a temp dir named `project with spaces` containing `docs/active/tickets`,
`docs/active/work`, `CLAUDE.md` — **and never runs `git init`**, so every existing black-box check
today takes the `copy_small_tree` arm, not the git arm. That is why the field bug has no
regression test: the black-box suite never exercised `--exclude-standard`.

Relevant black-box tests: `failing_check_declines_plainly_and_leaves_the_ticket_waiting` (`:168`),
`a_declined_check_reports_the_command_the_directory_the_code_and_both_streams` (`:197`),
`passing_check_reopens_and_the_next_schedule_sees_the_ticket` (`:408`),
`world_owned_passing_check_self_clears_without_an_operator_command` (`:490`),
`automatic_recheck_write_attempt_is_disposable_and_cannot_reopen` (`:551`),
`attempted_write_is_disposable_reported_plainly_and_does_not_reopen` (`:614`).

The last two, plus the two unit tests above, are the ones whose *premise* is the snapshot rather
than the check contract.

## 8. Provenance coupling

`record_check_override` (`unblock.rs:170-193`) writes `CheckOverrideRecord` with a `directory`
field = `run.directory.display().to_string()`, and `result` mapped by `override_outcome`
(`:195-203`) onto `CheckOverrideOutcome` (`lisa-core/src/provenance.rs`, `SCHEMA_VERSION` 10).
The outcome enum has a `ChangedFiles` variant on the wire. Any change to `CheckResult`'s variant
set has to keep that mapping total and the wire format readable.

## 9. Constraints collected

1. Both entry points must change together; they already share `run_check`.
2. Criterion 1's fixture declares no execution context, so whatever default a check gets must
   already see `out/marker` in a git repo that ignores `out/`.
3. Criterion 4 requires reported cwd == observed cwd; the reporting side is already written to
   carry, not recompute.
4. The live tree is concurrently written by other Lisa threads during both entry points.
5. `node_modules/` is the reason "just add `--ignored`" is not a fix — the ticket says so, and the
   `should_skip` list independently confirms it is treated as un-copyable.
6. T-056-01-03 owns the 5-second budget, the writability decision, record-time validation of
   checks, `run_world_rechecks`'s silence, and the `rdspi-workflow.md` contract paragraph. This
   ticket must not pre-empt them, but must not leave a gate that only they can make honest.
7. `just check` = `check-wasm` (cargo check on wasm32-wasip1) + `fmt-check` + clippy `-D warnings`
   on all three crates + `cargo test --workspace`.
