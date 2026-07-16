# Plan: purpose-first CLI strings

## Implementation goal

Deliver a wording-only production change that makes the CLI and both guide outputs purpose-first.
Prove the ordering with black-box string tests while retaining all E-044 help-surface locks.
Commit only the four ticket-owned source paths through Lisa's isolated transaction.

## Step 1: update the top-level self-description

File: `crates/lisa-cli/src/main.rs`.

Actions:

1. Locate the `Cli` derive metadata.
2. Replace the `about` literal with the selected purpose sentence.
3. Do not change `before_help`.
4. Do not change `after_help`.
5. Do not change any command variant or attribute.

Verification:

- Diff contains one changed production line in this file.
- The new line contains `coding agents`.
- The new line contains `ticket board`.
- The new line contains `approve every step by hand`.
- The new line contains none of DAG, WASM, Zellij, or scheduling.

## Step 2: update the generated setup-guide preamble

File: `crates/lisa-cli/src/setup_guide.rs`.

Actions:

1. Locate the header construction in `build_guide`.
2. Preserve the project name/type H1 interpolation.
3. Insert the purpose sentence as the first prose below the H1.
4. Retain the current setup directions after the new sentence.
5. Leave the seven-section vector untouched.

Verification:

- A generated guide starts with its existing H1.
- Its first prose sentence states Lisa's purpose.
- `## Step 1` through `## Step 7` remain present.
- Existing project detection tests remain unchanged and pass.
- The purpose offset is earlier than the first named mechanism offset.

## Step 3: update the embedded hooks-guide preamble

File: `crates/lisa-cli/data/hooks-guide.md`.

Actions:

1. Preserve `# Lisa Hooks Guide`.
2. Insert the purpose sentence directly below the title.
3. Preserve the existing setup/repair paragraph after it.
4. Leave all later contract content unchanged.

Verification:

- The embedded constant remains non-empty.
- Existing contract-marker tests continue to pass.
- The emitted guide contains the exact purpose anchor.
- Purpose appears before the first Zellij, scheduling, DAG, or WASM occurrence.

## Step 4: update the exact top-level snapshot

File: `crates/lisa-cli/tests/help_surface.rs`.

Actions:

1. Replace the old about line in `TOP_LEVEL_HELP_SNAPSHOT`.
2. Change no whitespace, grouping, command row, option row, or footer line.
3. Leave all operator subcommand snapshots byte-identical.

Verification:

- `top_level_help_matches_snapshot` passes.
- Snapshot diff shows exactly the about-line replacement.
- Existing grouping/visibility test passes.
- Existing command-resolution test passes.

## Step 5: add purpose-order regression coverage

File: `crates/lisa-cli/tests/help_surface.rs`.

Actions:

1. Add `PURPOSE_SENTENCE` as the exact reusable anchor.
2. Add `MECHANISM_TERMS` containing `dag`, `wasm`, `zellij`, and `scheduling`.
3. Add a case-insensitive ordering helper.
4. Require the purpose anchor to occur.
5. Find the earliest mechanism occurrence.
6. Require a mechanism occurrence so current coverage cannot pass vacuously.
7. Assert purpose precedes that occurrence.
8. Add one test covering `--help`, `setup-guide`, and `hooks-guide` output.

Verification:

- A missing purpose sentence produces an explicit failure.
- Moving purpose after any named mechanism produces an explicit failure.
- All calls exercise the built binary rather than internal rendering only.
- Hidden guide command visibility remains unchanged because direct invocation was already supported.

## Step 6: format and focused verification

Actions:

1. Run `cargo fmt --all -- --check` after applying Rust edits.
2. If formatting reports ticket-owned differences, run `cargo fmt --all` and inspect all touched paths.
3. Run `cargo test -p lisa-cli --test help_surface`.
4. Run focused setup-guide and hooks-guide unit tests, or the full `lisa-cli` test suite if filtering is inconvenient.

Verification criteria:

- Formatting check exits zero.
- All help-surface integration tests pass.
- Setup guide generation tests pass.
- Hooks embedded-content tests pass.
- No unrelated file is modified by formatting.

## Step 7: broad verification

Actions:

1. Run `cargo test --workspace`.
2. Run `just check` if available and proportionate after workspace tests.
3. Record command outcomes and durations in `progress.md`.
4. If an environmental or unrelated failure occurs, isolate and document it.

Verification criteria:

- Workspace test suite exits zero.
- WASM check and configured test suite exit zero if `just check` is run.
- No test requires production behavior changes beyond wording.

## Step 8: inspect ticket ownership and diff

Actions:

1. Run `git diff --` with the four exact source paths.
2. Confirm production diffs are wording only.
3. Confirm integration tests add only semantic string assertions and snapshot text.
4. Run `git status --short`.
5. Distinguish ticket-owned paths from pre-existing unrelated changes.

Verification criteria:

- Ticket-owned modified paths are exactly:
  - `crates/lisa-cli/src/main.rs`
  - `crates/lisa-cli/src/setup_guide.rs`
  - `crates/lisa-cli/data/hooks-guide.md`
  - `crates/lisa-cli/tests/help_surface.rs`
- No active ticket frontmatter is intentionally modified.
- No shared work artifact is written.
- No ordinary-index staging exists for ticket-owned files.

## Step 9: commit the meaningful source unit

Command shape:

`lisa commit-ticket --ticket-id T-046-07-01 --message "T-046-07-01: lead CLI surfaces with purpose" --include <four exact paths>`

Actions:

1. Invoke Lisa's transaction with one `--include` per exact repository-relative path.
2. Do not run `git add`.
3. Do not run ordinary `git commit`.
4. Check transaction output for success.
5. Check Git status afterward.

Verification criteria:

- The commit exists and is associated with the ticket-owned source unit.
- Ticket-owned files are no longer modified or untracked.
- Unrelated worktree changes remain untouched.
- The ordinary Git index is not consumed.

## Step 10: finish Implement artifact

File: `.lisa/attempts/T-046-07-01/1/work/progress.md`.

Actions:

1. Record each completed plan step.
2. Record exact verification commands and outcomes.
3. Record the Lisa transaction commit identifier if reported.
4. Record deviations, or explicitly state there were none.
5. Record that no ticket-owned source changes remain.

Verification:

- Progress artifact distinguishes implementation from review.
- It is located only in the attempt-private work directory.

## Step 11: Review

Files:

- `.lisa/attempts/T-046-07-01/1/work/review.md`
- `.lisa/attempts/T-046-07-01/1/work/review-disposition.json`

Actions:

1. Re-read ticket acceptance criteria.
2. Inspect the committed diff.
3. Confirm purpose precedes named mechanisms on all three outputs.
4. Confirm existing help grouping, ordering, visibility, and snapshots pass.
5. Summarize modified files and test coverage.
6. Identify open concerns or state that none remain.
7. Write the exact pass JSON only if work is ready.
8. Otherwise write block JSON with a non-empty actionable reason.

Pass conditions:

- Both acceptance criteria are satisfied.
- All relevant tests pass.
- Source is committed through `lisa commit-ticket`.
- Ticket-owned source paths are clean.
- Both Review artifacts exist.

## Contingencies

- If the existing help snapshot differs for unrelated reasons, inspect concurrent changes before updating it.
- If `lisa commit-ticket` rejects ownership, do not fall back to ordinary Git commands.
- If formatting touches unrelated files, preserve those user-owned changes and restrict the commit include list.
- If broad tests fail outside the modified surface, rerun focused tests and document evidence precisely.
- Do not change ticket phase or status under any contingency.
- After Review artifacts are complete, remain on this ticket and stop.
