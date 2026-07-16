# Review: purpose-first CLI strings

## Disposition recommendation

Pass.
The implementation satisfies both acceptance criteria.
All ticket-owned source changes are committed through Lisa's isolated transaction.
All focused, crate-wide, workspace-wide, and configured quick checks pass.
No blocking concern remains.

## Change summary

Lisa's three required installed surfaces now open with the same purpose:

`Runs coding agents through your ticket board, so you don't have to approve every step by hand.`

The sentence is plain, active, and outcome-oriented.
It names coding agents before explaining internal machinery.
It identifies the ticket board as the input to the work.
It explains why the operator uses Lisa without claiming that human attention is never needed.

## Files modified

### `crates/lisa-cli/src/main.rs`

- Changed the top-level Clap `about` line.
- Previous text described coding agents and project tickets but omitted the reduced-approval outcome.
- New text describes coding agents, the ticket board, and the operator benefit.
- No command definitions changed.
- No command descriptions changed.
- No `display_order` values changed.
- No `hide` attributes changed.
- No dispatch logic changed.
- The everyday-path orientation and plumbing footer remain unchanged.

### `crates/lisa-cli/src/setup_guide.rs`

- Added the purpose sentence to the generated guide header.
- It appears immediately after the project-specific H1.
- It appears before the existing setup instructions.
- It therefore appears before later scheduling and DAG vocabulary.
- Project detection remains unchanged.
- Rendering remains unchanged.
- All seven setup sections remain unchanged and in the same order.

### `crates/lisa-cli/data/hooks-guide.md`

- Added the purpose sentence immediately after the document H1.
- It now orients the reader before hook, signal, plugin, and Zellij explanations.
- The existing setup/repair paragraph follows unchanged.
- All hook contracts, lifecycle tables, notification details, and manual setup content remain unchanged.
- The raw Markdown and compiled `HOOKS_GUIDE` content remain aligned.

### `crates/lisa-cli/tests/help_surface.rs`

- Updated the exact top-level help snapshot for the new about sentence.
- Added an exact purpose sentence anchor.
- Added the acceptance criterion's four mechanism terms.
- Added a case-insensitive purpose-before-mechanism assertion helper.
- Added black-box coverage for top-level help, setup-guide output, and hooks-guide output.
- Existing operator help snapshots remain unchanged.
- Existing command-resolution coverage remains unchanged.
- Existing operator/plumbing/internal separation coverage remains unchanged.
- Existing jargon coverage remains unchanged.

## Files not modified

- No `lisa-core` source changed.
- No `lisa-plugin` source changed.
- No CLI runtime or scheduler behavior changed.
- No command inventory changed.
- No public API changed.
- No configuration format changed.
- No README or website copy changed.
- No active ticket phase or status was manually edited.
- No shared work artifact was written directly by this attempt.
- Unrelated concurrent worktree changes were not included in the source commit.

## Acceptance criterion assessment

### Criterion 1

> String tests assert the purpose sentence (naming coding agents) appears in `--help` output before any of: DAG, WASM, Zellij, scheduling; and that setup-guide/hooks-guide output opens with purpose before mechanism.

Satisfied.

Evidence:

- `PURPOSE_SENTENCE` contains the exact phrase `coding agents`.
- `MECHANISM_TERMS` contains `dag`, `wasm`, `zellij`, and `scheduling`.
- `assert_purpose_precedes_mechanism` first requires the entire purpose sentence.
- It then compares its offset with each mechanism term present.
- Matching is case-insensitive.
- `cli_and_guides_put_purpose_before_mechanism` invokes the built binary.
- The tested outputs are `lisa --help`, `lisa setup-guide`, and `lisa hooks-guide`.
- The setup and hooks outputs contain later mechanism detail, so those tests exercise actual offset comparisons.
- Top-level help currently avoids all four mechanism terms; it still positively requires the complete purpose sentence.

### Criterion 2

> Existing help-surface regression tests pass with wording-only updates — grouping, ordering, and command visibility are byte-for-byte unchanged in structure.

Satisfied.

Evidence:

- `TOP_LEVEL_HELP_SNAPSHOT` changed only at its about sentence.
- Full snapshot comparison passes.
- All five operator command snapshots pass without modification.
- All thirteen subcommands still resolve.
- Visible operator command order remains init, validate, status, doctor, loop.
- Plumbing commands remain confined to the curated footer.
- Setup-guide, hooks-guide, and version remain hidden from the primary listing.
- The jargon-free operator help check passes.

## Test coverage

### Focused semantic and snapshot coverage

Command:

`cargo test -p lisa-cli --test help_surface`

Result:

- 6 passed.
- 0 failed.
- Covers exact top-level bytes.
- Covers all operator help snapshots.
- Covers command inventory.
- Covers grouping and visibility.
- Covers jargon restrictions.
- Covers purpose-before-mechanism ordering on all three required surfaces.

### CLI crate coverage

Command:

`cargo test -p lisa-cli`

Result:

- Passed.
- 307 binary unit tests passed.
- All executed CLI integration tests passed.
- All CLI doc tests passed.
- The real-Zellij delivery boundary remained ignored under its documented environment gate.

Relevant unit coverage includes:

- Setup guide generation for Rust, Node, unknown, and initialized projects.
- Setup guide section numbering and ticket-format content.
- Hooks guide non-empty and contract-marker assertions.
- Embedded hooks-guide template assertions.

### Workspace coverage

Command:

`cargo test --workspace`

Result:

- Passed.
- Core unit suite: 207 passed.
- Plugin unit suite: 395 passed.
- CLI binary unit suite: 307 passed.
- All executed integration and doc tests passed.
- No regression appeared outside the wording surface.

### Project quick check

Command:

`just check`

Result:

- Passed.
- WASM target check passed: `cargo check -p lisa-plugin --target wasm32-wasip1`.
- Workspace tests passed again under the configured check recipe.

### Formatting and diff hygiene

Commands:

- `cargo fmt --all -- --check`
- `git diff --check -- <ticket-owned paths>`

Result:

- Both passed.
- No formatting rewrite was required.
- No whitespace error was found.

## Commit and ownership audit

Source commit:

`e43f4d54ddd57a3f91e16317ba45d907d18d56be`

Commit subject:

`T-046-07-01: lead CLI surfaces with purpose`

Commit method:

- `lisa commit-ticket`.
- Four exact repository-relative `--include` paths.
- No ordinary `git add`.
- No ordinary `git commit`.

Commit contents:

- 4 files changed.
- 36 insertions.
- 2 deletions.
- Production diffs are wording only.
- Test code supplies the semantic regression lock.

Post-commit audit:

- All four ticket-owned source paths are clean.
- `git diff --cached --name-only` is empty.
- Unrelated active tickets, stories, epics, and work artifacts remain outside this commit.

## Deviations and resolutions

The initial test helper required at least one mechanism term in every surface.
The focused test revealed that current `lisa --help` contains none of the four prohibited mechanism terms.
That absence is a desirable result, not a coverage failure.
The helper was corrected to require the purpose sentence and enforce ordering for every mechanism term that occurs.
Both guide outputs do contain later mechanism terms, so the ordering logic is concretely exercised.
This deviation affected test mechanics only and did not alter production copy or scope.

## Open concerns

No blocking concerns.

One low-risk maintenance observation is that the purpose sentence is intentionally duplicated at three existing ownership points.
This avoids refactoring Clap metadata and compile-time Markdown inclusion for a wording-only ticket.
The black-box test uses one exact anchor across all three outputs, so copy drift will fail coverage.

The real-Zellij delivery test is environment-gated and was not executed, but this ticket changes no Zellij interaction.
The embedded guide was exercised through the real CLI output test and its existing unit tests.

## Human review focus

A reviewer can focus on four questions:

1. Is the selected sentence accurate and appropriately plain?
2. Does the reduced-approval wording avoid overpromising?
3. Are the guide insertions early enough to orient a first-time reader?
4. Does the test helper express `before any of` correctly when a term is absent?

The implementation and test results support a passing disposition.
