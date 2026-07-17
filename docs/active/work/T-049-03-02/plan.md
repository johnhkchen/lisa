# Plan — T-049-03-02

## Execution rules

- Work continuously through Implement and Review.
- Keep all phase artifacts in the private attempt work directory.
- Do not edit ticket phase/status fields.
- Preserve unrelated dirty worktree files.
- Use `apply_patch` for repository edits.
- Use `lisa commit-ticket` for each meaningful owned source unit.
- Pass only exact repository-relative include paths.
- Do not use ordinary `git add` or `git commit`.

## Step 1 — establish baseline evidence

Read-only checks:

1. Run the existing explicit-commit compiled fixture:

   ```text
   cargo test -p lisa-cli --test seal_visibility \
     doctor_explicit_commit_uses_shared_missing_identity_hard_failure
   ```

2. Confirm it fails the subprocess, names explicit guard intent, identifies missing identity,
   and includes both remedies.
3. Record the result in `progress.md`.

Verification criterion: the first enforcement acceptance condition is already executable and
green before new edits.

## Step 2 — add temporal pinned-commit regression

Modify `crates/lisa-plugin/src/lib.rs` in the existing completion failure test area.

Implementation actions:

1. Create the standard completion failure fixture for `T-PINNED-COMMIT`.
2. Resolve `Auto + CommitSealSupport::Available` using the core resolver.
3. Assert and install the resulting `Commit` seal in plugin config.
4. Initialize a real temporary Git repository.
5. Configure local test identity.
6. Create an initial commit containing fixture state.
7. Dispatch Review completion and assert one commit launch.
8. Remove only the temporary `.git` directory.
9. Invoke the real native completion transaction and require failure.
10. Feed that real failure through the plugin result handler.
11. Assert Review/blocked parking and a nonempty disposition ask.
12. Assert commit-labeled failure journal rows.
13. Assert zero confirmed rows, journal seal labels, commit ids, or content hashes.

Focused verification:

```text
cargo test -p lisa-plugin \
  auto_pinned_commit_with_mid_run_repository_loss_parks_without_journal_seal
```

If the real repository-loss error classification parks immediately, retain the conservative
classification. Do not broaden production classification just to change retry count.

## Step 3 — format and commit enforcement unit

1. Run `cargo fmt --all -- --check`.
2. If formatting is needed, run `cargo fmt --all` and inspect the exact diff.
3. Ensure only the intended test block changed in the owned Rust file.
4. Run the focused plugin test again.
5. Commit with:

   ```text
   lisa commit-ticket --ticket-id T-049-03-02 \
     --message "Test pinned commit failure without downgrade" \
     --include crates/lisa-plugin/src/lib.rs
   ```

6. Confirm the owned Rust file is no longer modified.

Atomic result: the no-silent-switch invariant is durable independently of fixture docs.

## Step 4 — extend `/cbt/prepare`

Modify `docker/chromebook-test/bin/prepare`.

Actions:

1. Add `NO_GIT=0` state.
2. Parse `--no-git`.
3. Extend usage/help text.
4. In no-Git mode, fail if Git is available.
5. Refuse to overwrite an existing `~/no-git-demo`.
6. Create the exact ticket directory.
7. Write `T-NOGIT-001.md` as an evidence-only, no-source task.
8. Require the directory to remain free of `.git`.
9. Generate the tailored no-Git measured instruction.
10. Tell the agent to select the same configured Lisa client as the outer CLI.
11. Add `no_git: 0|1` to leg metadata.

Compatibility checks:

- normal instruction remains unchanged;
- discovery instruction remains unchanged;
- pin, ancient-Zellij, and XDG flags still parse;
- unknown flags still exit 2 with complete usage.

## Step 5 — extend `/cbt/grade`

Modify `docker/chromebook-test/bin/grade`.

Actions:

1. Detect the no-Git metadata marker.
2. Select normal demo versus no-Git project root.
3. Set 600-second normal and 1200-second no-Git bounds.
4. Run doctor from the correct project root.
5. Capture the exact completion-seal line.
6. Make normal noninteractive init explicit with `--no-history`.
7. In no-Git mode, rerun init idempotently in the completed project.
8. Validate and dry-run the selected project.
9. Require Git command absence for the no-Git leg.
10. Require no `.git` directory in the prepared project.
11. Require the fixed ticket's Done frontmatter.
12. Embed the Node JSONL/hash verifier.
13. Reject malformed/missing journal evidence and unsafe bindings.
14. Require the ticket path binding.
15. Capture the verifier summary.
16. Add wall limit, seal line, and verifier result to the run record.

Shell robustness checks:

- every possibly empty variable has a default before record generation;
- commands expected to fail do not trigger `set -e` (the grader does not enable it);
- heredoc delimiters are quoted;
- fixed project paths are shell-quoted;
- Node catches and prints exceptions before nonzero exit.

## Step 6 — extend safe evidence collection

Modify `justfile` in `cbt-collect`.

Actions:

1. Leave existing `/tmp` evidence collection unchanged.
2. Probe/copy only the fixed no-Git journal path.
3. When present, create destination `.lisa` and ticket/work parent directories.
4. Copy the journal, fixed ticket, and fixed work directory.
5. Do not copy whole home directories or agent state.

Syntax verification:

- evaluate/list just recipes;
- inspect the rendered recipe if supported;
- preserve current indentation and recipe shell model.

## Step 7 — verify fixture scripts before commit

Run:

```text
sh -n docker/chromebook-test/bin/prepare
sh -n docker/chromebook-test/bin/grade
```

Static checks:

- `--no-git` appears in parser and help;
- `no_git` appears in metadata and grader selector;
- exact journal-only line appears in grader;
- fixed ticket id is consistent across prepare, grade, collector, and runbook;
- collector paths are explicit.

Synthetic verifier check:

1. Create a disposable project under a temporary directory.
2. Write a Done ticket and one Review artifact.
3. Compute their SHA-256 values.
4. Write a matching schema-shaped confirmed journal row.
5. Exercise the same Node verifier body or the grader branch with controlled fixture inputs.
6. Mutate one file and verify the checker fails.

If direct grader execution would require an installed Lisa or mutate the workspace, keep the
test to extracted verifier logic and shell syntax.

## Step 8 — commit fixture instrumentation

Inspect exact diffs for the three paths, then commit:

```text
lisa commit-ticket --ticket-id T-049-03-02 \
  --message "Add scripted no-Git completion fixture" \
  --include docker/chromebook-test/bin/prepare \
  --include docker/chromebook-test/bin/grade \
  --include justfile
```

Confirm those owned paths are clean afterward.

## Step 9 — update the standing runbook

Modify `docs/knowledge/chromebook-install-test.md`.

Required prose changes:

1. Add `prepare --no-git` to the ritual command block.
2. Distinguish ordinary install legs from the full no-Git completion claim.
3. Add no-Git leg N to the matrix/protocol.
4. State fresh authentication and metering requirements remain unchanged.
5. Document the prepared directory and exact ticket id.
6. Document same-client configuration.
7. State this ticket ships the fixture but does not execute the leg.
8. Add the exact doctor journal-only sentence as a pass criterion.
9. Require ticket Done and a confirmed journal row.
10. Require every recorded SHA-256 binding to match current bytes.
11. Require no commit id and no Git installation.
12. State the 1200-second hard stop for the full completion leg.
13. Document extra collected evidence paths.
14. Add seal and journal-verification fields to the result template.

Review the prose against the scripts line by line. Commands in the runbook must match actual
flag names, paths, ids, and output strings.

## Step 10 — commit protocol documentation

Commit only the runbook:

```text
lisa commit-ticket --ticket-id T-049-03-02 \
  --message "Score repository-less completion in Chromebook protocol" \
  --include docs/knowledge/chromebook-install-test.md
```

Confirm the runbook path is clean afterward.

## Step 11 — package and workspace verification

Run in order:

1. focused mid-run plugin test;
2. `cargo test -p lisa-cli --test seal_visibility`;
3. `cargo test -p lisa-plugin`;
4. `cargo test --workspace`;
5. shell syntax checks;
6. justfile syntax/evaluation check;
7. `cargo fmt --all -- --check`.

If failures involve unrelated dirty files, identify the exact path/test and do not edit other
ticket work. If failures involve owned paths, fix, rerun, and make an additional exact-path
ticket commit.

## Step 12 — final ownership audit

Read-only checks:

- `git status --short`;
- `git diff --` each owned path;
- recent log entries for the ticket commits;
- no owned file staged, modified, or untracked;
- private phase artifacts all present.

Do not attempt to clean `.lisa` ledgers, ticket phase changes, or concurrent ticket files.

## Step 13 — Review artifacts

Write `progress.md` with:

- completed steps;
- exact ticket commits;
- tests and outcomes;
- any deviations;
- explicit note that the manual no-Git leg was not executed.

Write `review.md` with:

- outcome-first summary;
- changed files;
- behavioral guarantees;
- fixture protocol details;
- test coverage;
- open limitations and manual evidence still needed.

If all owned work is committed and automated verification passes, write exactly:

```json
{"disposition":"pass","reason":null}
```

Otherwise write a valid actionable block disposition. After both Review artifacts exist, remain
on this ticket and stop; Lisa owns completion publication and seat release.
