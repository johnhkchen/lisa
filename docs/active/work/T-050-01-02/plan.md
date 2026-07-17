# Plan — T-050-01-02 never-a-dead-end surfaces

## Execution rules

- Work from the ticket's current Plan phase into Implement without pausing.
- Record implementation results and deviations in `progress.md`.
- Use `apply_patch` for every source and artifact edit.
- Do not edit ticket frontmatter.
- Do not write phase artifacts to the shared work directory.
- Do not use `git add`, `git commit`, or the ordinary index.
- Commit each meaningful unit with `lisa commit-ticket` and exact include paths.
- Preserve unrelated Lisa-managed changes in the worktree.

## Step 1 — establish the pre-init process contract

Files:

- `crates/lisa-cli/src/main.rs`

Actions:

1. Add the exact setup first-line constant.
2. Add the project-marker predicate.
3. Recognize `.lisa.toml` as the explicit configured-project marker.
4. Recognize `docs/active/tickets` as the default-layout marker.
5. Add a preflight that renders the first-line and technical detail.
6. Add a small call helper only if it reduces duplicated exit handling cleanly.
7. Guard `doctor` immediately after resolving its root.
8. Guard `validate` immediately after resolving its root.
9. Guard the normal and ticket-specific `status` dispatch after root resolution.
10. Guard `loop` before client/config parsing and module entry.
11. Leave `init`, `notes`, help, and plumbing commands unguarded.

Verification:

- `cargo fmt --check` after the edit.
- Inspect the diff to ensure the exact sentence has no `Error:` prefix.
- Confirm no external check can run before the guard in the four arms.
- Confirm all existing post-guard error wrappers remain unchanged.

## Step 2 — make standalone notes empty output explicit

Files:

- `crates/lisa-cli/src/notes.rs`

Actions:

1. Keep `note_lines` unchanged.
2. Keep `print_notes` unchanged for embedded and populated rendering.
3. Branch in `run_list` on the collected slice.
4. Print exactly `Nothing to read.` for an empty collection.
5. Delegate populated collections to the existing printer.
6. Leave `run_ack` unchanged.

Verification:

- Confirm populated note output code is byte-identical.
- Confirm empty output has one newline and no extra blank line.
- Run the existing notes integration test after updating its expectation.

## Step 3 — make status optional sections legible

Files:

- `crates/lisa-cli/src/status.rs`

Actions:

1. Add the empty branch to `print_waiting_on_you`.
2. Print the existing heading, `Nothing waiting.`, and one blank separator.
3. Add a private status-specific notes renderer.
4. For empty notes, print the heading, `Nothing to read.`, and separator.
5. For populated notes, delegate to `notes::print_notes` unchanged.
6. Replace the direct notes printer call in `run_status`.
7. Preserve the ordering before `DAG:`.
8. Preserve the zero-ticket early return unless tests expose a ticket requirement conflict.

Verification:

- Run `cargo test -p lisa-cli --test parked_ux`.
- Run `cargo test -p lisa-cli --test notes_ux`.
- Inspect any expected-output change and ensure it is an intentional empty section only.
- Confirm populated remedy and note lines remain unchanged.

## Step 4 — convert clean empty validation to guidance success

Files:

- `crates/lisa-cli/src/init.rs`

Actions:

1. Remove the no-ticket readiness Error diagnostic.
2. Preserve the early return after an empty scan.
3. Preserve scan errors accumulated before that branch.
4. In `run_validate`, render diagnostics first when errors exist.
5. Detect clean `ticket_count == 0` before generic success output.
6. Resolve the configured ticket directory for display.
7. Print the exact one-paragraph guidance.
8. Return `Ok(())` before config summary output.
9. Leave non-empty valid and invalid output paths unchanged.

Verification:

- A complete empty project returns success.
- A missing-hook project remains failure even with zero tickets.
- A malformed-ticket project remains failure even if zero tickets parse.
- A valid non-empty project retains its current success text and config lines.
- A custom ticket directory is interpolated rather than replaced by the default.

## Step 5 — add focused black-box regressions

Files:

- Add `crates/lisa-cli/tests/never_dead_end.rs`.
- Update `crates/lisa-cli/tests/notes_ux.rs` only for the intentional empty expectation.
- Update other existing test snapshots only when the new empty section is the cause.

Actions:

1. Add the compiled-binary command helper.
2. Add a fresh temporary root helper.
3. Table-test `loop --dry-run`, `status`, `validate`, and `doctor` pre-init failures.
4. Pin exit 1, empty stdout, exact setup lead, and later technical detail.
5. Initialize a complete empty root using the CLI with `--no-history`.
6. Pin empty notes as exactly `Nothing to read.\n` and success.
7. Add `validate_empty_board_exits_zero_with_ticket_guidance`.
8. Pin its exact default-path paragraph, stderr, and exit 0.
9. Add one ready ticket to the initialized root for status.
10. Pin both named empty sections and their order before `DAG:`.
11. Pin status success and empty stderr.
12. Update the old empty notes regression name and expected output.

Verification:

- Run `cargo test -p lisa-cli --test never_dead_end`.
- Run `cargo test -p lisa-cli --test notes_ux`.
- Run `cargo test -p lisa-cli --test parked_ux`.
- Ensure failure messages show complete stdout/stderr for diagnosis.

## Step 6 — run targeted module tests

Commands:

```text
cargo test -p lisa-cli notes::tests
cargo test -p lisa-cli status::tests
cargo test -p lisa-cli init::tests::test_validate
cargo test -p lisa-cli loop_cmd::tests
```

Checks:

- The notes formatter still yields empty lines for the low-level empty slice.
- Status DAG behavior and config display remain intact.
- Validation setup, parse, DAG, route, and hook diagnostics remain intact.
- Direct loop structure tests still receive their existing technical errors.
- No new warning indicates dead or duplicated helpers.

## Step 7 — format and inspect the implementation unit

Commands:

```text
cargo fmt --check
git diff --check
git diff -- crates/lisa-cli/src/main.rs crates/lisa-cli/src/notes.rs crates/lisa-cli/src/status.rs crates/lisa-cli/src/init.rs
git status --short
```

Checks:

- Only ticket-owned source paths show implementation changes.
- Lisa-managed ticket paths remain outside the include set.
- No ordinary staged changes exist for ticket-owned paths.
- Copy matches Design exactly.
- No flags or help strings changed.

## Step 8 — commit the behavior unit

Command shape:

```text
lisa commit-ticket \
  --ticket-id T-050-01-02 \
  --message "Lead empty CLI surfaces with guidance" \
  --include crates/lisa-cli/src/main.rs \
  --include crates/lisa-cli/src/notes.rs \
  --include crates/lisa-cli/src/status.rs \
  --include crates/lisa-cli/src/init.rs
```

Checks:

- Record the returned commit ID in `progress.md`.
- Confirm the four source paths are clean afterward.
- Confirm unrelated ticket metadata remains modified and untouched.

## Step 9 — commit the test unit

Command shape:

```text
lisa commit-ticket \
  --ticket-id T-050-01-02 \
  --message "Pin never-dead-end CLI output" \
  --include crates/lisa-cli/tests/never_dead_end.rs \
  --include crates/lisa-cli/tests/notes_ux.rs
```

- Include any additional exact existing test path only if it was actually modified.
- Do not include unchanged files.
- Record the returned commit ID in `progress.md`.
- Confirm every ticket-owned test path is clean afterward.

## Step 10 — full verification

Commands:

```text
cargo test -p lisa-cli
cargo test --workspace
cargo fmt --check
git diff --check
```

Additional inspection:

- Run the new black-box test with `--nocapture` only if diagnosing failure.
- Review `git show --stat` for both ticket commits.
- Check `git status --short` for leftover ticket-owned files.
- Verify ordinary index state remains as found.
- Verify no grader file changed.

Pass criteria:

- All focused and workspace tests pass.
- All five operator-facing changed surfaces are string-pinned.
- Each pre-init failure leads with the exact setup line.
- Empty notes succeeds with exactly one sentence.
- Empty status optional sections are named and explicit.
- Empty validate succeeds with configured-path guidance.
- Valid non-empty and populated output regressions remain green.
- No ticket-owned source path remains modified, staged, or untracked.

## Step 11 — progress artifact

File:

- `.lisa/attempts/T-050-01-02/1/work/progress.md`

Content:

- Summarize each completed implementation unit.
- Record exact source and test paths.
- Record commit IDs from `lisa commit-ticket`.
- Record focused and full verification commands and results.
- Document deviations from this plan before relying on them.
- Name any open implementation concern.

## Step 12 — Review phase

Files:

- `.lisa/attempts/T-050-01-02/1/work/review.md`
- `.lisa/attempts/T-050-01-02/1/work/review-disposition.json`

Actions:

1. Review the final committed diff rather than only the working tree.
2. Map every acceptance criterion to exact tests and implementation paths.
3. Summarize files modified and added.
4. Assess coverage for untouched, partial, empty, populated, malformed, and custom paths.
5. Record known limitations and confirm the deferred grader change.
6. Verify no ticket-owned path remains dirty.
7. Write pass disposition only if all required work and tests are complete.
8. Run `lisa check-disposition T-050-01-02`.
9. Correct every disposition issue.
10. Stop on this ticket after Review; do not publish or start another ticket.
