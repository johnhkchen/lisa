# Progress: purpose-first CLI strings

## Status

Implementation is complete, verified, and committed through Lisa's isolated transaction.
Review artifacts remain.

## Completed plan steps

### 1. Top-level self-description

- Updated `crates/lisa-cli/src/main.rs`.
- Replaced only the Clap `about` value.
- New copy: `Runs coding agents through your ticket board, so you don't have to approve every step by hand.`
- The sentence begins with an active verb.
- It explicitly names coding agents.
- It identifies the ticket board as the work source.
- It states the reduced-approval outcome.
- No command definition or dispatch changed.
- `before_help`, `after_help`, ordering, and visibility attributes are unchanged.

### 2. Setup-guide preamble

- Updated `crates/lisa-cli/src/setup_guide.rs`.
- Preserved the project-specific Markdown H1.
- Added the purpose sentence as the first prose after the H1.
- Preserved the existing setup directions after the purpose sentence.
- Preserved all seven guide sections and their ordering.
- No rendering or project-detection logic changed.

### 3. Hooks-guide preamble

- Updated `crates/lisa-cli/data/hooks-guide.md`.
- Preserved the existing H1.
- Added the purpose sentence as the first prose after the H1.
- Preserved all existing hook instructions and contract content.
- Compile-time inclusion through `templates::HOOKS_GUIDE` is unchanged.

### 4. Exact help snapshot

- Updated `TOP_LEVEL_HELP_SNAPSHOT` in `crates/lisa-cli/tests/help_surface.rs`.
- Changed only the about sentence inside the snapshot.
- Operator command rows are unchanged.
- Options are unchanged.
- Plumbing footer is unchanged.
- Individual operator help snapshots are unchanged.

### 5. Purpose-order string test

- Added an exact lowercase `PURPOSE_SENTENCE` anchor.
- Added the four ticket-named mechanism terms: DAG, WASM, Zellij, scheduling.
- Added `assert_purpose_precedes_mechanism`.
- The helper requires the complete purpose sentence to be present.
- It compares purpose position with every named mechanism term present in output.
- Matching is case-insensitive.
- Added black-box coverage for:
  - `lisa --help`
  - `lisa setup-guide`
  - `lisa hooks-guide`
- The tests invoke the built executable and therefore cover dispatch and emitted output.

## Verification performed

### Formatting

Command:

`cargo fmt --all -- --check`

Result:

- Passed.
- No formatter-driven source changes were needed.

### Focused help-surface test

Command:

`cargo test -p lisa-cli --test help_surface`

First run:

- Five existing tests passed.
- The new test failed because it required at least one mechanism term in every output.
- `lisa --help` currently contains none of DAG, WASM, Zellij, or scheduling.
- This was a test-design assumption rather than a production defect.

Correction:

- Kept the positive purpose-sentence requirement.
- Changed mechanism checking to assert ordering for each term when present.
- This exactly implements `purpose before any of` without requiring jargon to exist.

Final result:

- Passed: 6 tests.
- Failed: 0.
- Includes exact top-level snapshot, grouping/visibility, command resolution, jargon, operator snapshots, and new ordering coverage.

### Full CLI crate suite

Command:

`cargo test -p lisa-cli`

Result:

- Passed all CLI library, binary unit, integration, and doc tests.
- Binary unit suite: 307 passed.
- Help-surface suite: 6 passed.
- One real-Zellij boundary test remained intentionally ignored by its environment gate.
- No failures.

### Workspace suite

Command:

`cargo test --workspace`

Result:

- Passed.
- `lisa-cli`, `lisa-core`, and `lisa-plugin` suites completed successfully.
- Core unit suite: 207 passed.
- Plugin unit suite: 395 passed.
- CLI binary unit suite: 307 passed.
- All executed integration and doc tests passed.
- The environment-gated real-Zellij test remained ignored as designed.

### Project quick check

Command:

`just check`

Result:

- Passed.
- `cargo check -p lisa-plugin --target wasm32-wasip1` passed.
- Its workspace test run passed.

### Diff validation

Commands:

- `git diff --check -- <four exact ticket-owned paths>`
- `git diff -- <four exact ticket-owned paths>`
- `git status --short`
- `git diff --cached --name-only`

Result:

- No whitespace errors.
- Production changes are wording only.
- Test changes are the snapshot line, constants, helper, and one test.
- The ordinary index had no staged paths.
- The four source paths are the only ticket-owned modified source files.
- Unrelated active-ticket and work-artifact changes remain present and untouched.

## Deviation from Plan

The plan proposed requiring at least one mechanism term in every tested output.
The built `lisa --help` output intentionally contains none of the four named terms.
Requiring jargon would contradict the product outcome and make the semantic test fail on the cleanest surface.
The final helper instead requires the purpose sentence and verifies it precedes every named term that occurs.
Both guides contain mechanism detail, so their assertions exercise concrete ordering comparisons.
No production design changed.

## Commit unit

The meaningful ticket-owned source unit consists of:

- `crates/lisa-cli/src/main.rs`
- `crates/lisa-cli/src/setup_guide.rs`
- `crates/lisa-cli/data/hooks-guide.md`
- `crates/lisa-cli/tests/help_surface.rs`

These paths were committed together because the wording and its regression lock form one acceptance unit.
The commit used `lisa commit-ticket` with exact `--include` paths.
Commit: `e43f4d54ddd57a3f91e16317ba45d907d18d56be`.
No ordinary Git staging or commit command was used.

## Remaining

1. Confirm ticket-owned paths are clean afterward.
2. Write `review.md`.
3. Write the exact `review-disposition.json` pass or block shape.
4. Remain on this ticket and stop.
